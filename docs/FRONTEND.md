# 前端文档

CosKit 的前端是单文件 `src/app.js` + `src/index.html` + `src/style.css`，**无构建步骤、无框架**，通过 Tauri 的 `invoke` API 调用后端。

## 关键文件

- `src/index.html` — 静态骨架（header、chat 区、输入栏、设置/历史/帮助 modal）
- `src/app.js` — 全部状态与逻辑（IIFE 闭包）
- `src/style.css` — 暗色主题样式

## 状态模型

`app.js` 维护：

```js
let currentSessionId = null;
let sessionData = null;            // { active_path, nodes, ... }
let editFromNodeId = null;         // 用户点击非末节点时记录
let referenceImages = [];          // 参考图（base64 + description）
let pollingTimers = {};            // node_id → setInterval handle
let settingsProviderConfigs = {};  // provider 切换记忆
let cachedDefaults = null;
```

## Tauri 命令封装

`api()` 返回一个 proxy，命令名映射到入参顺序：

```js
const COMMAND_ARGS = {
  create_session: ["image_base64", "filename"],
  submit_edit: ["session_id", "parent_node_id", "prompt", "modules", "reference_images"],
  get_node_status: ["session_id", "node_id"],
  get_workflow_status: ["session_id", "node_id"],
  // ... 更多
};
```

调用形如 `await api().submit_edit(sid, parent, prompt, modules, refs)`。

## 主要 UI 组件

### 工具栏（pipeline-modules）

```html
<button data-module="agent_mode" class="active">智能规划</button>
<button data-module="combined_mode" class="agent-sub-toggle">合并执行</button>
<button data-module="save_intermediates" class="agent-sub-toggle active">保存中间结果</button>
<button data-module="review_enabled" class="agent-sub-toggle">审核</button>
<button data-module="retouch" class="legacy-toggle dimmed">美化人像</button>
<button data-module="background" class="legacy-toggle dimmed">更换背景</button>
<button data-module="effects" class="legacy-toggle dimmed">添加特效</button>
```

切换逻辑：
- 点击「智能规划」会切换主开关，并 dim/un-dim 子开关与 legacy 开关
- agent-sub-toggle 仅在 agent_mode 开启时可点
- legacy-toggle 仅在 agent_mode 关闭时可点

启动时 `init()` 读取 `Settings.review_enabled` 作为「审核」按钮初始状态。

### 工作流可视化（renderWorkflowProgress）

输出形如：

```html
<details class="wf-details" data-details-key="root" open>
  <summary>智能规划 · 3 步骤</summary>
  <div class="workflow-progress">
    <div class="wf-reasoning">用户需要换背景并加特效……</div>
    <div class="wf-steps">
      <details class="wf-step wf-step-done" data-step-id="step_1">
        <summary class="wf-step-summary">
          <span class="wf-step-icon">✓</span>
          <span class="wf-step-name">背景替换</span>
          <span class="wf-step-prompt">换成夜景城市</span>
          <img class="wf-step-thumb" src="file:///.../intermediate_xxx_step_1_thumb.jpg">
        </summary>
        <div class="wf-step-detail">
          <div class="wf-detail-row"><span class="wf-detail-label">技能 ID</span><span class="wf-detail-value">bg_replace</span></div>
          <div class="wf-detail-row"><span class="wf-detail-label">完整指令</span><span class="wf-detail-value wf-detail-multiline">换成夜景城市</span></div>
        </div>
      </details>
      ...
    </div>
    <div class="wf-reviews">
      <details class="wf-review wf-review-pass" data-review-id="0">
        <summary class="wf-review-summary">
          <span class="wf-review-score">审核 #1 ✓ 8.5/10</span>
          <span class="wf-review-feedback">整体效果良好...</span>
        </summary>
        <div class="wf-review-detail">
          <div class="wf-detail-row"><span class="wf-detail-label">美学</span><span class="wf-detail-value">8.0/10</span></div>
          ...
          <div class="wf-detail-row"><span class="wf-detail-label">改进建议</span><ul>...</ul></div>
        </div>
      </details>
    </div>
  </div>
</details>
```

### 轮询与展开状态保留

处理中节点每 500ms 调一次 `get_node_status`，调用 `renderWorkflowInto(el, status)`：

```js
function renderWorkflowInto(el, status) {
  const prev = captureWfExpanded(el);   // 记录所有 details[open] 的 data-step-id / data-review-id / data-details-key
  el.innerHTML = renderWorkflowProgress(status);
  if (prev.hasState) restoreWfExpanded(el, prev);  // 按 key 恢复
}
```

这是为了防止用户刚展开某步详情时被下一次轮询重渲染收回。

### 完成节点（done）

`refreshSession` 渲染 EditNode 树时，对完成节点同样调用 `renderWorkflowProgress`，保证 step-by-step 节点和审核历史在完成态依然可展开查看。

## 设置面板

三个 tab：

1. **API 配置** — text/image provider + key + base_url + model + timeout
2. **审核 Agent** — review_enabled / auto_correct / threshold / max_retries + provider/model/key/base_url
3. **系统提示词** — modular pipeline 4 条 prompt 模板，每条独立「重置」按钮

切换 provider 时自动保存当前配置到 `settingsProviderConfigs[type_provider]`，再恢复目标 provider 的历史值；保存时 merge 到 `Settings.provider_configs`。

## 历史会话 modal

`list_sessions` 拿到全部会话（按 created_at 倒序），用户可点击切换或删除（删除会同时删本地目录）。

## 拖拽上传

监听 `#upload-zone` 与 `#file-input` 的 `change`/`drop` 事件，把图片转 dataURL → 调 `create_session`。

## 可访问性 / 键盘

- `Esc` 关闭任意 modal 或图像查看器
- 设置 modal 点背景关闭
- 历史/帮助/设置入口在 header 右上角

## 调试技巧

- 后端日志通过 `eprintln!("[CosKit] ...")` 输出到 stderr，开发模式下在终端可见
- 前端 console 在 Tauri 的 webview 中（macOS：右键 → Inspect Element / 或开发模式自动开 devtools）
- 想看完整 workflow_status：浏览器 console 中 `await api().get_workflow_status(sid, nid)`
