import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type ConfigType = "claude" | "gemini" | "codex";

interface Config {
  id: string;
  name: string;
  config_type: ConfigType;
  api_key: string;
  base_url: string;
  is_active: boolean;
}

const CONFIG_TYPE_LABELS: Record<ConfigType, string> = {
  claude: "Claude",
  gemini: "Gemini",
  codex: "Codex",
};

const CONFIG_TYPE_COLORS: Record<ConfigType, string> = {
  claude: "#f97316", // orange
  gemini: "#3b82f6", // blue
  codex: "#8b5cf6", // purple
};

let configs: Config[] = [];
let editingConfig: Config | null = null;
let currentTab: ConfigType | "opencode" = "claude";

async function loadConfigs() {
  try {
    configs = await invoke<Config[]>("get_configs");
    renderConfigs();
  } catch (e) {
    console.error("Failed to load configs:", e);
  }
}

async function saveConfig(config: Omit<Config, "id" | "is_active">) {
  try {
    if (editingConfig) {
      await invoke("update_config", {
        id: editingConfig.id,
        name: config.name,
        apiKey: config.api_key,
        baseUrl: config.base_url,
      });
    } else {
      await invoke("add_config", {
        name: config.name,
        configType: config.config_type,
        apiKey: config.api_key,
        baseUrl: config.base_url,
      });
    }
    await loadConfigs();
    closeModal();
    showToast(editingConfig ? "配置已更新" : "配置已添加");
  } catch (e) {
    console.error("Failed to save config:", e);
    showToast("保存失败");
  }
}

async function deleteConfig(id: string) {
  try {
    await invoke("delete_config", { id });
    await loadConfigs();
    showToast("配置已删除");
  } catch (e) {
    console.error("Failed to delete config:", e);
  }
}

async function activateConfig(id: string) {
  showLoading("正在切换配置...");
  try {
    await invoke("activate_config", { id });
    await loadConfigs();
    hideLoading();
    showToast("配置已激活");
  } catch (e) {
    console.error("Failed to activate config:", e);
    hideLoading();
    showToast("切换失败");
  }
}

async function applyOpenCodeConfig() {
  const claudeSelect = document.getElementById("opencode-claude") as HTMLSelectElement;
  const geminiSelect = document.getElementById("opencode-gemini") as HTMLSelectElement;
  const codexSelect = document.getElementById("opencode-codex") as HTMLSelectElement;

  const claudeId = claudeSelect?.value || null;
  const geminiId = geminiSelect?.value || null;
  const codexId = codexSelect?.value || null;

  if (!claudeId && !geminiId && !codexId) {
    showToast("请至少选择一个配置");
    return;
  }

  showLoading("正在应用 OpenCode 配置...");
  try {
    await invoke("apply_opencode_config", {
      claudeId: claudeId || null,
      geminiId: geminiId || null,
      codexId: codexId || null,
    });
    hideLoading();
    showToast("OpenCode 配置已应用");
  } catch (e) {
    console.error("Failed to apply opencode config:", e);
    hideLoading();
    showToast("应用失败: " + e);
  }
}

function getConfigsByType(type: ConfigType): Config[] {
  return configs.filter((c) => c.config_type === type);
}

function getKeyLabel(type: ConfigType): string {
  switch (type) {
    case "claude":
      return "ANTHROPIC_AUTH_TOKEN";
    case "gemini":
      return "GEMINI_API_KEY";
    case "codex":
      return "API Key";
  }
}

function getUrlLabel(type: ConfigType): string {
  switch (type) {
    case "claude":
      return "ANTHROPIC_BASE_URL";
    case "gemini":
      return "GOOGLE_GEMINI_BASE_URL";
    case "codex":
      return "Base URL";
  }
}

function renderConfigs() {
  const app = document.getElementById("app")!;

  const tabConfigs = currentTab === "opencode" ? [] : getConfigsByType(currentTab);
  const activeConfig = tabConfigs.find((c) => c.is_active);

  app.innerHTML = `
    <div class="header" id="drag-region">
      <h1>Config Manager</h1>
      <div class="header-actions">
        ${
          currentTab !== "opencode"
            ? `
        <button class="btn btn-primary" onclick="openModal()">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
          添加
        </button>
        `
            : ""
        }
      </div>
    </div>

    <div class="tabs">
      <button class="tab ${currentTab === "claude" ? "active" : ""}" onclick="switchTab('claude')" style="--tab-color: ${CONFIG_TYPE_COLORS.claude}">
        Claude
      </button>
      <button class="tab ${currentTab === "gemini" ? "active" : ""}" onclick="switchTab('gemini')" style="--tab-color: ${CONFIG_TYPE_COLORS.gemini}">
        Gemini
      </button>
      <button class="tab ${currentTab === "codex" ? "active" : ""}" onclick="switchTab('codex')" style="--tab-color: ${CONFIG_TYPE_COLORS.codex}">
        Codex
      </button>
      <button class="tab ${currentTab === "opencode" ? "active" : ""}" onclick="switchTab('opencode')" style="--tab-color: #10b981">
        OpenCode
      </button>
    </div>

    ${currentTab === "opencode" ? renderOpenCodePanel() : renderConfigList(tabConfigs)}

    <div class="status-bar">
      <span>共 ${configs.length} 个配置</span>
      ${
        currentTab !== "opencode"
          ? `
      <span class="${activeConfig ? "status-active" : ""}">
        ${activeConfig ? `当前: ${escapeHtml(activeConfig.name)}` : "未激活配置"}
      </span>
      `
          : '<span class="status-active">选择配置并应用</span>'
      }
    </div>
  `;

  setupDragRegion();
}

