# 架构文档

## 总览

CosKit 是一个客户端-服务端形态的桌面应用：

- **前端**：原生 HTML/CSS/JS 单页应用，通过 Tauri 的 `invoke` API 调用后端命令
- **后端**：Rust 异步运行时（tokio），负责会话管理、AI 调用编排、图像处理、磁盘持久化
- **AI 模型**：Gemini 与 OpenAI 兼容协议（文本 + 图像生成），review agent 可独立使用任意 provider

## 目录结构

```
CosKitRust/
├── src/                     # 前端
│   ├── index.html           # 主页面骨架
│   ├── app.js               # 状态管理 + Tauri invoke 桥接
│   ├── style.css            # 样式
│   └── logo.svg
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # 入口（仅启动 lib::run）
│   │   ├── lib.rs           # 模块注册 + Tauri builder
│   │   ├── commands.rs      # Tauri 命令（IPC 边界）
│   │   ├── engine.rs        # 会话生命周期、编辑管线编排
│   │   ├── workflow.rs      # DAG 工作流执行器（含 combined 模式）
│   │   ├── planner.rs       # LLM 规划器（生成 WorkflowPlan）
│   │   ├── reviewer.rs      # 审核 Agent（评分 + 反馈）
│   │   ├── skills.rs        # 内置技能定义
│   │   ├── gemini_client.rs # Gemini + 通用 dispatcher
│   │   ├── openai_client.rs # OpenAI 兼容协议适配
│   │   ├── settings.rs      # 设置持久化
│   │   ├── models.rs        # 数据结构（Session、EditNode、Settings 等）
│   │   ├── image_utils.rs   # 图像 I/O、缩略图、EXIF 旋转
│   │   └── dotenv.rs        # .env 加载
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                    # 本目录
└── README.md
```

## 后端模块职责

### `commands.rs` — Tauri IPC 边界

所有 `#[tauri::command]` 标记函数。
**关键命令**：
- `create_session` / `get_session` / `list_sessions` / `delete_session`
- `submit_edit` — 触发编辑管线
- `get_node_status` — 前端轮询，返回状态 + workflow_plan + workflow_status + **review_history**
- `get_image` / `export_image`
- `get_settings` / `save_settings` / `get_default_settings`
- `get_workflow_status` / `list_skills`

### `engine.rs` — 会话与管线编排

**类型**：`AppState { sessions: Arc<RwLock<HashMap<String, Session>>> }`

**关键流程**：`submit_edit` 创建 EditNode → spawn `run_edit_pipeline` → 根据 `modules.agent_mode` 分发：
- **agent_mode=true** → 调用 `run_agent_workflow_with_review`：planner → workflow（step-by-step 或 combined）→ 可选 reviewer → 不达阈值则 re-plan → 重试，直至达标或耗尽次数
- **agent_mode=false** → `run_modular_pipeline`：scene → background → retouch → effects 的传统线性管线

`update_node` / `save_session_from_map` 是工具函数：原子地更新会话内的某个节点并持久化。

### `workflow.rs` — DAG 工作流

**核心**：`execute_workflow(plan)` 按拓扑序运行计划中的每个 `PlannedNode`，独立节点（无相互依赖的）通过 `tokio::spawn` 并行。

每个节点：
1. 解析输入（依赖节点的输出 or 父图片）
2. 用 `SkillDef.prompt_template` 填入 `skill_prompt`
3. 调用 `gemini_client::call_image_generation`
4. 写入 `outputs` HashMap，更新 `workflow_status` metadata
5. **若 `save_intermediates=true`，将中间图片与缩略图写入会话目录**

**合并模式**：`execute_workflow_combined(plan)` 调用 `merge_plan_into_single_prompt(plan)` 把所有节点的 prompt 拼成单条指令，仅一次图像调用。

### `planner.rs` — LLM 规划器

- `plan_workflow(image, user_prompt, refs)` — 首轮规划
- `plan_workflow_with_feedback(... , feedback, suggestions)` — 带审核反馈的二次规划
- 内部统一走 `plan_workflow_inner(... , feedback_section)`

输出 `WorkflowPlan { reasoning, nodes }`，`PlannedNode { node_id, skill_id, skill_prompt, depends_on }`。
`validate_plan` 校验 skill 存在、依赖完整。

### `reviewer.rs` — 审核 Agent

