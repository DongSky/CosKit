use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use serde_json::json;

use crate::engine;
use crate::gemini_client;
use crate::image_utils;
use crate::models::{ReferenceImage, Session};
use crate::planner::WorkflowPlan;
use crate::skills;

/// Execute a workflow plan as a DAG, returning (final_image_b64, reasoning).
pub async fn execute_workflow(
    sessions: &RwLock<HashMap<String, Session>>,
    session_id: &str,
    node_id: &str,
    parent_image_b64: &str,
    _original_size: (u32, u32),
    plan: &WorkflowPlan,
    references: &[ReferenceImage],
) -> Result<(Vec<u8>, String), String> {
    let registry = skills::skill_registry();
    let total_steps = plan.nodes.len() as u32;

    // Store plan in metadata
    if let Ok(plan_json) = serde_json::to_value(plan) {
        engine::update_node(sessions, session_id, node_id, |n| {
            n.metadata.insert("workflow_plan".into(), plan_json.clone());
            n.progress_total = total_steps;
        });
    }

    // Initialize workflow status for all nodes
    let mut wf_status: HashMap<String, serde_json::Value> = HashMap::new();
    for pn in &plan.nodes {
        let skill_name = registry
            .get(&pn.skill_id)
            .map(|s| s.name.as_str())
            .unwrap_or("未知");
        wf_status.insert(
            pn.node_id.clone(),
            json!({
                "status": "pending",
                "skill_name": skill_name,
                "skill_prompt": pn.skill_prompt,
            }),
        );
    }
    update_workflow_status(sessions, session_id, node_id, &wf_status);

    // Track completed node outputs: node_id -> image bytes
    let outputs: Arc<tokio::sync::RwLock<HashMap<String, Vec<u8>>>> =
        Arc::new(tokio::sync::RwLock::new(HashMap::new()));
    let mut completed: HashSet<String> = HashSet::new();
    let mut steps_done = 0u32;

    // Topological execution loop
    loop {
        // Find ready nodes: all deps satisfied, not yet completed
        let ready: Vec<_> = plan
            .nodes
            .iter()
            .filter(|pn| {
                !completed.contains(&pn.node_id)
                    && pn.depends_on.iter().all(|d| completed.contains(d))
            })
            .cloned()
            .collect();

        if ready.is_empty() {
            if completed.len() < plan.nodes.len() {
                return Err("工作流存在循环依赖".to_string());
            }
            break;
        }

        eprintln!(
            "[CosKit] workflow: batch ready [{}]",
            ready.iter().map(|pn| format!("{}({})", pn.node_id, pn.skill_id)).collect::<Vec<_>>().join(", ")
        );

        // Mark ready nodes as running
        for pn in &ready {
            if let Some(st) = wf_status.get_mut(&pn.node_id) {
                st["status"] = json!("running");
            }
        }
        update_workflow_status(sessions, session_id, node_id, &wf_status);

        engine::update_node(sessions, session_id, node_id, |n| {
            n.progress_step = steps_done;
            n.progress_msg = format!(
                "执行中: {}",
                ready
                    .iter()
                    .filter_map(|pn| registry.get(&pn.skill_id).map(|s| s.name.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        });

        // Execute ready nodes in parallel
        let mut handles = Vec::new();
        for pn in &ready {
            let skill = match registry.get(&pn.skill_id) {
                Some(s) => s.clone(),
                None => continue,
            };

            // Resolve input image: use first dependency's output, or parent image
            let input_b64 = if let Some(dep_id) = pn.depends_on.first() {
                let outs = outputs.read().await;
                if let Some(dep_bytes) = outs.get(dep_id) {
                    image_utils::bytes_to_base64(dep_bytes)
                } else {
                    parent_image_b64.to_string()
                }
            } else {
                parent_image_b64.to_string()
            };

            // Fill prompt template
            let prompt = skill
                .prompt_template
                .replace("{{SKILL_PROMPT}}", &pn.skill_prompt);

            let refs = references.to_vec();
            let temp = skill.default_temperature;
            let pn_id = pn.node_id.clone();
            let outputs_clone = Arc::clone(&outputs);

            let handle = tokio::spawn(async move {
                let result =
                    gemini_client::call_image_generation(&input_b64, &prompt, &refs, temp).await;
                match result {
                    Ok(bytes) => {
                        outputs_clone.write().await.insert(pn_id.clone(), bytes);
                        (pn_id, Ok(()))
                    }
                    Err(e) => {
                        // On failure, propagate input image so downstream can continue
                        if let Ok(fallback) = image_utils::base64_to_bytes(&input_b64) {
                            outputs_clone.write().await.insert(pn_id.clone(), fallback);
                        }
                        (pn_id, Err(e))
                    }
                }
            });
            handles.push(handle);
        }

        // Await all parallel tasks
        for handle in handles {
            match handle.await {
                Ok((pn_id, result)) => {
                    completed.insert(pn_id.clone());
                    steps_done += 1;
                    if let Some(st) = wf_status.get_mut(&pn_id) {
                        match result {
                            Ok(()) => {
                                st["status"] = json!("done");
                                eprintln!("[CosKit] workflow: node {} done", pn_id);
                            }
                            Err(ref e) => {
                                st["status"] = json!("error");
                                st["error"] = json!(e.to_string());
                                eprintln!("[CosKit] workflow: node {} error: {e}", pn_id);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[CosKit] workflow task join error: {e}");
                }
            }
        }

        update_workflow_status(sessions, session_id, node_id, &wf_status);
        engine::update_node(sessions, session_id, node_id, |n| {
            n.progress_step = steps_done;
        });
    }

    // Find the final output: last node in plan (the "sink" node)
    // Prefer the last node that has no downstream dependents
    let all_deps: HashSet<&str> = plan
        .nodes
        .iter()
        .flat_map(|pn| pn.depends_on.iter().map(|s| s.as_str()))
        .collect();
    let sink_nodes: Vec<&str> = plan
        .nodes
        .iter()
        .filter(|pn| !all_deps.contains(pn.node_id.as_str()))
        .map(|pn| pn.node_id.as_str())
        .collect();

    let fallback_node_id = plan.nodes.last().map(|n| n.node_id.as_str());
    let final_node_id = sink_nodes.last().copied().or(fallback_node_id);
    eprintln!("[CosKit] workflow: sink nodes={:?}, selected={:?}", sink_nodes, final_node_id);

    let outs = outputs.read().await;
    let final_bytes = if let Some(fid) = final_node_id {
        outs.get(fid)
            .cloned()
            .unwrap_or_else(|| image_utils::base64_to_bytes(parent_image_b64).unwrap_or_default())
    } else {
        image_utils::base64_to_bytes(parent_image_b64).unwrap_or_default()
    };

    Ok((final_bytes, plan.reasoning.clone()))
}

fn update_workflow_status(
    sessions: &RwLock<HashMap<String, Session>>,
    session_id: &str,
    node_id: &str,
    wf_status: &HashMap<String, serde_json::Value>,
) {
    let status_val = json!(wf_status);
    engine::update_node(sessions, session_id, node_id, |n| {
        n.metadata
            .insert("workflow_status".into(), status_val.clone());
    });
    engine::save_session_from_map(sessions, session_id);
}

#[cfg(test)]
mod tests {
    use crate::planner::{PlannedNode, WorkflowPlan};
    use std::collections::HashSet;

    fn make_node(id: &str, deps: Vec<&str>) -> PlannedNode {
        PlannedNode {
            node_id: id.to_string(),
            skill_id: "bg_replace".to_string(),
            skill_prompt: "test".to_string(),
            depends_on: deps.into_iter().map(String::from).collect(),
        }
    }

    fn find_sink_nodes(plan: &WorkflowPlan) -> Vec<String> {
        let all_deps: HashSet<&str> = plan
            .nodes
            .iter()
            .flat_map(|pn| pn.depends_on.iter().map(|s| s.as_str()))
            .collect();
        plan.nodes
            .iter()
            .filter(|pn| !all_deps.contains(pn.node_id.as_str()))
            .map(|pn| pn.node_id.clone())
            .collect()
    }

    #[test]
    fn sink_node_linear_chain() {
        // step_1 -> step_2 -> step_3: only step_3 is sink
        let plan = WorkflowPlan {
            reasoning: "test".into(),
            nodes: vec![
                make_node("step_1", vec![]),
                make_node("step_2", vec!["step_1"]),
                make_node("step_3", vec!["step_2"]),
            ],
        };
        assert_eq!(find_sink_nodes(&plan), vec!["step_3"]);
    }

    #[test]
    fn sink_node_parallel_merge() {
        // step_1 and step_2 parallel, step_3 depends on both
        let plan = WorkflowPlan {
            reasoning: "test".into(),
            nodes: vec![
                make_node("step_1", vec![]),
                make_node("step_2", vec![]),
                make_node("step_3", vec!["step_1", "step_2"]),
            ],
        };
        assert_eq!(find_sink_nodes(&plan), vec!["step_3"]);
    }

    #[test]
    fn sink_node_multiple_sinks() {
        // step_1 -> step_2, step_1 -> step_3: two sinks
        let plan = WorkflowPlan {
            reasoning: "test".into(),
            nodes: vec![
                make_node("step_1", vec![]),
                make_node("step_2", vec!["step_1"]),
                make_node("step_3", vec!["step_1"]),
            ],
        };
        assert_eq!(find_sink_nodes(&plan), vec!["step_2", "step_3"]);
    }

    #[test]
    fn sink_node_single_step() {
        let plan = WorkflowPlan {
            reasoning: "test".into(),
            nodes: vec![make_node("step_1", vec![])],
        };
        assert_eq!(find_sink_nodes(&plan), vec!["step_1"]);
    }

    #[tokio::test]
    #[ignore] // requires GEMINI_API_KEY and network
    async fn full_pipeline_with_example() {
        use crate::{dotenv, gemini_client, image_utils, models, planner};

        // Load .env so API key is available
        dotenv::load_dotenv_files();
        gemini_client::GeminiClients::init().expect("failed to init Gemini clients");

        // Load example image
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let img_bytes = std::fs::read(manifest.join("../example.jpg"))
            .expect("example.jpg not found");
        let img = image_utils::load_image_from_bytes(&img_bytes).expect("load image");
        let original_size = (img.width(), img.height());
        let jpg_bytes = image_utils::image_to_jpeg_bytes(&img, 90).expect("encode jpeg");
        let image_b64 = image_utils::bytes_to_base64(&jpg_bytes);

        // Load prompt
        let prompt = std::fs::read_to_string(manifest.join("../example.txt"))
            .expect("example.txt not found");
        let prompt = prompt.trim();

        // Step 1: Plan
        eprintln!("=== Planning ===");
        let plan = planner::plan_workflow(&image_b64, prompt, &[])
            .await
            .expect("planning failed");
        eprintln!("Reasoning: {}", plan.reasoning);
        for n in &plan.nodes {
            eprintln!("  {} -> {} (deps: {:?})", n.node_id, n.skill_id, n.depends_on);
        }
        assert!(!plan.nodes.is_empty(), "plan should have at least one step");

        // Step 2: Execute workflow
        eprintln!("=== Executing ===");
        let sessions = std::sync::Arc::new(std::sync::RwLock::new(
            std::collections::HashMap::new(),
        ));

        let sid = "integration_test";
        let nid = "test_node";
        let mut session = models::Session::new(sid.into(), "root".into(), original_size);
        let mut root = models::EditNode::new("root".into(), None);
        root.status = "done".into();
        session.nodes.insert("root".into(), root);
        let mut node = models::EditNode::new(nid.into(), Some("root".into()));
        node.status = "processing".into();
        session.nodes.insert(nid.into(), node);
        sessions.write().unwrap().insert(sid.into(), session);

        let result = super::execute_workflow(
            &sessions, sid, nid, &image_b64, original_size, &plan, &[],
        )
        .await;

        match result {
            Ok((bytes, note)) => {
                eprintln!("OK: {} bytes, note: {}", bytes.len(), note);
                assert!(!bytes.is_empty(), "output image should not be empty");
            }
            Err(e) => panic!("workflow failed: {e}"),
        }
    }
}
