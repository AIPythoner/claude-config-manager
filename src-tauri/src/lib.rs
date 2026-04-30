use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigType {
    Claude,
    Gemini,
    Codex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub name: String,
    pub config_type: ConfigType,
    pub api_key: String,
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigStore {
    pub configs: Vec<Config>,
}

fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-config-manager");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("configs.json")
}

fn get_user_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn load_store() -> ConfigStore {
    let path = get_config_path();
    if !path.exists() {
        return ConfigStore::default();
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("load_store: failed to read {}: {}", path.display(), e);
            return ConfigStore::default();
        }
    };
    if content.trim().is_empty() {
        return ConfigStore::default();
    }
    match serde_json::from_str(&content) {
        Ok(store) => store,
        Err(e) => {
            // Don't silently overwrite the user's data on next save: back up the
            // unreadable file so it can be recovered, then start with an empty store.
            eprintln!("load_store: failed to parse {}: {}", path.display(), e);
            let backup = path.with_extension("json.broken");
            if let Err(be) = fs::copy(&path, &backup) {
                eprintln!("load_store: backup to {} failed: {}", backup.display(), be);
            } else {
                eprintln!("load_store: backed up unreadable file to {}", backup.display());
            }
            ConfigStore::default()
        }
    }
}

fn save_store(store: &ConfigStore) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

fn read_json_object(path: &Path) -> Result<serde_json::Value, String> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    if content.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

