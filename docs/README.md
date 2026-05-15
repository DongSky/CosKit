# CosKit 文档索引

本目录提供给 AI 工具（Codex、Claude、Cursor 等）和开发者阅读，用于快速理解项目结构、核心模块和最近的功能演进。

## 文档列表

| 文档 | 用途 |
|------|------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | 项目整体架构、模块职责、数据流 |
| [AGENT_WORKFLOW.md](AGENT_WORKFLOW.md) | 智能规划工作流（planner + DAG 执行 + 合并执行模式） |
| [REVIEW_AGENT.md](REVIEW_AGENT.md) | 多模型审核 Agent + 自动修正回路 |
| [SKILLS.md](SKILLS.md) | 内置修图技能目录与提示词模板 |
| [CONFIGURATION.md](CONFIGURATION.md) | 配置项、Settings 字段、PipelineModules 字段 |
| [FRONTEND.md](FRONTEND.md) | 前端 UI 状态、轮询、可展开步骤渲染 |
| [CHANGELOG.md](CHANGELOG.md) | 版本演进与新功能时间线 |

## 项目速览

- **类型**：Tauri v2 桌面应用（Rust 后端 + 原生 HTML/JS 前端）
- **领域**：AI 人像修图（cosplay 特效、智能规划、多模型审核）
- **入口命令**：`npx tauri dev` / `npx tauri build`
- **后端核心目录**：`src-tauri/src/`
- **前端目录**：`src/`
- **数据存储**：会话目录在系统标准位置（macOS：`~/Library/Application Support/CosKit/`）

## 核心数据流（一览）

```
用户输入 prompt + 图片
        ↓
commands::submit_edit (Tauri IPC)
        ↓
engine::submit_edit → tokio::spawn(run_edit_pipeline)
        ↓
        ├─ agent_mode=true  → planner::plan_workflow → workflow::execute_workflow
        │                                            ↓
        │                                    [可选] reviewer::review_image
        │                                            ↓
        │                                    [评分不足] planner::plan_workflow_with_feedback → 重新执行
        │                                            ↓
        │                                    最终图片 + 审核历史
        │
        └─ agent_mode=false → engine::run_modular_pipeline
                                  ├─ detect_scene_type
                                  ├─ analyze_background
                                  ├─ retouch_image
                                  └─ apply_cosplay_effect
        ↓
保存图片 + 缩略图到 session 目录
        ↓
更新 EditNode.metadata（workflow_plan / workflow_status / review_history）
        ↓
前端轮询 commands::get_node_status → 渲染
```

## 给 AI 工具的提示

- 每次添加新功能或修复 Bug 后，请同步更新对应文档
- 配置项变更必须更新 `CONFIGURATION.md`
- 模块职责调整必须更新 `ARCHITECTURE.md`
- 版本发布必须在 `CHANGELOG.md` 追加条目
