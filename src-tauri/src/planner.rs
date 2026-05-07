use serde::{Deserialize, Serialize};

use crate::gemini_client;
use crate::models::ReferenceImage;
use crate::skills;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedNode {
    pub node_id: String,
    pub skill_id: String,
    pub skill_prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPlan {
    pub reasoning: String,
    pub nodes: Vec<PlannedNode>,
}

pub fn validate_plan(plan: &WorkflowPlan) -> Result<(), String> {
    if plan.nodes.is_empty() {
        return Err("规划结果为空，没有任何步骤".to_string());
    }

    let registry = skills::skill_registry();
    let node_ids: std::collections::HashSet<&str> =
        plan.nodes.iter().map(|n| n.node_id.as_str()).collect();

    for node in &plan.nodes {
        if !registry.contains_key(&node.skill_id) {
            return Err(format!("未知技能: {}", node.skill_id));
        }
        for dep in &node.depends_on {
            if !node_ids.contains(dep.as_str()) {
                return Err(format!("步骤 {} 依赖的 {} 不存在", node.node_id, dep));
            }
        }
    }

    Ok(())
}

pub async fn plan_workflow(
    image_b64: &str,
    user_prompt: &str,
    references: &[ReferenceImage],
) -> Result<WorkflowPlan, String> {
    plan_workflow_inner(image_b64, user_prompt, references, "").await
}

pub async fn plan_workflow_with_feedback(
    image_b64: &str,
    user_prompt: &str,
    references: &[ReferenceImage],
    feedback: &str,
    suggestions: &[String],
) -> Result<WorkflowPlan, String> {
    let suggestions_text = suggestions
        .iter()
        .map(|s| format!("- {s}"))
        .collect::<Vec<_>>()
        .join("\n");

    let feedback_section = format!(
        "\n\n## 上一次执行的审核反馈\n{feedback}\n\n## 改进建议\n{suggestions_text}\n\n\
         请根据以上反馈优化你的规划，避免之前的问题。"
    );

    plan_workflow_inner(image_b64, user_prompt, references, &feedback_section).await
}

async fn plan_workflow_inner(
    image_b64: &str,
    user_prompt: &str,
    references: &[ReferenceImage],
    feedback_section: &str,
) -> Result<WorkflowPlan, String> {
    let catalog = skills::skills_catalog_for_planner();

    let system_prompt = format!(
        r#"你是一个智能修图规划器。根据用户的自然语言需求，分析照片并规划修图步骤。

{catalog}

## 专业修图流程参考

完整流程（按阶段依序，各阶段内有顺序依赖，跨阶段可并行）：
1. 调色阶段：影调调整(tone_adjust) → 色彩风格化(color_style) → 细节增强(detail_enhance)
2. 人像阶段：磨皮美肤(skin_smooth) → 美白提亮(skin_whiten) → 人脸调整(face_adjust) → 身材调整(body_reshape)
3. 背景阶段：背景替换(bg_replace) → 光线调整(lighting_adjust)
4. 特效阶段：特效添加(special_fx)

简易流程（需求简单时优先考虑，2-3步即可）：
影调调整(tone_adjust) → 色彩风格化(color_style) → 磨皮美肤(skin_smooth)/美白提亮(skin_whiten) → 人脸调整(face_adjust)/身材调整(body_reshape) → 背景替换(bg_replace)/特效添加(special_fx)

## 规划规则
1. 根据用户需求选择合适的技能（skill），可以选择 1~5 个步骤
2. 每个步骤必须使用上述技能列表中的 skill_id
3. 每个步骤需要提供具体的 skill_prompt，描述该步骤要做什么（要具体，不要笼统）
4. 通过 depends_on 指定步骤间的依赖关系（填写依赖的 node_id）
5. 没有依赖关系的步骤可以并行执行
6. 如果用户需求简单，1-2 个步骤即可，不要过度规划
7. node_id 使用 "step_1", "step_2" 等格式

## 规划原则
- 简单需求（如"修一下""美化一下"）→ 走简易流程，2-3步
- 具体需求（如"换背景+加特效+调色"）→ 按完整流程中相关部分规划
- cosplay 摄影 → 优先考虑 bg_replace + special_fx + color_style 的组合
- 同阶段内的步骤有顺序依赖（如先 tone_adjust 再 detail_enhance），请设置 depends_on
- 不同阶段的步骤可以并行（如调色和人像可同时进行）

## 输出格式
严格输出以下 JSON 格式，不要输出其他内容：
```json
{{
  "reasoning": "分析用户需求的思考过程（中文）",
  "nodes": [
    {{
      "node_id": "step_1",
      "skill_id": "技能ID",
      "skill_prompt": "该步骤的具体指令",
      "depends_on": []
    }}
  ]
}}
```

## 用户需求
{user_prompt}{feedback_section}"#
    );

    let resp =
        gemini_client::call_text_generation(image_b64, &system_prompt, references, 0.2).await?;

    let text = gemini_client::extract_text(&resp);
    if text.is_empty() {
        return Err("规划器未返回内容".to_string());
    }

    let json_val = gemini_client::parse_json(&text)?;
    let plan: WorkflowPlan =
        serde_json::from_value(json_val).map_err(|e| format!("解析规划结果失败: {e}"))?;

    validate_plan(&plan)?;

    let reasoning_preview: String = plan.reasoning.chars().take(80).collect();
    eprintln!(
        "[CosKit] planner: {} steps, reasoning: {}",
        plan.nodes.len(),
        reasoning_preview
    );
    for node in &plan.nodes {
        eprintln!(
            "[CosKit]   step {}: skill={}, prompt={}",
            node.node_id, node.skill_id, node.skill_prompt
        );
    }

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plan(reasoning: &str, nodes: Vec<PlannedNode>) -> WorkflowPlan {
        WorkflowPlan {
            reasoning: reasoning.to_string(),
            nodes,
        }
    }

    fn make_node(id: &str, skill: &str, deps: Vec<&str>) -> PlannedNode {
        PlannedNode {
            node_id: id.to_string(),
            skill_id: skill.to_string(),
            skill_prompt: "test".to_string(),
            depends_on: deps.into_iter().map(String::from).collect(),
        }
    }

    // -- UTF-8 truncation (the panic fix) --

    #[test]
    fn utf8_truncation_chinese_no_panic() {
        // The original bug: byte slicing "头" (3-byte char) at byte boundary panics
        let reasoning = "用户需要替换背景为城市街头夜景，添加逆光和轮廓光效果，同时让刀发出紫色光芒。需要分三步完成：背景替换、光线调整、特效添加。";
        let truncated: String = reasoning.chars().take(80).collect();
        assert!(!truncated.is_empty());
        assert!(truncated.len() <= reasoning.len());
    }

    #[test]
    fn utf8_truncation_exact_80_chinese_chars() {
        // 80 Chinese chars = 240 bytes; old code &s[..80] would slice mid-char
        let reasoning: String = "测".repeat(80);
        assert_eq!(reasoning.len(), 240);
        let truncated: String = reasoning.chars().take(80).collect();
        assert_eq!(truncated.chars().count(), 80);
    }

    #[test]
    fn utf8_truncation_short_string_unchanged() {
        let reasoning = "简单修图";
        let truncated: String = reasoning.chars().take(80).collect();
        assert_eq!(truncated, reasoning);
    }

    // -- WorkflowPlan serde --

    #[test]
    fn serde_roundtrip_chinese() {
        let plan = make_plan(
            "用户需要替换背景并添加特效",
            vec![
                make_node("step_1", "bg_replace", vec![]),
                make_node("step_2", "special_fx", vec!["step_1"]),
            ],
        );
        let json = serde_json::to_string(&plan).unwrap();
        let de: WorkflowPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(de.reasoning, plan.reasoning);
        assert_eq!(de.nodes.len(), 2);
        assert_eq!(de.nodes[1].depends_on, vec!["step_1"]);
    }

    // -- validate_plan --

    #[test]
    fn validate_empty_nodes_rejected() {
        let plan = make_plan("empty", vec![]);
        assert!(validate_plan(&plan).is_err());
    }

    #[test]
    fn validate_unknown_skill_rejected() {
        let plan = make_plan(
            "bad skill",
            vec![make_node("step_1", "nonexistent_skill", vec![])],
        );
        let err = validate_plan(&plan).unwrap_err();
        assert!(err.contains("未知技能"));
    }

    #[test]
    fn validate_missing_dep_rejected() {
        let plan = make_plan(
            "missing dep",
            vec![make_node("step_1", "bg_replace", vec!["step_99"])],
        );
        let err = validate_plan(&plan).unwrap_err();
        assert!(err.contains("不存在"));
    }

    #[test]
    fn validate_valid_plan_accepted() {
        let plan = make_plan(
            "valid",
            vec![
                make_node("step_1", "bg_replace", vec![]),
                make_node("step_2", "lighting_adjust", vec!["step_1"]),
                make_node("step_3", "special_fx", vec!["step_2"]),
            ],
        );
        assert!(validate_plan(&plan).is_ok());
    }
}
