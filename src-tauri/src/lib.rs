use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

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
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigStore {
    pub configs: Vec<Config>,
}

// OpenCode configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeProvider {
    name: String,
    #[serde(rename = "type")]
    provider_type: String,
    #[serde(rename = "apiKey")]
    api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "baseURL")]
    base_url: Option<String>,
    options: OpenCodeOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeOptions {
    model: String,
    #[serde(rename = "maxTokens")]
    max_tokens: i32,
    temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeConfig {
    #[serde(rename = "$schema")]
    schema: String,
    providers: std::collections::HashMap<String, OpenCodeProvider>,
}

// Codex auth.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexAuth {
    #[serde(rename = "apiKey")]
    api_key: String,
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
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        ConfigStore::default()
    }
}

fn save_store(store: &ConfigStore) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn set_user_env_var(key: &str, value: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_SET_VALUE)
        .map_err(|e| e.to_string())?;
    env.set_value(key, &value).map_err(|e| e.to_string())?;

    // Broadcast WM_SETTINGCHANGE to notify other applications
    broadcast_settings_change();

    Ok(())
}

#[cfg(target_os = "windows")]
fn broadcast_settings_change() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let environment: Vec<u16> = OsStr::new("Environment")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        const HWND_BROADCAST: isize = 0xffff;
        const WM_SETTINGCHANGE: u32 = 0x001A;
        const SMTO_ABORTIFHUNG: u32 = 0x0002;

        #[link(name = "user32")]
        extern "system" {
            fn SendMessageTimeoutW(
                hwnd: isize,
                msg: u32,
                wparam: usize,
                lparam: *const u16,
                flags: u32,
                timeout: u32,
                result: *mut usize,
            ) -> isize;
        }

        let mut result: usize = 0;
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            environment.as_ptr(),
            SMTO_ABORTIFHUNG,
            5000, // 5 seconds timeout
            &mut result,
        );
    }
}

#[cfg(target_os = "windows")]
fn delete_user_env_var(key: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_SET_VALUE)
        .map_err(|e| e.to_string())?;
    env.delete_value(key).ok(); // Ignore error if key doesn't exist
    broadcast_settings_change();
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_user_env_var(_key: &str, _value: &str) -> Result<(), String> {
    Err("Environment variable modification is only supported on Windows".to_string())
}

#[cfg(not(target_os = "windows"))]
fn delete_user_env_var(_key: &str) -> Result<(), String> {
    Err("Environment variable modification is only supported on Windows".to_string())
}

fn apply_claude_config(config: &Config) -> Result<(), String> {
    set_user_env_var("ANTHROPIC_AUTH_TOKEN", &config.api_key)?;

    if !config.base_url.is_empty() {
        set_user_env_var("ANTHROPIC_BASE_URL", &config.base_url)?;
    } else {
        delete_user_env_var("ANTHROPIC_BASE_URL")?;
    }

    Ok(())
}

fn apply_gemini_config(config: &Config) -> Result<(), String> {
    set_user_env_var("GEMINI_API_KEY", &config.api_key)?;

    if !config.base_url.is_empty() {
        set_user_env_var("GOOGLE_GEMINI_BASE_URL", &config.base_url)?;
    } else {
        delete_user_env_var("GOOGLE_GEMINI_BASE_URL")?;
    }

    Ok(())
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

    // Write config.toml - keep the exact format from reference file
    let config_path = codex_dir.join("config.toml");
    let base_url = if config.base_url.is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        config.base_url.clone()
    };

    let config_content = format!(
r#"model_provider = "fox"
model = "gpt-5.2"
model_reasoning_effort = "medium"
disable_response_storage = true

[model_providers.fox]
name = "fox"
base_url = "{}"
wire_api = "responses"
requires_openai_auth = true

[notice.model_migrations]
gpt-5 = "gpt-5.2-codex"
"gpt-5.2" = "gpt-5.2-codex"
"#, base_url);

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
    delete_user_env_var("ANTHROPIC_AUTH_TOKEN")?;
    delete_user_env_var("ANTHROPIC_BASE_URL")?;
    Ok(())
}

fn clear_gemini_config() -> Result<(), String> {
    delete_user_env_var("GEMINI_API_KEY")?;
    delete_user_env_var("GOOGLE_GEMINI_BASE_URL")?;
    Ok(())
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
fn add_config(name: String, config_type: String, api_key: String, base_url: String) -> Result<Config, String> {
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
) -> Result<(), String> {
    let mut store = load_store();
    if let Some(config) = store.configs.iter_mut().find(|c| c.id == id) {
        config.name = name;
        config.api_key = api_key;
        config.base_url = base_url;

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
fn apply_opencode_config(claude_id: Option<String>, gemini_id: Option<String>, codex_id: Option<String>) -> Result<(), String> {
    let store = load_store();
    let home = get_user_home();
    let opencode_dir = home.join(".config").join("opencode");
    let config_path = opencode_dir.join("opencode.json");

    // Create directory if it doesn't exist
    fs::create_dir_all(&opencode_dir).map_err(|e| format!("Failed to create opencode directory: {}", e))?;

    // Read existing file or use template
    let existing_content = if config_path.exists() {
        fs::read_to_string(&config_path).unwrap_or_else(|_| get_opencode_template().to_string())
    } else {
        get_opencode_template().to_string()
    };

    let mut json_value: serde_json::Value = serde_json::from_str(&existing_content)
        .unwrap_or_else(|_| serde_json::from_str(get_opencode_template()).unwrap());

    let providers = json_value.get_mut("provider")
        .ok_or("No 'provider' field found in opencode.json")?;

    // Update Claude provider (foxcode-claude) if selected
    if let Some(id) = claude_id {
        if let Some(config) = store.configs.iter().find(|c| c.id == id && c.config_type == ConfigType::Claude) {
            if let Some(provider) = providers.get_mut("foxcode-claude") {
                if let Some(options) = provider.get_mut("options") {
                    options["apiKey"] = serde_json::Value::String(config.api_key.clone());
                    if !config.base_url.is_empty() {
                        options["baseURL"] = serde_json::Value::String(config.base_url.clone());
                    }
                }
            }
        }
    }

    // Update Gemini provider (foxcode-gemini) if selected
    if let Some(id) = gemini_id {
        if let Some(config) = store.configs.iter().find(|c| c.id == id && c.config_type == ConfigType::Gemini) {
            if let Some(provider) = providers.get_mut("foxcode-gemini") {
                if let Some(options) = provider.get_mut("options") {
                    options["apiKey"] = serde_json::Value::String(config.api_key.clone());
                    if !config.base_url.is_empty() {
                        options["baseURL"] = serde_json::Value::String(config.base_url.clone());
                    }
                }
            }
        }
    }

    // Update OpenAI/Codex provider (foxcode-oai) if selected
    if let Some(id) = codex_id {
        if let Some(config) = store.configs.iter().find(|c| c.id == id && c.config_type == ConfigType::Codex) {
            if let Some(provider) = providers.get_mut("foxcode-oai") {
                if let Some(options) = provider.get_mut("options") {
                    options["apiKey"] = serde_json::Value::String(config.api_key.clone());
                    if !config.base_url.is_empty() {
                        options["baseURL"] = serde_json::Value::String(config.base_url.clone());
                    }
                }
            }
        }
    }

    // Write back the modified JSON
    let content = serde_json::to_string_pretty(&json_value).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| format!("Failed to write opencode.json: {}", e))?;

    Ok(())
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
            apply_opencode_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
