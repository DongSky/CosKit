//! Persistent, branching CKPipe checkpoints and isolated parameter replay.
use crate::{CommandSpec, EngineError, Result, Session};
use photocraft_doc::{
    LayerId,
    pipeline::{Operation, Pipeline},
};
use serde_json::{Value, json};
use std::sync::Arc;
fn error(e: impl std::fmt::Display) -> EngineError {
    EngineError::Other(e.to_string())
}
fn enabled(s: &Session) -> std::result::Result<(), String> {
    s.active().map(|_| ()).ok_or_else(|| "no document".into())
}

pub fn replayable(id: &str) -> bool {
    (id.starts_with("image.adjustments.") || id.starts_with("filter.") || id == "paint.stroke" || id.starts_with("select.") || id.starts_with("layer.set"))
        && !id.to_ascii_lowercase().contains("file")
        && !id.to_ascii_lowercase().contains("load")
        && !id.to_ascii_lowercase().contains("save")
}
pub(crate) fn record(s: &mut Session, id: &str, params: &Value, journal: bool) {
    if !journal || id.starts_with("file.") || id.starts_with("coskit.project.") {
        return;
    }
    let mut params = crate::channel_cmds::inject_target(s, id, params.clone());
    if id == "paint.stroke"
        && let Ok(brush) = crate::brush_cmds::resolve_brush(s, &params, id)
        && let Some(map) = params.as_object_mut()
    {
        map.remove("preset");
        map.insert("brush".into(), serde_json::to_value(&brush).unwrap_or(Value::Null));
        map.insert("color".into(), json!(brush.color));
        map.insert("seed".into(), json!(brush.seed));
    }
    let Some(st) = s.active_mut() else {
        return;
    };
    // Keep only editor commands; never API settings, credentials, scripts, or encoded AI images.
    if !["image.", "filter.", "paint.", "select.", "layer.", "type.", "path.", "edit."].iter().any(|prefix| id.starts_with(prefix)) {
        return;
    }
    let active_layer = st.active_layer.map(|v| v.0);
    let selected_layers = st.selected_layers().iter().map(|v| v.0).collect();
    let p = Arc::make_mut(st.pipeline.get_or_insert_with(|| Arc::new(Pipeline::default())));
    if p.operations.len() >= 100_000 || params.to_string().len() > 1 << 20 {
        p.omitted_operations = p.omitted_operations.saturating_add(1);
        return;
    }
    p.operations.push(Operation {
        command: id.into(),
        params: params.clone(),
        base: p.head.clone(),
        active_layer,
        selected_layers,
        replayable: replayable(id),
    });
}
fn inspect(s: &mut Session, _: &Value) -> Result<Value> {
    let st = s.active().ok_or(EngineError::NoDocument)?;
    Ok(json!({"project":st.pipeline,"width":st.doc.size.width,"height":st.doc.size.height,"depth":format!("{:?}",st.doc.depth)}))
}
fn ensure_pipeline(s: &Session, source: &Option<Arc<Pipeline>>) -> Result<()> {
    let current = &s.active().ok_or(EngineError::NoDocument)?.pipeline;
    let same = match (source, current) {
        (None, None) => true,
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        _ => false,
    };
    if same { Ok(()) } else { Err(error("Project metadata changed during the task; retry with the latest conversation")) }
}
fn checkpoint(s: &mut Session, params: &Value) -> Result<Value> {
    let st = s.active().ok_or(EngineError::NoDocument)?;
    let source_pipeline = st.pipeline.clone();
    let mut doc = st.project_document();
    let label = params.get("label").and_then(Value::as_str).unwrap_or("Checkpoint").to_string();
    let active = st.active_layer.map(|v| v.0);
    let selected = st.selected_layers().iter().map(|v| v.0).collect();
    crate::jobs::run(
        s,
        "Create CKPipe version",
        true,
        move |ctx| {
            ctx.progress(0.05, "Recording changed blocks");
            let id = photocraft_format::pipeline::checkpoint(&mut doc, &label, active, selected).map_err(error)?;
            ctx.check()?;
            ctx.progress(0.7, "Rendering version preview");
            photocraft_io::project::cache_current_preview(&mut doc).map_err(error)?;
            ctx.check()?;
            Ok((doc.pipeline, id))
        },
        move |s, (pipeline, id)| {
            ensure_pipeline(s, &source_pipeline)?;
            let st = s.active_mut().ok_or(EngineError::NoDocument)?;
            st.pipeline = pipeline;
            st.saved_revision = u64::MAX;
            s.rebalance_memory();
            Ok(json!({"version":id}))
        },
    )
}
fn restore(s: &mut Session, p: &Value) -> Result<Value> {
    let id = p.get("version").and_then(Value::as_str).ok_or_else(|| error("missing version"))?.to_string();
    let st = s.active().ok_or(EngineError::NoDocument)?;
    let source_pipeline = st.pipeline.clone();
    let mut source = st.project_document();
    let active = st.active_layer.map(|v| v.0);
    let selected = st.selected_layers().iter().map(|v| v.0).collect();
    crate::jobs::run(
        s,
        "Restore CKPipe version",
        true,
        move |ctx| {
            ctx.progress(0.05, "Protecting current version");
            photocraft_format::pipeline::checkpoint(&mut source, "Before restore", active, selected).map_err(error)?;
            photocraft_io::project::cache_current_preview(&mut source).map_err(error)?;
            ctx.check()?;
            ctx.progress(0.5, "Loading shared version blocks");
            let (doc, version) = photocraft_format::pipeline::restore(&source, &id).map_err(error)?;
            ctx.check()?;
            Ok((doc, version, id))
        },
        move |s, (restored, v, id)| {
            ensure_pipeline(s, &source_pipeline)?;
            s.edit("Restore CKPipe version", move |doc, active| {
                *doc = restored;
                *active = v.active_layer.map(LayerId);
                Ok(())
            })?;
            if let Some(st) = s.active_mut() {
                st.selected_layers = v.selected_layers.iter().copied().map(LayerId).filter(|id| st.doc.layer(*id).is_some()).collect();
            }
            s.rebalance_memory();
            Ok(json!({"version":id,"restored":true}))
        },
    )
}
fn safe_replay_params(p: &Value, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    match p {
        Value::Object(map) => map.iter().all(|(key, value)| {
            !matches!(key.to_ascii_lowercase().as_str(), "file" | "path" | "mappath" | "mapdocument" | "script" | "url" | "profile" | "directory")
                && safe_replay_params(value, depth + 1)
        }),
        Value::Array(array) => array.iter().all(|v| safe_replay_params(v, depth + 1)),
        _ => true,
    }
}
fn replay(s: &mut Session, p: &Value) -> Result<Value> {
    let id = p.get("version").and_then(Value::as_str).ok_or_else(|| error("missing base version"))?;
    let steps = p
        .get("steps")
        .and_then(Value::as_array)
        .filter(|v| !v.is_empty() && v.len() <= 128)
        .ok_or_else(|| error("steps must contain 1–128 command/params objects"))?;
    // Validate every command before any work. File/script/AI execution is never replayed.
    for step in steps {
        let command = step.get("command").and_then(Value::as_str).ok_or_else(|| error("missing replay command"))?;
        if !replayable(command) {
            return Err(error(format!("{command} cannot be replayed; re-run it explicitly")));
        }
        if !safe_replay_params(step, 0) {
            return Err(error("replay cannot read external files, other documents, scripts or profiles"));
        }
        if !step.get("params").is_some_and(Value::is_object) {
            return Err(error("replay params must be an object"));
        }
    }
    if p.to_string().len() > 8 << 20 {
        return Err(error("replay parameters exceed 8 MiB"));
    }
    let st = s.active().ok_or(EngineError::NoDocument)?;
    let source_pipeline = st.pipeline.clone();
    let active = st.active_layer.map(|v| v.0);
    let selected = st.selected_layers().iter().map(|v| v.0).collect();
    let authorize = s.authorize;
    let id = id.to_string();
    let steps = steps.clone();
    let mut source = st.project_document();
    crate::jobs::run(
        s,
        "Replay CKPipe parameters",
        true,
        move |ctx| {
            ctx.progress(0.02, "Protecting current version");
            photocraft_format::pipeline::checkpoint(&mut source, "Before parameter replay", active, selected).map_err(error)?;
            photocraft_io::project::cache_current_preview(&mut source).map_err(error)?;
            let (base, version) = photocraft_format::pipeline::restore(&source, &id).map_err(error)?;
            let mut isolated = Session::new();
            isolated.authorize = authorize;
            isolated.add_document(base, None);
            if let Some(layer) = version.active_layer {
                crate::layer_multi_cmds::set_selection(
                    &mut isolated,
                    version.selected_layers.iter().copied().map(LayerId).collect(),
                    Some(LayerId(layer)),
                    Some(LayerId(layer)),
                )?;
            }
            for (index, step) in steps.iter().enumerate() {
                ctx.check()?;
                ctx.progress(0.1 + 0.7 * index as f32 / steps.len() as f32, "Replaying parameters");
                if let Some(layer) = step.get("active_layer").and_then(Value::as_u64) {
                    isolated.select_layer(LayerId(layer))?;
                }
                if let Some(value) = step.get("selected_layers") {
                    let ids = value
                        .as_array()
                        .filter(|v| v.len() <= 10000)
                        .ok_or_else(|| error("selected_layers must be a bounded array"))?
                        .iter()
                        .map(|v| v.as_u64().map(LayerId).ok_or_else(|| error("invalid selected layer")))
                        .collect::<Result<Vec<_>>>()?;
                    let active = isolated.active().and_then(|s| s.active_layer);
                    crate::layer_multi_cmds::set_selection(&mut isolated, ids, active, active)?;
                }
                let command = step.get("command").and_then(Value::as_str).ok_or_else(|| error("missing command"))?;
                if command == "paint.stroke"
                    && let Some(background) = step.get("params").and_then(|p| p.get("brush")).and_then(|b| b.get("background")).and_then(Value::as_array)
                    && background.len() == 4
                {
                    for (target, value) in isolated.tools.background.iter_mut().zip(background) {
                        *target =
                            value.as_f64().filter(|v| v.is_finite() && (0.0..=1.0).contains(v)).ok_or_else(|| error("invalid recorded brush background"))?
                                as f32;
                    }
                }
                isolated.execute(command, step.get("params").cloned().unwrap_or(Value::Null))?;
            }
            let st = isolated.active().ok_or(EngineError::NoDocument)?;
            let mut result = st.project_document();
            let active = st.active_layer;
            let new_id = photocraft_format::pipeline::checkpoint(
                &mut result,
                "Parameter replay",
                active.map(|v| v.0),
                st.selected_layers().iter().map(|v| v.0).collect(),
            )
            .map_err(error)?;
            photocraft_io::project::cache_current_preview(&mut result).map_err(error)?;
            ctx.check()?;
            Ok((result, active, st.selected_layers().to_vec(), new_id, steps.len()))
        },
        move |s, (result, active, selected, new_id, count)| {
            ensure_pipeline(s, &source_pipeline)?;
            s.edit("Replay CKPipe parameters", move |doc, target| {
                *doc = result;
                *target = active;
                Ok(())
            })?;
            if let Some(st) = s.active_mut() {
                st.selected_layers = selected;
            }
            s.rebalance_memory();
            Ok(json!({"version":new_id,"steps":count}))
        },
    )
}