fn write_json_pretty(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

fn get_claude_settings_path() -> PathBuf {
    get_user_home().join(".claude").join("settings.json")
}

fn get_gemini_env_path() -> PathBuf {
    get_user_home().join(".gemini").join(".env")
}

fn update_claude_env(api_key: Option<&str>, base_url: Option<&str>, model: Option<&str>) -> Result<(), String> {
    let path = get_claude_settings_path();
    let mut json = read_json_object(&path)?;

    let obj = json
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?;

    let env_value = obj
        .entry("env".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let env_obj = env_value
        .as_object_mut()
        .ok_or_else(|| format!("{} 'env' field is not an object", path.display()))?;

    match api_key {
        Some(v) => {
            env_obj.insert(
                "ANTHROPIC_AUTH_TOKEN".to_string(),
                serde_json::Value::String(v.to_string()),
            );
        }
        None => {
            env_obj.remove("ANTHROPIC_AUTH_TOKEN");
        }
    }

    match base_url {
        Some(v) if !v.is_empty() => {
            env_obj.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                serde_json::Value::String(v.to_string()),
            );
        }
        _ => {
            env_obj.remove("ANTHROPIC_BASE_URL");
        }
    }

    match model {
        Some(v) if !v.is_empty() => {
            env_obj.insert(
                "ANTHROPIC_MODEL".to_string(),
                serde_json::Value::String(v.to_string()),
            );
        }
        _ => {
            env_obj.remove("ANTHROPIC_MODEL");
        }
    }

    // Always strip ANTHROPIC_API_KEY: it conflicts with ANTHROPIC_AUTH_TOKEN
    // when activating, and must be gone for OAuth fallback when clearing.
    env_obj.remove("ANTHROPIC_API_KEY");

    write_json_pretty(&path, &json)
}

fn update_gemini_env(api_key: Option<&str>, base_url: Option<&str>, model: Option<&str>) -> Result<(), String> {
    let path = get_gemini_env_path();

    let existing = if path.exists() {
        fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?
    } else {
        String::new()
    };

    let target_keys = ["GEMINI_API_KEY", "GOOGLE_GEMINI_BASE_URL", "GEMINI_MODEL"];
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return true;
            }
            let key = trimmed.split('=').next().unwrap_or("").trim();
            !target_keys.contains(&key)
        })
        .map(|s| s.to_string())
        .collect();

    if let Some(v) = api_key {
        lines.push(format!("GEMINI_API_KEY={}", v));
    }
    if let Some(v) = base_url {
        if !v.is_empty() {
            lines.push(format!("GOOGLE_GEMINI_BASE_URL={}", v));
        }
    }
    if let Some(v) = model {
        if !v.is_empty() {
            lines.push(format!("GEMINI_MODEL={}", v));
        }
    }

    let only_blank = lines.iter().all(|l| l.trim().is_empty());
    if only_blank {
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

fn apply_claude_config(config: &Config) -> Result<(), String> {
    update_claude_env(Some(&config.api_key), Some(&config.base_url), Some(&config.model))
}

fn apply_gemini_config(config: &Config) -> Result<(), String> {
    update_gemini_env(Some(&config.api_key), Some(&config.base_url), Some(&config.model))
}

fn apply_codex_config(config: &Config) -> Result<(), String> {
    let home = get_user_home();
    let codex_dir = home.join(".codex");

    // Create .codex directory if it doesn't exist
    fs::create_dir_all(&codex_dir).map_err(|e| format!("Failed to create .codex directory: {}", e))?;

    // Write auth.json - use OPENAI_API_KEY as the key name
    let auth_path = codex_dir.join("auth.json");
    let auth_content = serde_json::json!({
        "OPENAI_API_KEY": config.api_key
    });
    fs::write(&auth_path, serde_json::to_string_pretty(&auth_content).unwrap())
        .map_err(|e| format!("Failed to write auth.json: {}", e))?;

    // Write config.toml
    let config_path = codex_dir.join("config.toml");
    let base_url = if config.base_url.is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        config.base_url.clone()
    };
    let model = if config.model.is_empty() {
        "gpt-5.2-codex".to_string()
    } else {
        config.model.clone()
    };

    let config_content = format!(
r#"model_provider = "fox"
model = "{}"
model_reasoning_effort = "medium"
disable_response_storage = true

[model_providers.fox]
name = "fox"
base_url = "{}"
wire_api = "responses"
requires_openai_auth = true
"#, model, base_url);

    fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to write config.toml: {}", e))?;

    Ok(())
}

fn apply_config(config: &Config) -> Result<(), String> {
    match config.config_type {
        ConfigType::Claude => apply_claude_config(config),
        ConfigType::Gemini => apply_gemini_config(config),
        ConfigType::Codex => apply_codex_config(config),
    }
}

fn clear_claude_config() -> Result<(), String> {
    let path = get_claude_settings_path();
    if !path.exists() {
        return Ok(());
    }
    update_claude_env(None, None, None)
}

fn clear_gemini_config() -> Result<(), String> {
    update_gemini_env(None, None, None)
}

fn clear_codex_config() -> Result<(), String> {
    let home = get_user_home();
    let codex_dir = home.join(".codex");

    // Remove auth.json
    let auth_path = codex_dir.join("auth.json");
    if auth_path.exists() {
        fs::remove_file(&auth_path).ok();
    }

    // Remove config.toml
    let config_path = codex_dir.join("config.toml");
    if config_path.exists() {
        fs::remove_file(&config_path).ok();
    }

    Ok(())
}

fn clear_config(config_type: &ConfigType) -> Result<(), String> {
    match config_type {
        ConfigType::Claude => clear_claude_config(),
        ConfigType::Gemini => clear_gemini_config(),
        ConfigType::Codex => clear_codex_config(),
    }
}

#[tauri::command]
fn get_configs() -> Vec<Config> {
    load_store().configs
}

#[tauri::command]
fn add_config(name: String, config_type: String, api_key: String, base_url: String, model: String) -> Result<Config, String> {
    let mut store = load_store();

    let config_type_enum = match config_type.as_str() {
        "claude" => ConfigType::Claude,
        "gemini" => ConfigType::Gemini,
        "codex" => ConfigType::Codex,
        _ => return Err("Invalid config type".to_string()),
    };

    let config = Config {
        id: Uuid::new_v4().to_string(),
        name,
        config_type: config_type_enum,
        api_key,
        base_url,
        model,
        is_active: false,
    };
    store.configs.push(config.clone());
    save_store(&store)?;
    Ok(config)
}

#[tauri::command]
fn update_config(
    id: String,
    name: String,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<(), String> {
    let mut store = load_store();
    if let Some(config) = store.configs.iter_mut().find(|c| c.id == id) {
        config.name = name;
        config.api_key = api_key;
        config.base_url = base_url;
        config.model = model;

        // If this config is active, re-apply it
        if config.is_active {
            let config_clone = config.clone();
            save_store(&store)?;
            apply_config(&config_clone)?;
            return Ok(());
        }
    }
    save_store(&store)
}

#[tauri::command]
fn delete_config(id: String) -> Result<(), String> {
    let mut store = load_store();
    let config_to_delete = store.configs.iter().find(|c| c.id == id).cloned();

    if let Some(config) = config_to_delete {
        if config.is_active {
            clear_config(&config.config_type)?;
        }
    }

    store.configs.retain(|c| c.id != id);
    save_store(&store)
}

#[tauri::command]
fn activate_config(id: String) -> Result<(), String> {
    let mut store = load_store();

    // Find the config to activate
    let config_to_activate = store
        .configs
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .ok_or("Config not found")?;

    // Deactivate only configs of the same type
    for config in &mut store.configs {
        if config.config_type == config_to_activate.config_type {
            config.is_active = false;
        }
    }

    // Activate the selected config
    if let Some(config) = store.configs.iter_mut().find(|c| c.id == id) {
        config.is_active = true;
    }

    save_store(&store)?;
    apply_config(&config_to_activate)?;

    Ok(())
}

#[tauri::command]
fn deactivate_config(id: String) -> Result<(), String> {
    let mut store = load_store();

    if let Some(config) = store.configs.iter_mut().find(|c| c.id == id) {
        if config.is_active {
            config.is_active = false;
            let config_type = config.config_type.clone();
            save_store(&store)?;
            clear_config(&config_type)?;
        }
    }

    Ok(())
}

fn get_opencode_template() -> &'static str {
    r#"{
  "$schema": "https://opencode.ai/config.json",
  "tui": {
    "scroll_speed": 3,
    "scroll_acceleration": {
      "enabled": true
    }
  },
  "provider": {
    "foxcode-claude": {
      "npm": "@ai-sdk/anthropic",
      "name": "foxcode-claude",
      "options": {
        "baseURL": "https://api.anthropic.com",
        "apiKey": "",
        "headers": {
          "anthropic-beta": "prompt-caching-2024-07-31"
        }
      },
      "models": {
        "claude-sonnet-4-5-20250929": {
          "name": "claude-sonnet-4-5-20250929"
        },
        "claude-haiku-4-5-20251001": {
          "name": "claude-haiku-4-5-20251001"
        },
        "gemini-claude-opus-4-5-thinking": {
          "name": "gemini-claude-opus-4-5-thinking"
        },
        "claude-opus-4-5-20251101": {
          "name": "claude-opus-4-5-20251101"
        },
        "gemini-claude-sonnet-4-5-thinking": {
          "name": "gemini-claude-sonnet-4-5-thinking"
        }
      }
    },
    "foxcode-oai": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "foxcode-oai",
      "options": {
        "baseURL": "https://api.openai.com/v1",
        "apiKey": "",
        "setCacheKey": false
      },
      "models": {
        "gpt-5-high": {
          "name": "gpt-5-high"
        },
        "gpt-5.2-codex": {
          "name": "gpt-5.2-codex",
          "options": {
            "include": [
              "reasoning.encrypted_content"
            ],
            "store": false
          },
          "variants": {
            "xhigh": {
              "reasoningEffort": "xhigh",
              "textVerbosity": "medium",
              "reasoningSummary": "auto"
            },
            "high": {
              "reasoningEffort": "high",
              "textVerbosity": "medium",
              "reasoningSummary": "auto"
            },
            "medium": {
              "reasoningEffort": "medium",
              "textVerbosity": "medium",
              "reasoningSummary": "auto"
            },
            "low": {
              "reasoningEffort": "low",
              "textVerbosity": "medium",
              "reasoningSummary": "auto"
            }
          }
        },
        "gpt-5.2": {
          "id": "gpt-5.2",
          "name": "GPT-5.2",
          "tool_call": true,
          "attachment": true,
          "temperature": false,
          "reasoning": true,
          "release_date": "2025-12-11",
          "modalities": {
            "input": [
              "text",
              "image",
              "pdf"
            ],
            "output": [
              "text"
            ]
          },
          "cost": {
            "input": 1.75,
            "output": 14,
            "cache_read": 0.175
          },
          "limit": {
            "context": 400000,
            "output": 128000
          },
          "options": {
            "store": false,
            "reasoningEffort": "medium",
            "textVerbosity": "medium",
            "include": [
              "reasoning.encrypted_content"
            ]
          }
        },
        "gpt-5.1-codex-max": {
          "name": "gpt-5.1-codex-max",
          "options": {
            "include": [
              "reasoning.encrypted_content"
            ],
            "store": false
          },
          "variants": {
            "xhigh": {
              "reasoningEffort": "xhigh",
              "textVerbosity": "medium",
              "reasoningSummary": "auto"
            },
            "high": {
              "reasoningEffort": "high",
              "textVerbosity": "medium",
              "reasoningSummary": "auto"
            },
            "medium": {
              "reasoningEffort": "medium",
              "textVerbosity": "medium",
              "reasoningSummary": "auto"
            },
            "low": {
              "reasoningEffort": "low",
              "textVerbosity": "medium",
              "reasoningSummary": "auto"
            }
          }
        }
      }
    },
    "foxcode-gemini": {
      "npm": "@ai-sdk/google",
      "name": "foxcode-gemini",
      "options": {
        "baseURL": "https://generativelanguage.googleapis.com/v1beta",
        "apiKey": ""
      },
      "models": {
        "gemini-3-pro": {
          "name": "gemini-3-pro"
        },
        "gemini-2.5-flash-lite": {
          "name": "gemini-2.5-flash-lite"
        },
        "gemini-3-pro-preview": {
          "name": "gemini-3-pro-preview"
        },
        "gemini-3-flash-preview": {
          "name": "gemini-3-flash-preview"
        }
      }
    }
  },
  "model": "foxcode-claude/claude-opus-4-5-20251101",
  "small_model": "foxcode-claude/claude-haiku-4-5-20251001",
  "plugin": [],
  "mcp": {}
}"#
}

