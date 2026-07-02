use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::engines::gemini::DEFAULT_MODEL;

const STORE_FILE: &str = "settings.json";
const STORE_KEY: &str = "settings";
const KEYRING_SERVICE: &str = "com.chienhua.mytranslator";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub default_engine: String,
    pub target_lang: String,
    pub hotkey_enabled: bool,
    pub gemini_model: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_engine: "google".into(),
            target_lang: "zh-TW".into(),
            hotkey_enabled: true,
            gemini_model: DEFAULT_MODEL.into(),
        }
    }
}

pub fn load(app: &AppHandle) -> AppSettings {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(STORE_KEY))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(STORE_KEY, json!(settings));
    store.save().map_err(|e| e.to_string())
}

/// API key 存 OS 憑證庫（Windows Credential Manager / macOS Keychain），不落明文
pub fn get_api_key(engine_id: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, engine_id)
        .ok()?
        .get_password()
        .ok()
        .filter(|k| !k.is_empty())
}

pub fn set_api_key(engine_id: &str, key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, engine_id).map_err(|e| e.to_string())?;
    if key.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        entry.set_password(key).map_err(|e| e.to_string())
    }
}
