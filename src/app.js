/* ================================================================
   CosKit — Frontend logic (Tauri v2)
   ================================================================ */

(function () {
  "use strict";

  // ── State ──────────────────────────────────────────────
  let currentSessionId = null;
  let sessionData = null; // full session from get_session()
  let editFromNodeId = null; // non-null when user clicks a historical node
  let pollingTimers = {}; // node_id -> intervalId
  let cachedDefaults = null; // cached default settings for prompt reset
  let viewerSessionId = null; // tracked for export
  let viewerNodeId = null; // tracked for export
  let referenceImages = []; // Array of { dataUrl: string, description: string }

  // ── DOM refs ───────────────────────────────────────────
  const welcome = document.getElementById("welcome");
  const messagesEl = document.getElementById("messages");
  const inputBar = document.getElementById("input-bar");
  const promptInput = document.getElementById("prompt-input");
  const btnSend = document.getElementById("btn-send");
  const btnNewSession = document.getElementById("btn-new-session");
  const btnSettings = document.getElementById("btn-settings");
  const fileInput = document.getElementById("file-input");
  const fileInputInline = document.getElementById("file-input-inline");
  const imageViewer = document.getElementById("image-viewer");
  const viewerImg = document.getElementById("viewer-img");
  const viewerClose = document.getElementById("viewer-close");
  const viewerExport = document.getElementById("viewer-export");
  const chatArea = document.getElementById("chat-area");

  // History & Help DOM refs
  const btnHistory = document.getElementById("btn-history");
  const btnHelp = document.getElementById("btn-help");
  const historyModal = document.getElementById("history-modal");
  const historyClose = document.getElementById("history-close");
  const historyList = document.getElementById("history-list");
  const historyEmpty = document.getElementById("history-empty");
  const helpModal = document.getElementById("help-modal");
  const helpClose = document.getElementById("help-close");

  // Settings DOM refs
  const settingsModal = document.getElementById("settings-modal");
  const settingsClose = document.getElementById("settings-close");
  const settingsCancel = document.getElementById("settings-cancel");
  const settingsSave = document.getElementById("settings-save");

  // ── Tauri invoke bridge ─────────────────────────────────
  const { invoke } = window.__TAURI__.core;

  const _SIGNATURES = {
    create_session: ["image_base64", "filename"],
    get_session: ["session_id"],
    list_sessions: [],
    delete_session: ["session_id"],
    submit_edit: ["session_id", "parent_node_id", "prompt", "modules", "reference_images"],
    get_node_status: ["session_id", "node_id"],
    navigate_branch: ["session_id", "parent_node_id", "direction"],
    goto_node: ["session_id", "node_id"],
    get_image: ["session_id", "node_id", "thumbnail"],
    export_image: ["session_id", "node_id"],
    get_settings: [],
    save_settings: ["settings_val"],
    get_default_settings: [],
  };

  function _buildArgs(method, args) {
    const names = _SIGNATURES[method] || [];
    const obj = {};
    names.forEach((name, i) => {
      if (i < args.length) obj[name] = args[i];
    });
    return obj;
  }

  function api() {
    return new Proxy({}, {
      get(_, method) {
        return (...args) => {
          const argMap = _buildArgs(method, args);
          return invoke(method, argMap);
        };
      }
    });
  }

  async function waitForApi() {
    // Tauri invoke is available immediately
    return true;
  }

  // ── File reading ───────────────────────────────────────
  function readFileAsBase64(file) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result);
      reader.onerror = reject;
      reader.readAsDataURL(file);
    });
  }

  // ── Upload handler ─────────────────────────────────────
  let uploading = false;

  async function handleUpload(file) {
    if (!file || !file.type.startsWith("image/")) return;
    if (uploading) return;
    uploading = true;

    // Show immediate feedback
    welcome.style.pointerEvents = "none";
    welcome.style.opacity = "0.5";

    try {
      const base64 = await readFileAsBase64(file);
      const result = await api().create_session(base64, file.name);
      if (result.error) {
        alert("创建会话失败: " + result.error);
        return;
      }

      currentSessionId = result.session_id;
      editFromNodeId = null;
      await refreshSession();
      showChatMode();
    } finally {
      uploading = false;
      welcome.style.pointerEvents = "";
      welcome.style.opacity = "";
    }
  }

  // ── Session refresh ────────────────────────────────────
  async function refreshSession() {
    if (!currentSessionId) return;
    sessionData = await api().get_session(currentSessionId);
    renderMessages();
  }

  // ── Render messages along active_path ──────────────────
  function renderMessages() {
    if (!sessionData) return;

    const { nodes, active_path, root_id } = sessionData;
    messagesEl.innerHTML = "";

    for (let i = 0; i < active_path.length; i++) {
      const nodeId = active_path[i];
      const node = nodes[nodeId];
      if (!node) continue;

      if (nodeId === root_id) {
        // Root: show original image card
        renderBotCard(node, null, i === active_path.length - 1);
      } else {
        // User prompt bubble
        renderUserBubble(node.prompt);
        // Bot response card
        const parentId = node.parent_id;
        const parent = nodes[parentId];
        renderBotCard(node, parent, i === active_path.length - 1);
      }
    }

    // Update edit-from state
    updateEditFromUI();

    // Scroll to bottom
    chatArea.scrollTop = chatArea.scrollHeight;
  }

  function renderUserBubble(text) {
    const div = document.createElement("div");
    div.className = "msg msg-user";
    div.innerHTML = `<div class="bubble">${escapeHtml(text)}</div>`;
    messagesEl.appendChild(div);
  }

  function renderBotCard(node, parent, isLast) {
    const div = document.createElement("div");
    div.className = "msg msg-bot";
    div.dataset.nodeId = node.id;

    const card = document.createElement("div");
    card.className = "card";

    // Header with branch nav
    const header = document.createElement("div");
    header.className = "card-header";

    const label = document.createElement("span");
    label.textContent = node.parent_id === null ? "原图" : "CosKit";
    header.appendChild(label);

    // Branch navigation if parent has multiple children
    if (parent && parent.children && parent.children.length > 1) {
      const childIdx = parent.children.indexOf(node.id);
      const total = parent.children.length;

      const nav = document.createElement("div");
      nav.className = "branch-nav";

      const btnPrev = document.createElement("button");
      btnPrev.textContent = "◀";
      btnPrev.onclick = (e) => {
        e.stopPropagation();
        navigateBranch(parent.id, -1);
      };

      const info = document.createElement("span");
      info.textContent = `${childIdx + 1}/${total}`;

      const btnNext = document.createElement("button");
      btnNext.textContent = "▶";
      btnNext.onclick = (e) => {
        e.stopPropagation();
        navigateBranch(parent.id, 1);
      };

      nav.appendChild(btnPrev);
      nav.appendChild(info);
      nav.appendChild(btnNext);
      header.appendChild(nav);
    }

    card.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "card-body";

    if (node.status === "done") {
      // Thumbnail
      const thumbContainer = document.createElement("div");
      thumbContainer.className = "thumb-container";
      const img = document.createElement("img");
      img.dataset.sessionId = currentSessionId;
      img.dataset.nodeId = node.id;
      img.alt = "结果图";
      thumbContainer.appendChild(img);
      thumbContainer.onclick = (e) => {
        e.stopPropagation();
        showImageViewer(currentSessionId, node.id);
      };
      body.appendChild(thumbContainer);

      // Load thumbnail async
      loadThumbnail(img, currentSessionId, node.id);

      // Note
      if (node.note) {
        const noteEl = document.createElement("div");
        noteEl.className = "card-note";
        noteEl.textContent = node.note;
        body.appendChild(noteEl);
      }

      // Make non-last nodes clickable for "continue from here"
      if (!isLast) {
        div.classList.add("clickable");
        card.onclick = () => setEditFrom(node.id);
      }
    } else if (node.status === "processing" || node.status === "pending") {
      const proc = document.createElement("div");
      proc.className = "card-processing";
      proc.id = `processing-${node.id}`;
      proc.innerHTML = `
        <div class="spinner"></div>
        <span>${node.progress_msg || "准备中..."}</span>
      `;
      body.appendChild(proc);

      // Start polling
      startPolling(node.id);
    } else if (node.status === "error") {
      const errEl = document.createElement("div");
      errEl.className = "card-error";
      errEl.textContent = "出错: " + (node.error_msg || "未知错误");
      body.appendChild(errEl);
    }

    card.appendChild(body);
    div.appendChild(card);
    messagesEl.appendChild(div);
  }

  async function loadThumbnail(imgEl, sessionId, nodeId) {
    const dataUrl = await api().get_image(sessionId, nodeId, true);
    if (dataUrl) {
      imgEl.src = dataUrl;
    }
  }

  // ── Branch navigation ─────────────────────────────────
  async function navigateBranch(parentNodeId, direction) {
    const result = await api().navigate_branch(currentSessionId, parentNodeId, direction);
    if (result.error) return;
    sessionData.active_path = result.active_path;
    renderMessages();
  }

  // ── Edit from historical node ──────────────────────────
  function setEditFrom(nodeId) {
    editFromNodeId = nodeId;
    updateEditFromUI();
    promptInput.focus();
  }

  function clearEditFrom() {
    editFromNodeId = null;
    updateEditFromUI();
  }

  function updateEditFromUI() {
    // Remove existing indicator
    const existing = document.querySelector(".edit-from-indicator");
    if (existing) existing.remove();

    if (editFromNodeId && sessionData) {
      const node = sessionData.nodes[editFromNodeId];
      const indicator = document.createElement("div");
      indicator.className = "edit-from-indicator";
      const notePreview = node
        ? (node.note || "").substring(0, 30)
        : editFromNodeId;
      indicator.innerHTML = `
        <span>基于: ${escapeHtml(notePreview)}...</span>
        <button onclick="window._coskit_clearEditFrom()">✕</button>
      `;
      inputBar.insertBefore(indicator, promptInput);
    }
  }

  // Expose for inline onclick
  window._coskit_clearEditFrom = clearEditFrom;

  // ── Reference images ─────────────────────────────────
  const refImagesArea = document.getElementById("ref-images-area");
  const refImagesList = document.getElementById("ref-images-list");
  const refFileInput = document.getElementById("ref-file-input");

  function renderReferenceImages() {
    refImagesList.innerHTML = "";
    if (referenceImages.length === 0) {
      refImagesArea.style.display = "none";
      return;
    }
    refImagesArea.style.display = "flex";

    referenceImages.forEach((ref, idx) => {
      const card = document.createElement("div");
      card.className = "ref-card";

      const thumbWrap = document.createElement("div");
      thumbWrap.className = "ref-card-thumb";
      const img = document.createElement("img");
      img.src = ref.dataUrl;
      img.alt = "参考图";
      thumbWrap.appendChild(img);

      const removeBtn = document.createElement("button");
      removeBtn.className = "ref-card-remove";
      removeBtn.textContent = "✕";
      removeBtn.title = "移除";
      removeBtn.onclick = (e) => {
        e.stopPropagation();
        referenceImages.splice(idx, 1);
        renderReferenceImages();
      };
      thumbWrap.appendChild(removeBtn);
      card.appendChild(thumbWrap);

      const descInput = document.createElement("input");
      descInput.type = "text";
      descInput.className = "ref-card-desc";
      descInput.placeholder = "说明...";
      descInput.value = ref.description;
      descInput.addEventListener("input", () => {
        referenceImages[idx].description = descInput.value;
      });
      card.appendChild(descInput);

      refImagesList.appendChild(card);
    });
  }

  function collectReferenceData() {
    return referenceImages.map((ref) => {
      const b64 = ref.dataUrl.includes(",")
        ? ref.dataUrl.split(",")[1]
        : ref.dataUrl;
      return { data: b64, description: ref.description };
    });
  }

  // ── Submit edit ────────────────────────────────────────
  async function submitEdit() {
    const prompt = promptInput.value.trim();
    if (!prompt || !currentSessionId || !sessionData) return;

    // Determine parent node
    let parentId;
    if (editFromNodeId) {
      parentId = editFromNodeId;
      editFromNodeId = null;
    } else {
      // Last node in active path
      parentId = sessionData.active_path[sessionData.active_path.length - 1];
    }

    // Read module toggles
    const modules = {
      retouch: !!document.querySelector('.module-toggle[data-module="retouch"].active'),
      background: !!document.querySelector('.module-toggle[data-module="background"].active'),
      effects: !!document.querySelector('.module-toggle[data-module="effects"].active'),
    };

    // Collect reference images
    const refData = collectReferenceData();

    promptInput.value = "";
    btnSend.disabled = true;

    const result = await api().submit_edit(currentSessionId, parentId, prompt, modules, refData);
    if (result.error) {
      alert("提交失败: " + result.error);
      btnSend.disabled = false;
      return;
    }

    // Clear reference images after submission
    referenceImages = [];
    renderReferenceImages();

    // Update active path
    sessionData.active_path = result.active_path;
    await refreshSession();
    btnSend.disabled = false;
  }

  // ── Polling ────────────────────────────────────────────
  function startPolling(nodeId) {
    if (pollingTimers[nodeId]) return;

    pollingTimers[nodeId] = setInterval(async () => {
      const status = await api().get_node_status(currentSessionId, nodeId);
      if (!status) return;

      if (status.status === "processing") {
        const el = document.getElementById(`processing-${nodeId}`);
        if (el) {
          const msg =
            status.progress_total > 0
              ? `步骤 ${status.progress_step}/${status.progress_total}: ${status.progress_msg}`
              : status.progress_msg || "处理中...";
          el.querySelector("span").textContent = msg;
        }
      } else if (status.status === "done" || status.status === "error") {
        stopPolling(nodeId);
        await refreshSession();
      }
    }, 500);
  }

  function stopPolling(nodeId) {
    if (pollingTimers[nodeId]) {
      clearInterval(pollingTimers[nodeId]);
      delete pollingTimers[nodeId];
    }
  }

  // ── Image viewer ───────────────────────────────────────
  async function showImageViewer(sessionId, nodeId) {
    viewerSessionId = sessionId;
    viewerNodeId = nodeId;
    imageViewer.style.display = "flex";
    viewerImg.src = "";
    const dataUrl = await api().get_image(sessionId, nodeId, false);
    if (dataUrl) {
      viewerImg.src = dataUrl;
    }
  }

  function hideImageViewer() {
    imageViewer.style.display = "none";
    viewerImg.src = "";
    viewerSessionId = null;
    viewerNodeId = null;
  }

  async function exportCurrentImage() {
    if (!viewerSessionId || !viewerNodeId) return;
    const result = await api().export_image(viewerSessionId, viewerNodeId);
    if (result.error) alert("导出失败: " + result.error);
  }

  // ── UI mode switching ──────────────────────────────────
  function showChatMode() {
    welcome.style.display = "none";
    messagesEl.style.display = "flex";
    inputBar.style.display = "flex";
    document.getElementById("pipeline-modules").style.display = "flex";
  }

  function showWelcomeMode() {
    welcome.style.display = "flex";
    messagesEl.style.display = "none";
    inputBar.style.display = "none";
    document.getElementById("pipeline-modules").style.display = "none";
    currentSessionId = null;
    sessionData = null;
    editFromNodeId = null;
    referenceImages = [];
    renderReferenceImages();
    // Stop all polling
    Object.keys(pollingTimers).forEach(stopPolling);
  }

  // ── Settings ───────────────────────────────────────────

  // Prompt key → textarea element id mapping
  const PROMPT_FIELDS = {
    detect_scene_type: "s-prompt-detect",
    analyze_background: "s-prompt-bg",
    retouch_image: "s-prompt-retouch",
    apply_cosplay_effect: "s-prompt-effect",
  };

  async function openSettings() {
    const settings = await api().get_settings();
    // Populate API fields
    document.getElementById("s-text-model").value = settings.text_model || "";
    document.getElementById("s-text-base-url").value = settings.text_base_url || "";
    document.getElementById("s-text-api-key").value = settings.text_api_key || "";
    document.getElementById("s-text-timeout").value = settings.text_timeout_ms || 180000;
    document.getElementById("s-image-model").value = settings.image_model || "";
    document.getElementById("s-image-base-url").value = settings.image_base_url || "";
    document.getElementById("s-image-api-key").value = settings.image_api_key || "";
    document.getElementById("s-image-timeout").value = settings.image_timeout_ms || 300000;
    // Populate prompt fields
    const prompts = settings.prompts || {};
    for (const [key, elId] of Object.entries(PROMPT_FIELDS)) {
      document.getElementById(elId).value = prompts[key] || "";
    }
    // Show modal, default to API tab
    switchSettingsTab("api");
    settingsModal.style.display = "flex";
  }

  function closeSettings() {
    settingsModal.style.display = "none";
  }

  async function saveSettingsFromUI() {
    const settings = {
      text_model: document.getElementById("s-text-model").value.trim(),
      text_base_url: document.getElementById("s-text-base-url").value.trim(),
      text_api_key: document.getElementById("s-text-api-key").value.trim(),
      text_timeout_ms: parseInt(document.getElementById("s-text-timeout").value, 10) || 180000,
      image_model: document.getElementById("s-image-model").value.trim(),
      image_base_url: document.getElementById("s-image-base-url").value.trim(),
      image_api_key: document.getElementById("s-image-api-key").value.trim(),
      image_timeout_ms: parseInt(document.getElementById("s-image-timeout").value, 10) || 300000,
      prompts: {},
    };
    for (const [key, elId] of Object.entries(PROMPT_FIELDS)) {
      settings.prompts[key] = document.getElementById(elId).value;
    }

    settingsSave.disabled = true;
    settingsSave.textContent = "保存中...";
    try {
      const result = await api().save_settings(settings);
      settingsSave.textContent = "已保存";
      setTimeout(() => closeSettings(), 400);
    } catch (err) {
      alert("保存失败: " + err);
    } finally {
      settingsSave.disabled = false;
      settingsSave.textContent = "保存";
    }
  }

  function switchSettingsTab(tabName) {
    // Update tab buttons
    document.querySelectorAll(".settings-tab").forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.tab === tabName);
    });
    // Update panes
    document.getElementById("pane-api").style.display = tabName === "api" ? "" : "none";
    document.getElementById("pane-prompts").style.display = tabName === "prompts" ? "" : "none";
  }

  async function resetSinglePrompt(promptKey) {
    // Fetch defaults if not cached
    if (!cachedDefaults) {
      cachedDefaults = await api().get_default_settings();
    }
    const defaultPrompts = cachedDefaults.prompts || {};
    const elId = PROMPT_FIELDS[promptKey];
    if (elId && defaultPrompts[promptKey] !== undefined) {
      document.getElementById(elId).value = defaultPrompts[promptKey];
    }
  }

  // ── History modal ─────────────────────────────────────
  async function openHistoryModal() {
    historyList.innerHTML = "";
    historyModal.style.display = "flex";

    const sessions = await api().list_sessions();
    if (sessions.length === 0) {
      historyEmpty.style.display = "";
      historyList.style.display = "none";
      return;
    }
    historyEmpty.style.display = "none";
    historyList.style.display = "flex";

    for (const s of sessions) {
      const item = document.createElement("div");
      item.className = "history-item";

      // Thumbnail
      const thumb = document.createElement("img");
      thumb.className = "history-item-thumb";
      thumb.alt = "";
      item.appendChild(thumb);

      // Async load thumbnail
      (async () => {
        const dataUrl = await api().get_image(s.session_id, s.root_id, true);
        if (dataUrl) thumb.src = dataUrl;
      })();

      // Info
      const info = document.createElement("div");
      info.className = "history-item-info";

      const note = document.createElement("div");
      note.className = "history-item-note";
      note.textContent = s.note || s.session_id;
      info.appendChild(note);

      const meta = document.createElement("div");
      meta.className = "history-item-meta";
      const date = new Date(s.created_at * 1000);
      meta.textContent = date.toLocaleString("zh-CN") + " · " + s.node_count + " 节点";
      info.appendChild(meta);

      item.appendChild(info);

      // Delete button
      const delBtn = document.createElement("button");
      delBtn.className = "history-item-delete";
      delBtn.title = "删除";
      delBtn.textContent = "✕";
      delBtn.onclick = async (e) => {
        e.stopPropagation();
        if (!confirm("确定删除此会话？此操作不可撤销。")) return;
        await api().delete_session(s.session_id);
        if (currentSessionId === s.session_id) {
          showWelcomeMode();
        }
        // Refresh list
        openHistoryModal();
      };
      item.appendChild(delBtn);

      // Click to load session
      item.onclick = async () => {
        currentSessionId = s.session_id;
        editFromNodeId = null;
        await refreshSession();
        showChatMode();
        closeHistoryModal();
      };

      historyList.appendChild(item);
    }
  }

  function closeHistoryModal() {
    historyModal.style.display = "none";
  }

  // ── Help modal ────────────────────────────────────────
  function openHelpModal() {
    helpModal.style.display = "flex";
  }

  function closeHelpModal() {
    helpModal.style.display = "none";
  }

  // ── Utility ────────────────────────────────────────────
  function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  // ── Event listeners ────────────────────────────────────
  fileInput.addEventListener("change", (e) => {
    if (e.target.files[0]) handleUpload(e.target.files[0]);
    e.target.value = "";
  });

  fileInputInline.addEventListener("change", (e) => {
    if (e.target.files[0]) handleUpload(e.target.files[0]);
    e.target.value = "";
  });

  // Reference image file input
  refFileInput.addEventListener("change", async (e) => {
    for (const file of e.target.files) {
      if (!file.type.startsWith("image/")) continue;
      const dataUrl = await readFileAsBase64(file);
      referenceImages.push({ dataUrl, description: "" });
    }
    renderReferenceImages();
    e.target.value = "";
  });

  btnSend.addEventListener("click", submitEdit);

  promptInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submitEdit();
    }
  });

  btnNewSession.addEventListener("click", () => {
    showWelcomeMode();
    // Trigger file picker after switching to welcome screen
    setTimeout(() => fileInput.click(), 100);
  });

  // Image viewer events
  imageViewer.addEventListener("click", hideImageViewer);
  viewerClose.addEventListener("click", (e) => {
    e.stopPropagation();
    hideImageViewer();
  });
  viewerExport.addEventListener("click", (e) => {
    e.stopPropagation();
    exportCurrentImage();
  });
  document.querySelector(".viewer-actions").addEventListener("click", (e) => {
    e.stopPropagation();
  });

  // History & Help events
  btnHistory.addEventListener("click", openHistoryModal);
  btnHelp.addEventListener("click", openHelpModal);
  historyClose.addEventListener("click", closeHistoryModal);
  helpClose.addEventListener("click", closeHelpModal);
  historyModal.addEventListener("click", (e) => {
    if (e.target === historyModal) closeHistoryModal();
  });
  helpModal.addEventListener("click", (e) => {
    if (e.target === helpModal) closeHelpModal();
  });

  // Settings events
  btnSettings.addEventListener("click", openSettings);
  settingsClose.addEventListener("click", closeSettings);
  settingsCancel.addEventListener("click", closeSettings);
  settingsSave.addEventListener("click", saveSettingsFromUI);

  // Tab switching
  document.querySelectorAll(".settings-tab").forEach((btn) => {
    btn.addEventListener("click", () => switchSettingsTab(btn.dataset.tab));
  });

  // Individual prompt reset buttons
  document.querySelectorAll(".btn-reset-prompt").forEach((btn) => {
    btn.addEventListener("click", () => resetSinglePrompt(btn.dataset.prompt));
  });

  // Module toggle buttons
  document.querySelectorAll(".module-toggle").forEach((btn) => {
    btn.addEventListener("click", () => {
      const activeToggles = document.querySelectorAll(".module-toggle.active");
      // Prevent deactivating the last active toggle
      if (btn.classList.contains("active") && activeToggles.length <= 1) return;
      btn.classList.toggle("active");
    });
  });

  // Settings modal: click backdrop to close
  settingsModal.addEventListener("click", (e) => {
    if (e.target === settingsModal) closeSettings();
  });

  // ESC key closes modals
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      if (historyModal.style.display !== "none") {
        closeHistoryModal();
      } else if (helpModal.style.display !== "none") {
        closeHelpModal();
      } else if (settingsModal.style.display !== "none") {
        closeSettings();
      } else if (imageViewer.style.display !== "none") {
        hideImageViewer();
      }
    }
  });

  // Drag-and-drop on upload zone
  const uploadZone = document.getElementById("upload-zone");
  uploadZone.addEventListener("dragover", (e) => {
    e.preventDefault();
    uploadZone.style.borderColor = "var(--accent)";
  });
  uploadZone.addEventListener("dragleave", () => {
    uploadZone.style.borderColor = "";
  });
  uploadZone.addEventListener("drop", (e) => {
    e.preventDefault();
    uploadZone.style.borderColor = "";
    if (e.dataTransfer.files[0]) handleUpload(e.dataTransfer.files[0]);
  });

  // ── Init ───────────────────────────────────────────────
  async function init() {
    await waitForApi();

    // Check for existing sessions
    const sessions = await api().list_sessions();
    if (sessions.length > 0) {
      // Resume the most recent session
      currentSessionId = sessions[0].session_id;
      await refreshSession();
      showChatMode();
    }
  }

  init();
})();
