# CosKit MCP

CosKit exposes a standard **stdio MCP server**, with a headless engine and an authenticated bridge to the native desktop. The reference is the public interface of [Photoshop MCP](https://github.com/alisaitteke/photoshop-mcp) and its [tool catalog](https://github.com/alisaitteke/photoshop-mcp/blob/master/docs/available-tools.md), reviewed on 2026-10-09. No Adobe code or Adobe installation is required. This is a CosKit integration, not a drop-in server with Photoshop-prefixed names.

## Connect to the desktop

Start the editor with a workspace that contains the images you intend to edit:

```powershell
coskit.exe --control 50507 --control-token-file D:/Pictures/CosKitWork/control.token --automation-read-root D:/Pictures/CosKitWork --automation-write-root D:/Pictures/CosKitWork
```

Then add this to a standard MCP client's configuration, replacing the executable and workspace paths:

```json
{
  "mcpServers": {
    "coskit": {
      "command": "D:/Apps/CosKit/coskit-cli.exe",
      "args": ["mcp", "--bridge", "127.0.0.1:50507", "--control-token-file", "D:/Pictures/CosKitWork/control.token"]
    }
  }
}
```

The packaged `scripts/start-coskit-mcp.ps1 -Workspace D:/Pictures/CosKitWork` can start both processes for a client. It reuses the selected port only after token authentication. Use a different port for a second workspace. The private `.coskit-mcp` directory holds the token and app log; keep it outside version control. The application remains open when the MCP client disconnects.

Headless mode needs no window:

```powershell
coskit-cli.exe mcp --automation-read-root D:/Pictures/CosKitWork --automation-write-root D:/Pictures/CosKitWork
```

Use forward-slash **relative** file paths in tools, e.g. `doc_open {"path":"original.jpg"}`. The configured roots restrict file access. AI editing requires bridge mode and the desktop's model configuration. Credentials never appear in MCP schemas or `ai_status`.

## Tool mapping

| Photoshop MCP workflow | CosKit interface |
|---|---|
| Open/create/list/inspect/select/save/export documents | `doc_open`, `doc_new`, `session_list`, `doc_inspect`, `doc_select`, `doc_save`, `doc_export` |
| Inspect the result | `doc_render_preview`; in bridge mode this is a window screenshot, also available as `ui_screenshot`. Export PNG for an image-only preview. |
| Layers, masks, blend modes, groups, smart objects, text | `command_list` discovers the full engine registry; `command_run` executes its documented IDs. Examples: `layer.new.layer`, `layer.setProps`, `layer.layerMask.revealSelection`, `type.create`. |
| Adjustments, filters, selections, transforms, repair | All native engine commands; examples: `filter.cameraRaw`, `paint.healingBrush`, `paint.cloneStamp`, `paint.dodge`, `paint.burn`, `select.lasso`. |
| Precise brush trajectories | `brush_list`, **`brush_stroke`** |
| Multi-step workflows | `command_batch`, plus `jobs_list` / `jobs_cancel` for long native commands. A batch is sequential, **not an atomic transaction**; each command retains its own undo entry. |
| Background replacement, generative editing and effects | **`ai_configure`, `ai_edit`, `ai_status`, `ai_cancel`, `ai_pending`** |
| Live application inspection/control | `ui_inspect`, `ui_set`, `ui_menu_invoke`, `ui_pointer`, `control_call` |

All original MCP tools remain available. Photoshop-only services such as Adobe Generative Fill/account management are not implemented or advertised; CosKit uses the user's configured image model. CosKit does not execute arbitrary ExtendScript or install system fonts through MCP.

## Exact brush control

Coordinates and diameter are **document pixels**, unaffected by zoom, display scaling or window dimensions. A path is an ordered list; its first/last samples define the endpoints. A single sample stamps a dab. Pressure, flow, opacity and hardness are fractions in `0..1`. Pressure needs a pressure-enabled preset; all CosKit hair presets have it. Set a seed to replay identical jitter. Tilts are degrees in `-90..90`, times are monotonic milliseconds.

```json
{
  "name": "brush_stroke",
  "arguments": {
    "preset": "CosKit · Hair Single Strand",
    "points": [
      {"x": 3200, "y": 700, "pressure": 0.05, "time_ms": 0},
      {"x": 3190, "y": 750, "pressure": 0.6, "time_ms": 60},
      {"x": 3205, "y": 815, "pressure": 0.05, "time_ms": 120}
    ],
    "size": 4,
    "color": "#c5e8ec",
    "opacity": 0.6,
    "flow": 0.3,
    "seed": 42
  }
}
```

Create a separate raster layer before drawing. An optional `layer` targets an explicit layer ID; `mask:true` targets its mask and `erase:true` erases. Unknown top-level fields and invalid ranges fail before painting. Maximums are 10,000 samples, diameter 5,000 px and path length 300,000 px per call. The lower-level retouch commands use their original percentage units (`0..100`); inspect their parameter documentation rather than applying the brush tool's fraction convention to them.

## Autonomous retouch harness (default since beta.4)

`ai_edit` now runs an autonomous observe/act/review loop by default. Supply the goal, not a workflow. The model discovers and calls real native MCP tools, can choose the image model, reviews each edit against the original and previous image, and rolls back problematic attempts. Reviewed native layers commit as one undoable operation. `ai_status.harness` exposes its trace; `unchanged` means the final review found no edit needed.

`ai_configure` accepts `harness_enabled`, `harness_max_steps` (1–24, default 16), `harness_max_images` (0–6, default 3), and `harness_max_rollbacks` (0–6, default 3). Mandatory review uses the configured text/vision model even without separate legacy reviewer settings. See [architecture and limits](autonomous-retouch-harness.md).

Set `harness_enabled:false` to use the traditional pipeline described below.

## Traditional AI workflow and result checks

1. Inspect the document dimensions, layer IDs and revision; make a selection if only a region should change.
2. Configure `retouch`, `background`, `effects`, `agent_mode`, `combined_mode`, `review_enabled` and `save_intermediates` as needed. Omitted flags are unchanged. Configuration while busy is rejected.
3. Call `ai_edit {"prompt":"..."}`. The current composite/selection goes to the configured provider. The original document is not resized; model processing uses an sRGB proxy and can have lower spatial resolution.
4. Poll `ai_status`. `busy:false` alone does not mean success. Inspect `outcome`: `idle`, `running`, `applied`, `failed`, `cancelled`, `pending`, `opened`, or `discarded`; `error` and `result` provide the cause or resulting layer identity.
5. If the source document changed while the request was running, `pending:true` protects the user's edits. `ai_pending {"action":"open"}` opens the result separately; `apply` still requires the original document/revision; `discard` clears it.
6. Inspect/export the pixels, verify the dimensions and protected regions, then save `.pcraft`. A model can return a poor result even after a technically successful call.

`ai_cancel` prevents an already queued result from being applied. An HTTP request already sent to the provider may still finish/bill there. The planner and image model are separate: if the planner is unavailable, `agent_mode:false,retouch:true,background:false,effects:false` runs a direct image edit with the user's explicit instruction.

## Brushes

The Brushes panel and MCP include 9 original CosKit cosplay presets and 19 adapted CC0 painting tips by David Revoy. Hair strands/rakes, soft dodge and burn, rim light, glow, fabric, dust and stars cover common portrait finishing tasks. See [brush provenance](../native/assets/brushes/deevad-2023/README.md). Animated source tips use their first frame; their names state this. Krita-specific mixing dynamics are not emulated.

## Reproduce the private photo tests

`scripts/coskit_mcp_client.py` is a small stdio MCP client, not an alternate image editor. `cosplay-regression.py`, `cosplay-brush-tests.py` and `cosplay-live.py` drive the actual server. Photo files, model responses, tokens and test outputs are excluded from release packages and version control. Results and known limitations are documented in `cosplay-validation.md`.


完整工程保存为 `.ckpipe`：保存图层、参数化记录、对话与持久分支版本。参见 [CKPipe 格式与操作](ckpipe-project-format.md)。

## beta.5 工作台控制

`ui_set {"fields":{"studio":{"inspector":"ai","pinned":"layers","versions":true}}}` 可组合侧栏；`pinned:null` 取消固定。支持 ai、adjust、layers、color、history、navigator、character、parameters。`studio.enabled:false` 回到完整面板布局。`ui_inspect` 返回 `studio`、`projectView` 和 `canvasRect`；`projectView` 包含选择的版本、预览加载状态与缓存数，便于验证后台加载和只读浏览。非法枚举及未知字段会拒绝整个调用。


## beta.8 参数与原尺寸对比

通过 `control_call` 调用只读视图方法 `ui.project`：

```json
{"method":"ui.project","params":{"version":"v1","compare":true,"zoom":1,"center":[3300,850],"difference":false}}
```

`zoom:0` 适合窗口，`1` 为 100%，`2` 为放大镜，范围 0.01–16；center 为原图像素。`difference:true` 显示 ×4 差异。`parameters:true` 打开配方窗口，`operation` 为 `coskit.project.inspect` 中 operations 的零起始索引。无效字段、版本或坐标拒绝整次调用。查看不改像素；实际重放继续用 `coskit.project.replay`，`command_run wait:false` 返回任务 ID。

`ui_inspect.projectView` 增加 `parametersOpen` 和 `detailCompare`（缩放、中心、差异模式、加载状态、纹理字节、错误）。内部自主 Harness 的 batch/inspect 决策由模型自主选择，外部 MCP 原有命令批量调用协议保持不变；外部 `command_batch` 不承诺内部 Harness 的整体事务语义。
