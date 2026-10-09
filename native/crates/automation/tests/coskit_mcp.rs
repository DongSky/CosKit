use photocraft_automation::PhotocraftMcp;
use rmcp::model::{CallToolRequestParams, CallToolResult, ClientConfig};
use rmcp::service::RunningService;
use rmcp::{ClientHandler, RoleClient, ServiceExt};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct Client;
impl ClientHandler for Client {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::default()
    }
}
async fn client() -> RunningService<RoleClient, Client> {
    let (s, c) = tokio::io::duplex(1 << 20);
    tokio::spawn(async move {
        let server = PhotocraftMcp::headless().serve(s).await.unwrap();
        let _ = server.waiting().await;
    });
    Client.serve(c).await.unwrap()
}
async fn call(c: &RunningService<RoleClient, Client>, name: &str, args: Value) -> CallToolResult {
    c.call_tool(CallToolRequestParams::new(name.to_owned()).with_arguments(args.as_object().unwrap().clone())).await.unwrap()
}
fn value(r: CallToolResult) -> Value {
    assert_ne!(r.is_error, Some(true), "{r:?}");
    serde_json::from_str(&r.content.iter().filter_map(|c| c.as_text()).map(|t| t.text.clone()).collect::<String>()).unwrap()
}
fn stroke() -> Value {
    json!({"points":[{"x":10,"y":20,"pressure":0.1},{"x":55,"y":20,"pressure":1.0}],"size":14,"flow":0.3,"opacity":0.8,"color":"#0099ff","preset":"CosKit · Hair Single Strand","seed":31})
}

#[tokio::test(flavor = "multi_thread")]
async fn named_tools_draw_undo_redo_and_preserve_canvas() {
    let c = client().await;
    let tools = c.list_all_tools().await.unwrap();
    for name in ["brush_list", "brush_stroke", "ai_edit", "ai_configure", "ai_status", "ai_cancel", "ai_pending"] {
        assert!(tools.iter().any(|t| t.name == name));
    }
    value(call(&c, "doc_new", json!({"width":80,"height":60,"depth":16})).await);
    let before = call(&c, "doc_render_preview", json!({"max_side":80})).await;
    value(call(&c, "brush_stroke", stroke()).await);
    let after = call(&c, "doc_render_preview", json!({"max_side":80})).await;
    assert_ne!(serde_json::to_value(&before).unwrap(), serde_json::to_value(&after).unwrap());
    let doc = value(call(&c, "doc_inspect", json!({})).await);
    assert_eq!((doc["width"].as_u64(), doc["height"].as_u64(), doc["depth"].as_u64()), (Some(80), Some(60), Some(16)));
    value(call(&c, "command_run", json!({"id":"edit.undo"})).await);
    assert_eq!(serde_json::to_value(&before).unwrap(), serde_json::to_value(call(&c, "doc_render_preview", json!({"max_side":80})).await).unwrap());
    value(call(&c, "command_run", json!({"id":"edit.redo"})).await);
    assert_eq!(serde_json::to_value(&after).unwrap(), serde_json::to_value(call(&c, "doc_render_preview", json!({"max_side":80})).await).unwrap());
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_strokes_do_not_edit_and_ai_requires_bridge() {
    let c = client().await;
    value(call(&c, "doc_new", json!({"width":80,"height":60})).await);
    let before = value(call(&c, "doc_inspect", json!({})).await);
    for (key, v) in [("flow", json!(100)), ("size", json!(0)), ("color", json!("oops")), ("points", json!([])), ("points", json!([{"x":1,"y":1,"pressure":2}]))]
    {
        let mut p = stroke();
        p[key] = v;
        assert_eq!(call(&c, "brush_stroke", p).await.is_error, Some(true));
    }
    let after = value(call(&c, "doc_inspect", json!({})).await);
    assert_eq!(before, after);
    for name in ["ai_status", "ai_cancel"] {
        assert_eq!(call(&c, name, json!({})).await.is_error, Some(true));
    }
    let mut p = stroke();
    p["flwo"] = json!(0.2);
    let r = c.call_tool(CallToolRequestParams::new("brush_stroke").with_arguments(p.as_object().unwrap().clone())).await;
    assert!(!r.is_ok_and(|v| v.is_error != Some(true)));
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn ckpipe_versions_and_parameter_replay_are_available_over_real_mcp() {
    let c = client().await;
    value(call(&c, "doc_new", json!({"width":64,"height":48})).await);
    let base = value(call(&c, "command_run", json!({"id":"coskit.project.checkpoint","params":{"label":"Original"}})).await);
    assert_eq!(base["version"], "v1");
    value(call(&c, "brush_stroke", stroke()).await);
    let state = value(call(&c, "command_run", json!({"id":"coskit.project.inspect","params":{}})).await);
    let op = state["project"]["operations"].as_array().unwrap().last().unwrap();
    assert!(op["params"]["brush"].is_object());
    let mut params = op["params"].clone();
    params["flow"] = json!(0.15);
    value(call(&c, "command_run", json!({"id":"coskit.project.replay","params":{"version":"v1","steps":[{"command":"paint.stroke","params":params}]}})).await);
    let state = value(call(&c, "command_run", json!({"id":"coskit.project.inspect","params":{}})).await);
    assert_eq!(state["width"], 64);
    assert_eq!(state["height"], 48);
    assert_eq!(state["project"]["versions"].as_array().unwrap().len(), 3);
    value(call(&c, "command_run", json!({"id":"coskit.project.restore","params":{"version":"v1"}})).await);
    let denied = call(
        &c,
        "command_run",
        json!({"id":"coskit.project.replay","params":{"version":"v1","steps":[{"command":"file.saveACopy","params":{"path":"outside.png"}}]}}),
    )
    .await;
    assert_eq!(denied.is_error, Some(true));
    c.cancel().await.unwrap();
}
