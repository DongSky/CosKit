# CHANGELOG

格式约定：每个版本一节，**新增 / 改进 / 修复** 三类。最新版本在最上。

---

## v0.0.8 — Review Agent + 工作流增强

发布日期：2026-05-07

### 新增

- **Review Agent**：可独立配置 provider 的多模型审核，输出 4 维评分（美学 / 需求匹配 / 技术质量 / 一致性）+ 文字反馈 + 改进建议
  - 新模块：`src-tauri/src/reviewer.rs`
  - 新接口：`gemini_client::call_text_with_provider`（参数化文本调用，不依赖 GeminiClients 单例）
  - 与主文本/图像模型解耦，可用 GPT 评 Gemini 的输出
- **自动修正回路**：审核分数低于阈值时把 feedback/suggestions 喂给规划器重新规划重新执行
  - 新接口：`planner::plan_workflow_with_feedback`
  - `engine::run_agent_workflow_with_review` 编排 retry loop
- **工作流中间结果保存**：每步图像保存到 `{session_dir}/intermediate_{node_id}_{step_id}.jpg`，前端工作流 UI 显示缩略图
- **合并执行模式**：把所有 skill 的 prompt 合并为单条指令一次调用，节省成本与延迟
  - 新函数：`workflow::execute_workflow_combined`、`merge_plan_into_single_prompt`
- **可展开步骤详情**：每个工作流步骤是 `<details>`，展开查看完整 prompt、依赖、错误
- **可展开审核详情**：每条审核记录展开查看四个维度评分、完整反馈、改进建议列表
- **工具栏快速开关**：「合并执行」/「保存中间结果」/「审核」三个 agent 子开关，无需打开设置面板即可切换
- **审核临时关闭**：`PipelineModules.review_enabled` 由前端工具栏控制，与 `Settings.review_enabled`（启动初始值）独立，方便在出图异常时一键停用审核

### 改进

- 轮询重渲染保留 `<details>` 展开状态（用 `data-step-id` / `data-review-id` 作为稳定 key）
- 完成态节点也使用 `renderWorkflowProgress` 渲染（之前是单独一段旧代码，无法展开）
- 设置面板新增「审核 Agent」tab
- `EditNode.metadata` 新增 `review_history` 字段，记录每次重试的审核结果

### 修复

- README 中图像模型名 `gemini-3.1-pro-image-preview` → `gemini-3-pro-image-preview`

### 数据结构变化

- `Settings` 新增：`review_enabled`、`review_auto_correct`、`review_threshold`（默认 7.0）、`review_max_retries`（默认 1）、`review_provider`、`review_model`、`review_base_url`、`review_api_key`
- `PipelineModules` 新增：`save_intermediates`（默认 true）、`combined_mode`（默认 false）、`review_enabled`（默认 false）

所有新字段都使用 `#[serde(default)]`，向后兼容旧 `settings.json`。

---

## v0.0.5 — 智能规划 + EXIF 修复

### 新增

- 智能规划工作流（agent_mode）：planner 自动生成 DAG 计划，workflow 拓扑执行
- 10 个内置技能（详见 `SKILLS.md`）
- OpenAI provider 支持（gpt-5.5、gpt-image-2）
- Provider 配置记忆（切换时自动保存/恢复）

### 修复

- EXIF orientation 处理（修复某些手机拍照后旋转错误）

---

## v0.0.4 — Modular Pipeline + 参考图

### 新增

- modular pipeline 模式（scene → background → retouch → effects 线性流程）
- 参考图支持（多张图 + 描述）
- .env 环境变量 fallback

---

## v0.0.3

### 新增

- 多会话管理
- 树状编辑历史 + 分支切换
- macOS / Windows 双平台构建

---

## 文档约定（给开发者 / Codex 等工具）

每次发布新版本时，请：

1. 在本文件顶部追加新版本节
2. 同步更新 `tauri.conf.json` / `Cargo.toml` / `package.json` 的版本号
3. 如果涉及配置项变更，同步更新 `CONFIGURATION.md`
4. 如果涉及架构变更，同步更新 `ARCHITECTURE.md`
5. 如果新增 skill，同步更新 `SKILLS.md`