fn annotate(s: &mut Session, p: &Value) -> Result<Value> {
    if p.to_string().len() > 8 << 20 {
        return Err(error("project annotation exceeds 8 MiB"));
    }
    let messages = p
        .get("messages")
        .map(|v| v.as_array().filter(|v| v.len() <= 10_000).cloned().ok_or_else(|| error("messages must be an array of at most 10000 items")))
        .transpose()?;
    let id = p.get("document").and_then(Value::as_u64).ok_or_else(|| error("missing document id"))?;
    let st = s.docs.iter_mut().find(|st| st.doc.id.0 == id).ok_or(EngineError::NoDocument)?;
    let project = Arc::make_mut(st.pipeline.get_or_insert_with(Default::default));
    if let Some(run) = p.get("run") {
        if project.runs.len() >= 1000 {
            return Err(error("workflow history limit reached"));
        }
        project.runs.push(run.clone());
    }
    if let Some(messages) = messages {
        project.conversations = messages;
    }
    // Metadata does not invalidate the pixel revision an asynchronous AI job captured.
    st.saved_revision = u64::MAX;
    Ok(json!({"annotated":true}))
}
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "coskit.project.annotate",
            label: "Update CKPipe conversation",
            menu: &[],
            shortcut: None,
            params: r#"{"document":id,"messages":[]?,"run":{}?}"#,
            enabled,
            run: annotate,
            journal: false,
        },
        CommandSpec {
            id: "coskit.project.inspect",
            label: "Inspect CKPipe project",
            menu: &[],
            shortcut: None,
            params: "{}",
            enabled,
            run: inspect,
            journal: false,
        },
        CommandSpec {
            id: "coskit.project.checkpoint",
            label: "Create CKPipe version",
            menu: &[],
            shortcut: None,
            params: r#"{"label":"version name"}"#,
            enabled,
            run: checkpoint,
            journal: true,
        },
        CommandSpec {
            id: "coskit.project.restore",
            label: "Restore CKPipe version",
            menu: &[],
            shortcut: None,
            params: r#"{"version":"v1"}"#,
            enabled,
            run: restore,
            journal: true,
        },
        CommandSpec {
            id: "coskit.project.replay",
            label: "Replay CKPipe parameters",
            menu: &[],
            shortcut: None,
            params: r#"{"version":"v1","steps":[{"command":"image.adjustments.invert","params":{}}]}"#,
            enabled,
            run: replay,
            journal: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        let mut s = Session::new();
        s.execute("file.new", json!({"width":8,"height":6,"background":"white"})).unwrap();
        s
    }
    #[test]
    fn replay_restores_multiple_selected_layer_targets() {
        let mut s = session();
        s.execute("layer.new.layer", json!({"name":"second"})).unwrap();
        let ids: Vec<_> = s.active().unwrap().doc.layers.iter().map(|l| l.id).collect();
        let active = s.active().unwrap().active_layer;
        crate::layer_multi_cmds::set_selection(&mut s, ids.clone(), active, active).unwrap();
        s.execute("coskit.project.checkpoint", json!({"label":"two targets"})).unwrap();
        s.execute("layer.setLabelColor", json!({"color":"seafoam"})).unwrap();
        let op = s.active().unwrap().pipeline.as_ref().unwrap().operations.last().unwrap().clone();
        let expected = s.active().unwrap().doc.layers.clone();
        s.execute(
            "coskit.project.replay",
            json!({"version":"v1","steps":[{"command":op.command,"params":op.params,"active_layer":op.active_layer,"selected_layers":op.selected_layers}]}),
        )
        .unwrap();
        assert_eq!(s.active().unwrap().doc.layers, expected);
    }
    #[test]
    fn brush_replay_captures_preset_pressure_flow_and_seed() {
        let mut s = session();
        s.execute("tools.setBrush", json!({"size":3,"flow":0.4,"opacity":0.6})).unwrap();
        s.execute("coskit.project.checkpoint", json!({"label":"blank"})).unwrap();
        s.execute("paint.stroke", json!({"points":[[1,1,0.2],[6,4,0.9]],"color":"#ff5522"})).unwrap();
        let expected = photocraft_compose::flatten(&s.active().unwrap().doc).to_rgba8();
        let op = s.active().unwrap().pipeline.as_ref().unwrap().operations.last().unwrap().clone();
        assert_eq!(op.params["brush"]["flow"], json!(0.4f32));
        s.execute("tools.setBrush", json!({"size":50,"flow":1.0})).unwrap();
        s.execute("coskit.project.replay", json!({"version":"v1","steps":[{"command":op.command,"params":op.params}]})).unwrap();
        assert_eq!(photocraft_compose::flatten(&s.active().unwrap().doc).to_rgba8(), expected);
        assert!(
            s.execute("coskit.project.replay", json!({"version":"v1","steps":[{"command":"layer.setAdjustment","params":{"file":"outside.cube"}}]})).is_err()
        );
    }
    #[test]
    fn parameters_survive_save_restore_and_atomic_branch_replay() {
        let mut s = session();
        s.execute("coskit.project.checkpoint", json!({"label":"original"})).unwrap();
        let before = s.active().unwrap().doc.layers.clone();
        s.execute("image.adjustments.invert", json!({})).unwrap();
        let p = s.active().unwrap().pipeline.as_ref().unwrap();
        assert_eq!(p.operations.last().unwrap().command, "image.adjustments.invert");
        s.execute("coskit.project.restore", json!({"version":"v1"})).unwrap();
        assert_eq!(s.active().unwrap().doc.layers, before);
        s.execute("coskit.project.replay", json!({"version":"v1","steps":[{"command":"image.adjustments.invert","params":{}}]})).unwrap();
        assert_ne!(s.active().unwrap().doc.layers, before);
        let versions = s.active().unwrap().pipeline.as_ref().unwrap().versions.len();
        s.execute("edit.undo", json!({})).unwrap();
        assert_eq!(s.active().unwrap().doc.layers, before);
        assert_eq!(s.active().unwrap().pipeline.as_ref().unwrap().versions.len(), versions);
        s.execute("edit.redo", json!({})).unwrap();
        assert_ne!(s.active().unwrap().doc.layers, before);
        let saved = photocraft_io::export(&s.active().unwrap().project_document(), "ckpipe", &Default::default()).unwrap();
        let back = photocraft_io::import("work.ckpipe", &saved.bytes).unwrap().document;
        assert_eq!(back.layers, s.active().unwrap().doc.layers);
        assert!(back.pipeline.as_ref().unwrap().versions.len() >= versions);
    }
    #[test]
    fn invalid_replay_is_transactional_and_file_commands_are_refused() {
        let mut s = session();
        s.execute("coskit.project.checkpoint", json!({})).unwrap();
        let before = s.active().unwrap().doc.clone();
        for steps in [
            json!([{ "command":"file.saveACopy", "params":{"path":"outside.png"}}]),
            json!([{ "command":"image.adjustments.invert", "params":{}},{"command":"layer.setProps", "params":{"layer":99999,"opacity":0.4}}]),
        ] {
            assert!(s.execute("coskit.project.replay", json!({"version":"v1","steps":steps})).is_err());
            assert_eq!(s.active().unwrap().doc, before);
        }
        for id in ["coskit.project.restore", "coskit.project.replay", "coskit.project.annotate"] {
            assert!(s.execute(id, json!({})).is_err());
        }
    }
    #[test]
    fn annotations_are_document_scoped_and_do_not_invalidate_ai_revision() {
        let mut s = session();
        let a = s.active().unwrap().doc.id;
        let revision = s.active().unwrap().revision;
        s.execute("coskit.project.annotate", json!({"document":a.0,"messages":[{"role":"user","text":"natural skin"}]})).unwrap();
        assert_eq!(s.active().unwrap().revision, revision);
        assert_ne!(s.active().unwrap().saved_revision, revision);
        s.execute("file.new", json!({"width":4,"height":4})).unwrap();
        assert!(s.active().unwrap().pipeline.is_none());
        s.execute("coskit.project.annotate", json!({"document":a.0,"run":{"accepted":true}})).unwrap();
        assert!(s.active().unwrap().pipeline.is_none());
        assert_eq!(s.documents()[0].pipeline.as_ref().unwrap().runs.len(), 1);
    }
}
