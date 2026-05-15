# 配置参考

CosKit 配置由 Settings（持久化到 `settings.json`）和 PipelineModules（每次提交时由前端发送）两部分组成。

## Settings（持久化设置）

存储位置：`{data_dir}/settings.json`

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `text_provider` | String | `"gemini"` | 文本模型提供商：`gemini` / `openai` |
| `image_provider` | String | `"gemini"` | 图像模型提供商：`gemini` / `openai` |
| `text_base_url` | String | `""` | 文本 API base url（留空走 `.env` 或默认） |
| `text_api_key` | String | `""` | 文本 API key |
| `text_model` | String | `""` | 文本模型名 |
| `image_base_url` | String | `""` | 图像 API base url |
| `image_api_key` | String | `""` | 图像 API key |
| `image_model` | String | `""` | 图像模型名 |
| `text_timeout_ms` | u64 | `180000` | 文本调用超时（180s） |
| `image_timeout_ms` | u64 | `300000` | 图像调用超时（300s） |
| `prompts` | Map<String, String> | 见 `default_prompts()` | modular pipeline 4 个提示词模板（detect_scene_type / analyze_background / retouch_image / apply_cosplay_effect） |
| `provider_configs` | Map<String, ProviderConfig> | `{}` | 切换 provider 时的配置记忆，key 形如 `text_gemini` / `image_openai` |
| `review_enabled` | bool | `false` | 启用审核 Agent |
| `review_auto_correct` | bool | `false` | 评分低于阈值时自动重规划 |
| `review_threshold` | f64 | `7.0` | 通过分数（0-10） |
| `review_max_retries` | u32 | `1` | 最大重试次数（除首次执行外） |
| `review_provider` | String | `""` | 审核模型提供商 |
| `review_model` | String | `""` | 审核模型名 |
| `review_base_url` | String | `""` | 审核模型 base url |
| `review_api_key` | String | `""` | 审核模型 API key |

### Provider 默认值

| 提供商 | 文本默认模型 | 图像默认模型 | 默认 base_url |
|--------|--------------|--------------|---------------|
| Gemini | `gemini-3.1-pro-preview` | `gemini-3-pro-image-preview` | Google 官方 |
| OpenAI | `gpt-5.5` | `gpt-image-2` | `https://yunwu.ai/v1` |

### 加载优先级

文本/图像/审核字段查找顺序（前者优先）：

1. `settings.json` 中的字段
2. `.env` 中的对应环境变量
3. 内置默认值

例如文本 API key：`settings.text_api_key` → `GEMINI_API_KEY` 或 `OPENAI_API_KEY` → 报错（缺少 key）。

### .env 支持的环境变量

```env
GEMINI_API_KEY=...
GEMINI_BASE_URL=https://your-proxy/v1beta/models/gemini-3.1-pro-preview:generateContent
GEMINI_TEXT_MODEL=gemini-3.1-pro-preview
GEMINI_IMAGE_BASE_URL=https://your-proxy/v1beta/models/gemini-3-pro-image-preview:generateContent
GEMINI_IMAGE_API_KEY=...           # 可选；缺省时复用 GEMINI_API_KEY
GEMINI_IMAGE_MODEL=gemini-3-pro-image-preview

OPENAI_API_KEY=...
OPENAI_BASE_URL=https://yunwu.ai/v1
OPENAI_MODEL=gpt-5.5
OPENAI_IMAGE_MODEL=gpt-image-2
```

## PipelineModules（每次提交参数）

由前端在 `submit_edit` 时附带，控制本次编辑的执行模式：

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `retouch` | bool | `true` | （legacy）执行 retouch_image 步骤 |
| `background` | bool | `false` | （legacy）执行背景分析 + 替换 |
| `effects` | bool | `false` | （legacy）执行 cosplay 特效 |
| `agent_mode` | bool | `true` | **智能规划模式**（关闭则走 modular pipeline） |
| `save_intermediates` | bool | `true` | 保存工作流每步的中间图片到磁盘 |
| `combined_mode` | bool | `false` | 把所有 skill 合并为单次图像调用（成本/延迟低，但失去步骤可视化） |
| `review_enabled` | bool | `false` | 本次提交启用审核 Agent（与 Settings.review_enabled 含义不同：这是工具栏开关） |

### 工具栏开关与 Settings 字段的关系

前端工具栏共有 4 个开关：
- 「智能规划」(`agent_mode`)
- 「合并执行」(`combined_mode`，agent 子开关)
- 「保存中间结果」(`save_intermediates`，agent 子开关)
- 「审核」(`review_enabled`，agent 子开关)

「审核」开关在应用初始化时读取 `Settings.review_enabled` 作为初始状态，但用户**可以在不打开设置面板的情况下临时关闭/开启**。后端使用 `modules.review_enabled`（前端工具栏当前状态）作为最终决定，从而支持**临时关闭审核**避免对图像质量的副作用。

如要完全停用审核：在工具栏关闭「审核」开关即可，无需修改设置面板。

## 修改设置的途径

1. **应用内设置面板**（推荐）：右上角 ⚙ → API 配置 / 审核 Agent / 系统提示词三个 tab → 保存
2. **直接编辑** `{data_dir}/settings.json`（修改后需重启应用）
3. **使用 .env**（仅作为 fallback）

## Settings 字段加载实现

`settings::load_settings()` 显式逐字段 `merge`，缺失字段不会让加载失败（每个 `if let Some(...)` 都是独立的）。新加字段只需：

1. `models.rs` 中 `Settings` 加字段，配 `#[serde(default)]` 或 `#[serde(default = "...")]`
2. 更新 `Default for Settings` 实现
3. 在 `settings.rs::load_settings` 中追加对应的 `if let Some(v) = saved.get("xxx")...` 块

无需迁移老的 `settings.json`。

## 重启 / 热加载

- API 配置改动：`commands::save_settings` 调用 `GeminiClients::reset()` 后异步重新初始化，**无需重启应用**
- review 配置改动：在每次 `run_agent_workflow_with_review` 入口重新 `load_settings()`，**无需重启应用**
- prompts 改动：高层 helper（detect_scene_type 等）在调用时从单例的 `prompts` 字段读取，重新初始化后立即生效
