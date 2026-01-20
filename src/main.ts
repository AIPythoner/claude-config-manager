import { invoke } from "@tauri-apps/api/core";

interface Config {
  id: string;
  name: string;
  auth_token: string;
  base_url: string;
  is_active: boolean;
}

let configs: Config[] = [];
let editingConfig: Config | null = null;

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
        authToken: config.auth_token,
        baseUrl: config.base_url,
      });
    } else {
      await invoke("add_config", {
        name: config.name,
        authToken: config.auth_token,
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
  try {
    await invoke("activate_config", { id });
    await loadConfigs();
    showToast("配置已激活");
  } catch (e) {
    console.error("Failed to activate config:", e);
  }
}

function renderConfigs() {
  const app = document.getElementById("app")!;
  const activeConfig = configs.find((c) => c.is_active);

  app.innerHTML = `
    <div class="header">
      <h1>Claude Config Manager</h1>
      <div class="header-actions">
        <button class="btn btn-primary" onclick="openModal()">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
          添加
        </button>
      </div>
    </div>

    <div class="config-list">
      ${
        configs.length === 0
          ? `
        <div class="empty-state">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M9 12h6m-3-3v6m-7 4h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
          </svg>
          <p>暂无配置，点击上方添加按钮创建</p>
        </div>
      `
          : configs
              .map(
                (config) => `
        <div class="config-item ${config.is_active ? "active" : ""}" onclick="activateConfig('${config.id}')">
          <div class="config-header">
            <span class="config-name">${escapeHtml(config.name)}</span>
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
            <p><strong>Token:</strong> ${maskToken(config.auth_token)}</p>
            <p><strong>URL:</strong> ${escapeHtml(config.base_url) || "默认"}</p>
          </div>
        </div>
      `
              )
              .join("")
      }
    </div>

    <div class="status-bar">
      <span>共 ${configs.length} 个配置</span>
      <span class="${activeConfig ? "status-active" : ""}">
        ${activeConfig ? `当前: ${escapeHtml(activeConfig.name)}` : "未激活配置"}
      </span>
    </div>
  `;
}

function openModal(config?: Config) {
  editingConfig = config || null;
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
        <div class="form-group">
          <label for="auth_token">ANTHROPIC_AUTH_TOKEN</label>
          <input type="password" id="auth_token" placeholder="sk-ant-..." value="${config?.auth_token || ""}" required>
        </div>
        <div class="form-group">
          <label for="base_url">ANTHROPIC_BASE_URL (可选)</label>
          <input type="text" id="base_url" placeholder="https://api.anthropic.com" value="${escapeHtml(config?.base_url || "")}">
        </div>
        <div class="modal-actions">
          <button type="button" class="btn btn-secondary" onclick="closeModal()">取消</button>
          <button type="submit" class="btn btn-primary">${config ? "保存" : "添加"}</button>
        </div>
      </form>
    </div>
  `;
  document.body.appendChild(modal);

  document.getElementById("config-form")!.onsubmit = (e) => {
    e.preventDefault();
    const name = (document.getElementById("name") as HTMLInputElement).value;
    const auth_token = (document.getElementById("auth_token") as HTMLInputElement).value;
    const base_url = (document.getElementById("base_url") as HTMLInputElement).value;
    saveConfig({ name, auth_token, base_url });
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

// Expose functions to global scope for onclick handlers
(window as any).openModal = openModal;
(window as any).closeModal = closeModal;
(window as any).editConfig = editConfig;
(window as any).deleteConfig = deleteConfig;
(window as any).activateConfig = activateConfig;

// Initialize
loadConfigs();
