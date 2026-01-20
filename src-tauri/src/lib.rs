use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub name: String,
    pub auth_token: String,
    pub base_url: String,
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

    let _environment: Vec<u16> = OsStr::new("Environment")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Note: For full functionality, you'd call SendMessageTimeoutW here
    // This simplified version works but new terminals need to be opened to see changes
}

#[cfg(target_os = "windows")]
fn delete_user_env_var(key: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_SET_VALUE)
        .map_err(|e| e.to_string())?;
    env.delete_value(key).ok(); // Ignore error if key doesn't exist
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

fn apply_config(config: &Config) -> Result<(), String> {
    set_user_env_var("ANTHROPIC_AUTH_TOKEN", &config.auth_token)?;

    if !config.base_url.is_empty() {
        set_user_env_var("ANTHROPIC_BASE_URL", &config.base_url)?;
    } else {
        delete_user_env_var("ANTHROPIC_BASE_URL")?;
    }

    Ok(())
}

#[tauri::command]
fn get_configs() -> Vec<Config> {
    load_store().configs
}

#[tauri::command]
fn add_config(name: String, auth_token: String, base_url: String) -> Result<Config, String> {
    let mut store = load_store();
    let config = Config {
        id: Uuid::new_v4().to_string(),
        name,
        auth_token,
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
    auth_token: String,
    base_url: String,
) -> Result<(), String> {
    let mut store = load_store();
    if let Some(config) = store.configs.iter_mut().find(|c| c.id == id) {
        config.name = name;
        config.auth_token = auth_token;
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
    let was_active = store.configs.iter().find(|c| c.id == id).map(|c| c.is_active).unwrap_or(false);
    store.configs.retain(|c| c.id != id);

    // If we deleted the active config, clear the environment variables
    if was_active {
        delete_user_env_var("ANTHROPIC_AUTH_TOKEN")?;
        delete_user_env_var("ANTHROPIC_BASE_URL")?;
    }

    save_store(&store)
}

#[tauri::command]
fn activate_config(id: String) -> Result<(), String> {
    let mut store = load_store();

    // Deactivate all configs first
    for config in &mut store.configs {
        config.is_active = false;
    }

    // Activate the selected config
    let config_to_apply = store
        .configs
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or("Config not found")?;

    config_to_apply.is_active = true;
    let config_clone = config_to_apply.clone();

    save_store(&store)?;
    apply_config(&config_clone)?;

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
