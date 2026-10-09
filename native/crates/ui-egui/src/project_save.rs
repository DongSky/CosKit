//! Native project saves use the existing background job/progress/wait protocol.
use crate::PhotocraftApp;
use photocraft_engine::{EngineError, jobs::Started};
use serde_json::{Value, json};
use std::sync::Arc;
pub const COMMAND: &str = "coskit.project.save";
pub fn is_project(path: &str) -> bool {
    path.rsplit('.').next().is_some_and(|ext| ext.eq_ignore_ascii_case("ckpipe") || ext.eq_ignore_ascii_case("pcraft"))
}
pub fn start(app: &mut PhotocraftApp, path: String, automated: bool) -> Result<Value, String> {
    if app.session.active_job().is_some() {
        return Err("Wait for the current document task to finish".into());
    }
    let save = app.services.project_save.clone().ok_or("no background project writer")?;
    let state = app.session.active().ok_or("no document")?;
    let document = state.project_document();
    let revision = state.revision;
    let source_pipeline = state.pipeline.clone();
    let destination = path.clone();
    let started = app
        .session
        .start_job(
            COMMAND,
            json!({"path":path,"automated":automated}),
            "Save CosKit project",
            true,
            move |ctx| save(document, &destination, ctx, automated).map_err(EngineError::Other),
            move |session, (prepared, warnings)| {
                let state = session.active_mut().ok_or(EngineError::NoDocument)?;
                let same_pipeline = match (&source_pipeline, &state.pipeline) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    _ => false,
                };
                if state.revision != revision || !same_pipeline {
                    return Err(EngineError::Other("The captured version was saved; newer changes remain unsaved".into()));
                }
                state.pipeline = prepared.pipeline;
                state.path = Some(path.clone());
                state.saved_revision = revision;
                if !automated && let Some(i) = session.active_index() {
                    photocraft_engine::automate_cmds::document_saved(session, i);
                }
                session.rebalance_memory();
                Ok(json!({"path":path,"warnings":warnings}))
            },
        )
        .map_err(|e| e.to_string())?;
    match started {
        Started::Done(v) => Ok(v),
        Started::Job(job) => {
            app.jobs.last_started = Some(job);
            app.ui.status = "正在后台保存工程…".into();
            app.ui.status_error = false;
            Ok(json!({"job":job.0,"pending":true}))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use photocraft_engine::{Session, jobs::JobId};
    fn app(fail: bool) -> PhotocraftApp {
        let services = crate::Services {
            project_save: Some(Arc::new(move |doc, path, ctx, _| {
                ctx.check().map_err(|e| e.to_string())?;
                if fail {
                    return Err("simulated disk full".into());
                }
                let prepared = photocraft_io::project::prepare_save(&doc, path).map_err(|e| e.to_string())?;
                Ok((prepared, vec![]))
            })),
            ..Default::default()
        };
        let mut app = PhotocraftApp::new(Session::new(), services);
        app.background_jobs = true;
        app.run("file.new", json!({"width":8,"height":8})).unwrap();
        app.run("layer.new.layer", json!({})).unwrap();
        app
    }
    fn start_test(app: &mut PhotocraftApp) -> JobId {
        let v = start(app, "test.ckpipe".into(), true).unwrap();
        assert_eq!(v["pending"], true);
        JobId(v["job"].as_u64().unwrap())
    }
    #[test]
    fn project_save_finishes_on_original_document_after_tab_switch() {
        let mut app = app(false);
        let rev = app.session.active().unwrap().revision;
        let id = start_test(&mut app);
        assert!(app.session.active().unwrap().is_dirty());
        app.run("file.new", json!({"width":8,"height":8})).unwrap();
        app.session.wait_job(id).unwrap();
        assert_eq!(app.session.active_index(), Some(1));
        let saved = &app.session.documents()[0];
        assert!(!saved.is_dirty());
        assert_eq!(saved.revision, rev);
        assert_eq!(saved.path.as_deref(), Some("test.ckpipe"));
        assert!(!saved.pipeline.as_ref().unwrap().previews.is_empty());
        assert!(app.session.active().unwrap().path.is_none());
    }
    #[test]
    fn project_save_failure_cancel_and_new_metadata_keep_document_dirty() {
        for case in 0..3 {
            let mut app = app(case == 0);
            let id = start_test(&mut app);
            if case == 1 {
                app.session.cancel_job(id);
            }
            if case == 2 {
                let doc = app.session.active().unwrap().doc.id.0;
                app.run("coskit.project.annotate", json!({"document":doc,"messages":[{"text":"arrived while saving"}]})).unwrap();
            }
            assert!(app.session.wait_job(id).is_err());
            app.session.join_cancelled_jobs();
            let st = app.session.active().unwrap();
            assert!(st.is_dirty());
            assert!(st.path.is_none());
            if case == 2 {
                assert_eq!(st.pipeline.as_ref().unwrap().conversations[0]["text"], "arrived while saving");
            }
        }
    }
}