`review_image(config, original, result, prompt, plan, refs, threshold)` → `ReviewResult { overall_score, dimensions, feedback, suggestions, pass }`。

四个评分维度：`aesthetic_quality` / `requirement_match` / `technical_quality` / `consistency`。

通过 `gemini_client::call_text_with_provider` 使用独立 provider 调用，与主文本/图像模型解耦。

### `gemini_client.rs` — 模型分发层

- 单例 `GeminiClients`：装载主文本/图像 client、URL、API key、provider
- `dispatch_text` / `dispatch_image` — 内部统一入口，根据 provider 路由到 Gemini 或 OpenAI 适配器
- `call_text_with_provider` — **用独立 provider/model/key 的文本调用**（reviewer 专用）
- `call_text_generation` / `call_image_generation` — 通用 helper（planner / workflow 使用）
- 高层 helper：`detect_scene_type`、`analyze_background`、`retouch_image`、`apply_cosplay_effect`（modular pipeline 使用）

### `openai_client.rs` — OpenAI 兼容适配

- `gemini_contents_to_openai_messages` 把 Gemini-style `contents` 转换成 OpenAI chat messages
- `call_text` / `call_image` 分别调 `/chat/completions` 与 `/images/edits`（or `/images/generations`）
- `compute_output_size` 根据 gpt-image-2 约束（边长 16 倍数、长短边比 ≤3:1、像素总数区间）算出输出尺寸

### `models.rs` — 数据结构

- `EditNode { id, parent_id, children, prompt, image_path, thumbnail_path, note, status, error_msg, created_at, metadata }`
  - `metadata` 是个 `HashMap<String, serde_json::Value>`，承载 `workflow_plan`、`workflow_status`、`review_history`、`reference_images` 等扩展字段
  - 瞬态字段：`progress_step`、`progress_total`、`progress_msg`（serde skip）
- `Session { id, root_id, nodes, original_size, active_path, created_at }`
- `Settings`（详见 `CONFIGURATION.md`）
- `PipelineModules { retouch, background, effects, agent_mode, save_intermediates, combined_mode, review_enabled }`
- `ReferenceImage { data, description }`
- `ProviderConfig { model, base_url, api_key }` — 用于切换 provider 时的配置记忆

### `settings.rs` — 持久化

- `data_dir()` 返回平台标准目录
- `load_settings()` 逐字段 merge 用户的 `settings.json` 与 `default_settings()`，向后兼容缺失字段
- `default_prompts()` 返回 modular pipeline 用到的 4 个提示词模板

## 前端架构（简要）

详见 `FRONTEND.md`。要点：

- 状态全部在 `src/app.js` IIFE 闭包中
- 每个会话有一棵 EditNode 树，前端渲染时遍历 `active_path`
- 处理中节点通过 `setInterval` 每 500ms 轮询 `get_node_status`，调 `renderWorkflowInto` 重渲染（保留展开状态）
- 完成态节点同样调用 `renderWorkflowProgress` 以共享渲染逻辑

## 持久化布局

会话目录：`{data_dir}/{session_id}/`

| 文件 | 说明 |
|------|------|
| `session.json` | 整个 Session 序列化（含所有 EditNode） |
| `original.jpg` | 原图 |
| `original_thumb.jpg` | 原图缩略图 |
| `{node_id}.jpg` | 节点最终图片 |
| `{node_id}_thumb.jpg` | 节点缩略图 |
| `intermediate_{node_id}_{step_id}.jpg` | **工作流中间结果**（save_intermediates=true 时） |
| `intermediate_{node_id}_{step_id}_thumb.jpg` | 中间结果缩略图 |

## 安全 / 隐私

- API key 仅存于本地 `settings.json`，不上传任何分析端点
- 中间图片落盘有助于调试，但占用磁盘；可通过 `save_intermediates` 关闭
- review agent 是 **opt-in**，默认关闭以避免额外的 API 成本与对图片质量的副作用

## 错误处理策略

- 网络错误：自动重试，指数退避（最多 3-5 次，详见 `gemini_client::call_with_retry`）
- 永久错误（PROHIBITED_CONTENT、SAFETY、CONTENT_POLICY 等关键词）：立即终止，不重试
- 工作流单节点失败：在 `outputs` 中放入输入图片作为 fallback，让下游节点继续
- review 失败：graceful degrade，跳过审核，直接返回当前结果
- 重规划失败：保留当前结果并附错误说明
