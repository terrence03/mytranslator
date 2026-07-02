use std::sync::atomic::Ordering;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::hotkey::HOTKEY_ENABLED;
use crate::{settings, window};

/// 讓設定頁能同步系統匣勾選狀態
pub struct TrayState {
    pub hotkey_item: CheckMenuItem<tauri::Wry>,
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let hotkey_enabled = HOTKEY_ENABLED.load(Ordering::Relaxed);

    let open_settings =
        MenuItem::with_id(app, "open-settings", "開啟設定", true, None::<&str>)?;
    let hotkey_item = CheckMenuItem::with_id(
        app,
        "toggle-hotkey",
        "啟用複製兩次翻譯",
        true,
        hotkey_enabled,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "結束", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open_settings,
            &hotkey_item,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    app.manage(TrayState {
        hotkey_item: hotkey_item.clone(),
    });

    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().expect("app icon missing").clone())
        .tooltip("MyTranslator")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open-settings" => window::show_main_window(app),
            "toggle-hotkey" => {
                let enabled = !HOTKEY_ENABLED.load(Ordering::Relaxed);
                HOTKEY_ENABLED.store(enabled, Ordering::Relaxed);
                let mut s = settings::load(app);
                s.hotkey_enabled = enabled;
                let _ = settings::save(app, &s);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// 設定頁改了快捷鍵開關時，同步系統匣勾選
pub fn sync_hotkey_checked(app: &AppHandle, enabled: bool) {
    if let Some(state) = app.try_state::<TrayState>() {
        let _ = state.hotkey_item.set_checked(enabled);
    }
}
