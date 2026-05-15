# 审核 Agent

CosKit 在 v0.0.8 引入了独立的审核 Agent：在工作流执行后用一个可独立配置 provider 的模型评估结果，可选地基于反馈自动重规划重试。

## 关键文件

- `src-tauri/src/reviewer.rs` — 审核模块
- `src-tauri/src/gemini_client.rs:call_text_with_provider` — 参数化文本调用
- `src-tauri/src/planner.rs:plan_workflow_with_feedback` — 带反馈的重规划
- `src-tauri/src/engine.rs:run_agent_workflow_with_review` — 编排（重试回路）

## 数据结构

```rust
pub struct ReviewResult {
    pub overall_score: f64,            // 0-10
    pub dimensions: ReviewDimensions,
    pub feedback: String,              // 文字评价
    pub suggestions: Vec<String>,      // 改进建议（用于重规划）
    pub pass: bool,                    // overall_score >= threshold
}

pub struct ReviewDimensions {
    pub aesthetic_quality: f64,        // 美学质量
    pub requirement_match: f64,        // 需求匹配
    pub technical_quality: f64,        // 技术质量
    pub consistency: f64,              // 一致性
}

pub struct ReviewConfig {
    pub provider: String,              // "gemini" | "openai" | ""
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}
```

## 设计原则

1. **provider 独立**：审核可以用与文本/图像模型完全不同的 provider（例如 GPT-5 review，Gemini 出图）。`call_text_with_provider` 不依赖单例 `GeminiClients`，每次创建一次性的 reqwest client，使用调用方传入的 base_url/api_key/model
2. **opt-in**：默认关闭。需要在设置面板配置审核模型 + 在工具栏勾选「审核」按钮才生效
3. **graceful degrade**：审核或重规划失败不阻断主流程，最坏情况退回为「无审核」模式
4. **可视化历史**：每次重试的审核结果都保留，前端可展开查看

## 审核提示词

由 `reviewer::build_review_prompt` 生成，结构：

```
你是一位专业的摄影后期审核专家。请评估以下修图结果的质量。

## 输入信息
- 原始照片（第一张图片）
- 修图结果（第二张图片）
- 用户需求：{user_prompt}
- 执行的修图步骤：
  - step_1: 背景替换 — 换成夜景城市
  - step_2: 特效添加 — 添加紫色光效
{ref_hint?}

## 评估维度
1. aesthetic_quality（美学质量）：构图、色彩和谐、光影、整体视觉效果
2. requirement_match（需求匹配）：修图结果是否准确满足用户的具体需求
3. technical_quality（技术质量）：是否存在伪影、色彩断层、边缘不自然、变形等技术问题
4. consistency（一致性）：人物特征是否保持一致，风格是否统一，与参考图的匹配度

## 输出格式
严格输出以下 JSON：
{ "overall_score": 8.5, "dimensions": {...}, "feedback": "...", "suggestions": [...] }
```

`build_review_contents` 在多模态消息中放入：原始图、结果图、可选参考图。

## 重试回路（核心算法）

`engine::run_agent_workflow_with_review`：

```rust
let max_attempts = if review_enabled && auto_correct {
    1 + review_max_retries          // 1 次初始 + N 次重试
} else {
    1                                // 只跑一次
};

let mut current_plan = initial_plan;
for attempt in 0..max_attempts {
    let (result_bytes, note) = execute(&current_plan)?;

    if !review_enabled {
        return Ok((result_bytes, note));
    }

    match review_image(&review_config, original, &result_b64, prompt, &current_plan, refs, threshold).await {
        Ok(review) => {
            review_history.push({attempt, review});
            update_metadata("review_history", review_history);

            let is_last = attempt == max_attempts - 1;
            if review.pass || !auto_correct || is_last {
                // 通过 / 用户没开自动修正 / 已是最后一次 → 接受
                return Ok((result_bytes, note + format!("\n审核评分: {:.1}/10", review.overall_score)));
            }

            // 重新规划
            current_plan = plan_workflow_with_feedback(
                image_b64, prompt, refs,
                &review.feedback, &review.suggestions
            ).await?;
            // 下一次循环会用新 plan 重新执行
        }
        Err(e) => {
            // 审核本身失败 → 跳过，接受当前结果
            return Ok((result_bytes, note + format!("\n审核跳过: {}", e)));
        }
    }
}
```

## 反馈如何注入到 planner

`planner::plan_workflow_with_feedback` 在原 system prompt 末尾追加：

```
## 上一次执行的审核反馈
{feedback}

## 改进建议
- {suggestion_1}
- {suggestion_2}
...

请根据以上反馈优化你的规划，避免之前的问题。
```

LLM 看到反馈后可能：
- 调整 skill 的具体指令
- 增加补充步骤（如 lighting_adjust 修复光照不一致）
- 减少冗余步骤（如评审指出过度处理）
- 改变并行结构

## 配置

详见 [CONFIGURATION.md](CONFIGURATION.md)。简表：

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `review_enabled` (Settings) | bool | false | 在设置中启用审核（与 PipelineModules.review_enabled 任一关闭即跳过审核） |
| `review_auto_correct` | bool | false | 开启自动修正回路 |
| `review_threshold` | f64 | 7.0 | 通过分数阈值 |
| `review_max_retries` | u32 | 1 | 最大重试次数（除首次执行外） |
| `review_provider` | String | "" | "gemini" / "openai" / 空（未配置） |
| `review_model` | String | "" | 例如 `gpt-4o`、`gemini-2.5-pro` |
| `review_base_url` | String | "" | 留空走默认地址 |
| `review_api_key` | String | "" | 审核模型的 API key |

**生效条件**：`PipelineModules.review_enabled=true`（工具栏开关）→ 进入 review。具体逻辑在 `engine.rs`。

## 注意事项

- **图像大小**：审核会同时发送原图与结果图（base64），可能命中模型上下文/费率限制。reviewer 使用与主流程相同的 base64（已在 engine 中预先 resize 至 max 2048px）
- **副作用**：用户报告某些情况下启用审核会让出图行为发生异常变化（如模型解读混乱）。**前端工具栏新增了一键关闭的「审核」开关**（`PipelineModules.review_enabled`），用户可在不修改设置的前提下临时停用
- **配置完整性**：`reviewer::ReviewConfig::is_configured()` 要求 `provider`、`model`、`api_key` 全部非空，缺一即返回错误并 graceful degrade
- **历史保留**：所有重试的 ReviewResult 都保留在 `EditNode.metadata["review_history"]` 中（数组），前端展示每次评分

## 单元测试

`src-tauri/src/reviewer.rs` 中的测试：

- `review_result_serde_roundtrip` — 序列化/反序列化
- `review_prompt_contains_steps` — 提示词包含 plan 中的步骤
- `review_config_unconfigured` / `review_config_configured` — 配置有效性判断
