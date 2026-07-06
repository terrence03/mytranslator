mod engines;
mod hotkey;
mod settings;
mod tray;
mod update;
mod window;

use std::sync::atomic::Ordering;

use engines::{EngineContext, EngineInfo, EngineRegistry, TranslateRequest, TranslateResponse};
use settings::AppSettings;
use tauri::{AppHandle, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

struct AppState {
    registry: EngineRegistry,
}

#[tauri::command]
async fn translate(
    state: State<'_, AppState>,
    app: AppHandle,
    engine_id: String,
    text: String,
    target: String,
) -> Result<TranslateResponse, String> {
    let engine = state
        .registry
        .get(&engine_id)
        .ok_or_else(|| format!("未知的翻譯引擎：{engine_id}"))?;

    let s = settings::load(&app);
    let ctx = EngineContext {
        api_key: settings::get_api_key(&engine_id),
        model: Some(s.gemini_model),
    };
    let req = TranslateRequest {
        text,
        source: "auto".into(),
        target,
    };
    engine
        .translate(&req, &ctx)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_engines(state: State<'_, AppState>) -> Vec<EngineInfo> {
    state.registry.list()
}

#[tauri::command]
fn get_settings(app: AppHandle) -> AppSettings {
    settings::load(&app)
}

#[tauri::command]
fn update_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    hotkey::HOTKEY_ENABLED.store(settings.hotkey_enabled, Ordering::Relaxed);
    tray::sync_hotkey_checked(&app, settings.hotkey_enabled);
    settings::save(&app, &settings)
}

#[tauri::command]
fn set_api_key(engine_id: String, key: String) -> Result<(), String> {
    settings::set_api_key(&engine_id, &key)
}

#[tauri::command]
fn has_api_key(engine_id: String) -> bool {
    settings::get_api_key(&engine_id).is_some()
}

/// 用給定 key 打一次最小的翻譯請求驗證有效性；key 為空時退回已儲存的 key
#[tauri::command]
async fn validate_gemini_key(
    state: State<'_, AppState>,
    app: AppHandle,
    key: String,
) -> Result<(), String> {
    let engine = state.registry.get("gemini").ok_or("engine missing")?;
    let s = settings::load(&app);
    let api_key = if key.is_empty() {
        settings::get_api_key("gemini")
    } else {
        Some(key)
    };
    let ctx = EngineContext {
        api_key,
        model: Some(s.gemini_model),
    };
    let req = TranslateRequest {
        text: "hi".into(),
        source: "auto".into(),
        target: s.target_lang,
    };
    engine
        .translate(&req, &ctx)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn hotkey_status() -> hotkey::HotkeyStatus {
    hotkey::status()
}

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
async fn check_for_update(app: AppHandle) -> Result<update::UpdateInfo, String> {
    update::check(&app.package_info().version.to_string()).await
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())
    } else {
        autolaunch.disable().map_err(|e| e.to_string())
    }
}

/// macOS 全域鍵盤監聽需要「輔助使用」權限；未授權時觸發系統提示
#[cfg(target_os = "macos")]
fn ensure_accessibility_permission() {
    if !macos_accessibility_client::accessibility::application_is_trusted() {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            registry: EngineRegistry::new(),
        })
        .setup(|app| {
            let handle = app.handle();
            let s = settings::load(handle);
            hotkey::HOTKEY_ENABLED.store(s.hotkey_enabled, Ordering::Relaxed);

            tray::setup(handle)?;

            #[cfg(target_os = "macos")]
            ensure_accessibility_permission();

            hotkey::spawn_listener(handle.clone());
            Ok(())
        })
        .on_window_event(|win, event| {
            // 關閉視窗只隱藏，app 常駐系統匣
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = win.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            translate,
            list_engines,
            get_settings,
            update_settings,
            set_api_key,
            has_api_key,
            validate_gemini_key,
            hotkey_status,
            copy_text,
            get_autostart,
            set_autostart,
            app_version,
            check_for_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