function renderConfigList(tabConfigs: Config[]): string {
  if (tabConfigs.length === 0) {
    return `
      <div class="config-list">
        <div class="empty-state">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M9 12h6m-3-3v6m-7 4h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
          </svg>
          <p>暂无 ${CONFIG_TYPE_LABELS[currentTab as ConfigType]} 配置</p>
        </div>
      </div>
    `;
  }

  return `
    <div class="config-list">
      ${tabConfigs
        .map(
          (config) => `
        <div class="config-item ${config.is_active ? "active" : ""}" onclick="activateConfig('${config.id}')" style="--type-color: ${CONFIG_TYPE_COLORS[config.config_type]}">
          <div class="config-header">
            <div class="config-name-wrapper">
              <span class="config-type-badge" style="background: ${CONFIG_TYPE_COLORS[config.config_type]}">${CONFIG_TYPE_LABELS[config.config_type]}</span>
              <span class="config-name">${escapeHtml(config.name)}</span>
            </div>
            <div class="config-actions">
              ${config.is_active ? '<span class="active-badge">当前</span>' : ""}
              <button class="btn btn-icon" onclick="event.stopPropagation(); editConfig('${config.id}')" title="编辑">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/>
                  <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
                </svg>
              </button>
              <button class="btn btn-icon btn-danger" onclick="event.stopPropagation(); deleteConfig('${config.id}')" title="删除">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3,6 5,6 21,6"/>
                  <path d="M19,6v14a2,2,0,0,1-2,2H7a2,2,0,0,1-2-2V6m3,0V4a2,2,0,0,1,2-2h4a2,2,0,0,1,2,2v2"/>
                </svg>
              </button>
            </div>
          </div>
          <div class="config-details">
            <p><strong>Key:</strong> ${maskToken(config.api_key)}</p>
            <p><strong>URL:</strong> ${escapeHtml(config.base_url) || "默认"}</p>
          </div>
        </div>
      `
        )
        .join("")}
    </div>
  `;
}

function renderOpenCodePanel(): string {
  const claudeConfigs = getConfigsByType("claude");
  const geminiConfigs = getConfigsByType("gemini");
  const codexConfigs = getConfigsByType("codex");

  return `
    <div class="opencode-panel">
      <div class="opencode-info">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <path d="M12 16v-4"/>
          <path d="M12 8h.01"/>
        </svg>
        <p>从已有配置中选择，生成 OpenCode 配置文件</p>
      </div>

      <div class="opencode-selects">
        <div class="form-group">
          <label for="opencode-claude">Claude 配置</label>
          <select id="opencode-claude">
            <option value="">-- 不使用 --</option>
            ${claudeConfigs.map((c) => `<option value="${c.id}">${escapeHtml(c.name)}</option>`).join("")}
          </select>
        </div>

        <div class="form-group">
          <label for="opencode-gemini">Gemini 配置</label>
          <select id="opencode-gemini">
            <option value="">-- 不使用 --</option>
            ${geminiConfigs.map((c) => `<option value="${c.id}">${escapeHtml(c.name)}</option>`).join("")}
          </select>
        </div>

        <div class="form-group">
          <label for="opencode-codex">Codex/OpenAI 配置</label>
          <select id="opencode-codex">
            <option value="">-- 不使用 --</option>
            ${codexConfigs.map((c) => `<option value="${c.id}">${escapeHtml(c.name)}</option>`).join("")}
          </select>
        </div>
      </div>

      <button class="btn btn-primary btn-full" onclick="applyOpenCodeConfig()">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M5 12l5 5L20 7"/>
        </svg>
        应用到 OpenCode
      </button>

      <div class="opencode-path">
        <small>配置将写入: ~/.config/opencode/opencode.json</small>
      </div>
    </div>
  `;
}

function switchTab(tab: ConfigType | "opencode") {
  currentTab = tab;
  renderConfigs();
}

