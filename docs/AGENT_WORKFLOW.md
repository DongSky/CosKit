# 智能规划工作流

CosKit 的 **agent_mode**（默认开启）使用 LLM 规划器自动拆解用户意图为多个修图步骤，按 DAG 拓扑序执行。

## 关键文件

- `src-tauri/src/planner.rs` — 规划器
- `src-tauri/src/workflow.rs` — DAG 执行器
- `src-tauri/src/skills.rs` — 内置技能定义
- `src-tauri/src/engine.rs:run_agent_workflow_with_review` — 编排入口

## 数据结构

```rust
pub struct WorkflowPlan {
    pub reasoning: String,         // LLM 思考过程（中文）
    pub nodes: Vec<PlannedNode>,
}

pub struct PlannedNode {
    pub node_id: String,           // step_1, step_2 ...
    pub skill_id: String,          // 必须在 skill_registry 中
    pub skill_prompt: String,      // 该步骤的具体指令
    pub depends_on: Vec<String>,   // 依赖的 node_id
}
```

## 阶段一：规划

`planner::plan_workflow(image_b64, user_prompt, references)` 调用文本模型生成 `WorkflowPlan`。

### 系统提示词结构

1. 可用技能列表（`skills::skills_catalog_for_planner()` 拼接）
2. 专业修图流程参考（完整 4 阶段 + 简易流程）
3. 规划规则（1-5 步、必须使用 skill_id、依赖关系等）
4. 规划原则（简单需求走简易流程、cosplay 摄影优先 bg_replace+special_fx+color_style 等）
5. 输出 JSON schema
6. 用户需求

### 校验

`validate_plan(plan)` 检查：
- 至少 1 个步骤
- 每个 `skill_id` 存在于 `skill_registry()`
- 每个 `depends_on` 引用的 `node_id` 实际存在

## 阶段二：执行

### Step-by-step（默认）

`workflow::execute_workflow(... , save_intermediates: bool)` 拓扑执行：

```
loop:
    ready = 所有依赖已完成、自身未完成的节点
    if ready 为空 且仍有未完成节点 → 循环依赖错误
    if ready 为空 且全部完成 → break

    标记 ready 节点为 running，更新前端状态
    并行 spawn 每个 ready 节点：
        - 输入：第一个依赖的输出 image，或父图片（首个节点）
        - prompt = SkillDef.prompt_template.replace("{{SKILL_PROMPT}}", node.skill_prompt)
        - 调 gemini_client::call_image_generation
        - 成功 → 写入 outputs[node_id]
        - 失败 → 把输入图作为 fallback 放入 outputs，下游可继续
    join 所有 handle
    [save_intermediates=true]
        - 把 outputs[node_id] 缩放到 original_size
        - 保存到 {session_dir}/intermediate_{edit_node_id}_{step_id}.jpg
        - 生成缩略图
        - 把 image_path / thumbnail_path 写入 workflow_status[node_id]
```

**Sink 节点选择**：所有不被依赖的节点中，取最后一个；其输出作为最终图片。

### Combined（合并执行）

`workflow::execute_workflow_combined(plan)` 把所有节点压成单条提示词，一次图像调用：

```rust
fn merge_plan_into_single_prompt(plan: &WorkflowPlan) -> String {
    // 按拓扑序，每个节点：
    // "【步骤 N - 技能名】\n" + skill.prompt_template.replace("{{SKILL_PROMPT}}", pn.skill_prompt)
    // 用双换行分隔，前缀提示模型综合考虑所有要求
}
```

**温度**：所有节点 `default_temperature` 的均值。

**优势**：
- 仅 1 次 API 调用，**成本/延迟降低**
- 所有要求在同一上下文，**风格一致性更好**

**劣势**：
- 模型可能忽略某些细节
- 无中间结果可视化

### 选择执行模式

由 `PipelineModules.combined_mode` 决定（前端工具栏的「合并执行」开关）。

## 阶段三：审核（可选）

详见 [REVIEW_AGENT.md](REVIEW_AGENT.md)。简述：执行完成后若 `review_enabled=true`：

1. 调用 `reviewer::review_image` 评估结果
2. 评分 ≥ threshold → 接受
3. 评分 < threshold 且 `review_auto_correct=true` → 把 feedback/suggestions 喂给 `planner::plan_workflow_with_feedback` 重新规划，重新执行
4. 重试达到 `review_max_retries` → 接受当前结果
5. review 调用本身失败 → graceful degrade，直接接受

## 工作流状态可视化

前端通过轮询 `commands::get_node_status` 获取：

```json
{
  "status": "processing" | "done" | "error",
  "progress_step": 2,
  "progress_total": 4,
  "progress_msg": "执行中: 背景替换",
  "workflow_plan": { "reasoning": "...", "nodes": [...] },
  "workflow_status": {
    "step_1": {
      "status": "done",
      "skill_name": "背景替换",
      "skill_prompt": "换成夜景城市",
      "image_path": "/.../intermediate_xxx_step_1.jpg",
      "thumbnail_path": "/.../intermediate_xxx_step_1_thumb.jpg"
    },
    "step_2": { "status": "running", ... }
  },
  "review_history": [
    { "attempt": 0, "review": { "overall_score": 6.5, "feedback": "...", "suggestions": [...], "pass": false } },
    { "attempt": 1, "review": { "overall_score": 8.0, ..., "pass": true } }
  ]
}
```

前端用 `renderWorkflowProgress(status)` 渲染折叠的 `<details>`：
- 每个步骤可展开查看完整 prompt、依赖、错误
- 每条审核记录可展开查看四个维度评分、完整反馈、改进建议列表
- 完成节点显示中间结果缩略图

## 失败模式与回退

| 失败情形 | 处理 |
|---------|------|
| 规划返回非法 JSON | 返回错误，节点标记 error |
| 规划包含未知 skill_id | validate_plan 拒绝，返回错误 |
| 规划存在循环依赖 | execute_workflow 检测后报错 |
| 单节点图像调用失败 | 输入图作为 fallback，下游继续 |
| 所有节点都失败 | 输出最终为最后一个节点的输入图 |
| review API 失败 | 跳过审核，直接接受 |
| 重规划失败 | 接受当前结果并附错误说明 |