#[tauri::command]
fn apply_opencode_config(
    claude_id: Option<String>,
    gemini_id: Option<String>,
    codex_id: Option<String>,
    primary: Option<String>,
) -> Result<Option<String>, String> {
    let store = load_store();
    let home = get_user_home();
    let opencode_dir = home.join(".config").join("opencode");
    let config_path = opencode_dir.join("opencode.json");

    fs::create_dir_all(&opencode_dir).map_err(|e| format!("Failed to create opencode directory: {}", e))?;

    let existing_content = if config_path.exists() {
        fs::read_to_string(&config_path).unwrap_or_else(|_| get_opencode_template().to_string())
    } else {
        get_opencode_template().to_string()
    };

    let mut json_value: serde_json::Value = serde_json::from_str(&existing_content)
        .unwrap_or_else(|_| serde_json::from_str(get_opencode_template()).unwrap());

    let providers = json_value.get_mut("provider")
        .ok_or("No 'provider' field found in opencode.json")?;

    // Resolve a (provider_key, config) pair for each tab if a config was selected.
    let resolve = |id: Option<&String>, ty: ConfigType| -> Option<Config> {
        let id = id?;
        store.configs.iter().find(|c| &c.id == id && c.config_type == ty).cloned()
    };
    let claude_cfg = resolve(claude_id.as_ref(), ConfigType::Claude);
    let gemini_cfg = resolve(gemini_id.as_ref(), ConfigType::Gemini);
    let codex_cfg = resolve(codex_id.as_ref(), ConfigType::Codex);

    // Phase 1: write apiKey/baseURL into each selected provider block.
    let writes: [(&str, &Option<Config>); 3] = [
        ("foxcode-claude", &claude_cfg),
        ("foxcode-gemini", &gemini_cfg),
        ("foxcode-oai", &codex_cfg),
    ];
    for (provider_key, cfg) in &writes {
        if let Some(config) = cfg {
            if let Some(provider) = providers.get_mut(*provider_key) {
                if let Some(options) = provider.get_mut("options") {
                    options["apiKey"] = serde_json::Value::String(config.api_key.clone());
                    if !config.base_url.is_empty() {
                        options["baseURL"] = serde_json::Value::String(config.base_url.clone());
                    }
                }
            }
        }
    }

    // Phase 2: pick top-level model from the user-chosen primary, with fallback.
    let pick_model = |cfg: &Option<Config>, provider_key: &str| -> Option<String> {
        cfg.as_ref()
            .filter(|c| !c.model.is_empty())
            .map(|c| format!("{}/{}", provider_key, c.model))
    };
    let primary_pick = match primary.as_deref() {
        Some("claude") => pick_model(&claude_cfg, "foxcode-claude"),
        Some("gemini") => pick_model(&gemini_cfg, "foxcode-gemini"),
        Some("codex") => pick_model(&codex_cfg, "foxcode-oai"),
        _ => None,
    };
    let model_to_set = primary_pick
        .or_else(|| pick_model(&claude_cfg, "foxcode-claude"))
        .or_else(|| pick_model(&gemini_cfg, "foxcode-gemini"))
        .or_else(|| pick_model(&codex_cfg, "foxcode-oai"));

    if let Some(ref m) = model_to_set {
        json_value["model"] = serde_json::Value::String(m.clone());
    }

    let content = serde_json::to_string_pretty(&json_value).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| format!("Failed to write opencode.json: {}", e))?;

    Ok(model_to_set)
}

#[tauri::command]
fn restore_claude_login() -> Result<(), String> {
    let mut store = load_store();
    // Deactivate any active Claude config
    for config in &mut store.configs {
        if config.config_type == ConfigType::Claude && config.is_active {
            config.is_active = false;
        }
    }
    save_store(&store)?;
    clear_claude_config()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_configs,
            add_config,
            update_config,
            delete_config,
            activate_config,
            deactivate_config,
            restore_claude_login,
            apply_opencode_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