function openModal(config?: Config) {
  editingConfig = config || null;
  const configType = config?.config_type || (currentTab === "opencode" ? "claude" : currentTab);

  const modal = document.createElement("div");
  modal.className = "modal-overlay";
  modal.id = "modal";
  modal.innerHTML = `
    <div class="modal">
      <h2>${config ? "编辑配置" : "添加配置"}</h2>
      <form id="config-form">
        <div class="form-group">
          <label for="name">配置名称</label>
          <input type="text" id="name" placeholder="例如: 个人账户" value="${escapeHtml(config?.name || "")}" required>
        </div>
        ${
          !config
            ? `
        <div class="form-group">
          <label for="config_type">配置类型</label>
          <select id="config_type" required>
            <option value="claude" ${configType === "claude" ? "selected" : ""}>Claude</option>
            <option value="gemini" ${configType === "gemini" ? "selected" : ""}>Gemini</option>
            <option value="codex" ${configType === "codex" ? "selected" : ""}>Codex</option>
          </select>
        </div>
        `
            : ""
        }
        <div class="form-group">
          <label for="api_key" id="key-label">${getKeyLabel(configType as ConfigType)}</label>
          <input type="password" id="api_key" placeholder="sk-..." value="${config?.api_key || ""}" required>
        </div>
        <div class="form-group">
          <label for="base_url" id="url-label">${getUrlLabel(configType as ConfigType)} (可选)</label>
          <input type="text" id="base_url" placeholder="https://api.example.com" value="${escapeHtml(config?.base_url || "")}">
        </div>
        <div class="modal-actions">
          <button type="button" class="btn btn-secondary" onclick="closeModal()">取消</button>
          <button type="submit" class="btn btn-primary">${config ? "保存" : "添加"}</button>
        </div>
      </form>
    </div>
  `;
  document.body.appendChild(modal);

  // Update labels when type changes
  const typeSelect = document.getElementById("config_type") as HTMLSelectElement;
  if (typeSelect) {
    typeSelect.addEventListener("change", () => {
      const type = typeSelect.value as ConfigType;
      document.getElementById("key-label")!.textContent = getKeyLabel(type);
      document.getElementById("url-label")!.textContent = getUrlLabel(type) + " (可选)";
    });
  }

  document.getElementById("config-form")!.onsubmit = (e) => {
    e.preventDefault();
    const name = (document.getElementById("name") as HTMLInputElement).value;
    const api_key = (document.getElementById("api_key") as HTMLInputElement).value;
    const base_url = (document.getElementById("base_url") as HTMLInputElement).value;
    const config_type = editingConfig
      ? editingConfig.config_type
      : ((document.getElementById("config_type") as HTMLSelectElement).value as ConfigType);
    saveConfig({ name, config_type, api_key, base_url });
  };

  modal.onclick = (e) => {
    if (e.target === modal) closeModal();
  };
}

function closeModal() {
  const modal = document.getElementById("modal");
  if (modal) modal.remove();
  editingConfig = null;
}

function editConfig(id: string) {
  const config = configs.find((c) => c.id === id);
  if (config) openModal(config);
}

function showLoading(message: string = "切换中...") {
  const existing = document.querySelector(".loading-overlay");
  if (existing) return;

  const overlay = document.createElement("div");
  overlay.className = "loading-overlay";
  overlay.innerHTML = `
    <div class="loading-spinner">
      <div class="spinner"></div>
      <div class="loading-text">${escapeHtml(message)}</div>
    </div>
  `;
  document.body.appendChild(overlay);
}

function hideLoading() {
  const overlay = document.querySelector(".loading-overlay");
  if (overlay) overlay.remove();
}

function showToast(message: string) {
  const existing = document.querySelector(".toast");
  if (existing) existing.remove();

  const toast = document.createElement("div");
  toast.className = "toast";
  toast.textContent = message;
  document.body.appendChild(toast);

  setTimeout(() => toast.remove(), 2000);
}

function escapeHtml(str: string): string {
  if (!str) return "";
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function maskToken(token: string): string {
  if (!token || token.length < 10) return "****";
  return token.slice(0, 7) + "..." + token.slice(-4);
}

function setupDragRegion() {
  const dragRegion = document.getElementById("drag-region");
  if (dragRegion) {
    dragRegion.addEventListener("mousedown", async (e) => {
      const target = e.target as HTMLElement;
      // 不拦截按钮点击
      if (target.closest("button") || target.closest(".btn")) {
        return;
      }
      await getCurrentWindow().startDragging();
    });
  }
}

// Expose functions to global scope for onclick handlers
(window as any).openModal = openModal;
(window as any).closeModal = closeModal;
(window as any).editConfig = editConfig;
(window as any).deleteConfig = deleteConfig;
(window as any).activateConfig = activateConfig;
(window as any).switchTab = switchTab;
(window as any).applyOpenCodeConfig = applyOpenCodeConfig;

// Initialize
loadConfigs();
