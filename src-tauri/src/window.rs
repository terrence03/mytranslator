use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

pub const POPUP_LABEL: &str = "popup";
pub const MAIN_LABEL: &str = "main";

/// 游標與 popup 左上角的間距（logical px）
const CURSOR_OFFSET: f64 = 14.0;
/// 與螢幕邊緣保留的間距（logical px）
const SCREEN_MARGIN: f64 = 8.0;

/// 移動 popup 到滑鼠游標旁（含多螢幕與 DPI 縮放處理、邊緣夾取），
/// 顯示視窗並把待翻譯文字用事件送給前端。
pub fn show_popup_with_text(app: &AppHandle, text: String) {
    let Some(win) = app.get_webview_window(POPUP_LABEL) else {
        return;
    };

    // Wayland 不允許 client 指定全域視窗座標，游標定位整段跳過，交給合成器擺放
    #[cfg(target_os = "linux")]
    let position_at_cursor = !crate::hotkey::is_wayland();
    #[cfg(not(target_os = "linux"))]
    let position_at_cursor = true;

    if let Some(cursor) = app.cursor_position().ok().filter(|_| position_at_cursor) {
        let monitor = app
            .monitor_from_point(cursor.x, cursor.y)
            .ok()
            .flatten()
            .or_else(|| app.primary_monitor().ok().flatten());

        let popup_size = win.outer_size().ok();

        let (mut x, mut y) = (cursor.x, cursor.y);
        if let (Some(mon), Some(size)) = (monitor, popup_size) {
            let scale = mon.scale_factor();
            let offset = CURSOR_OFFSET * scale;
            let margin = SCREEN_MARGIN * scale;
            let (w, h) = (size.width as f64, size.height as f64);
            let mon_pos = mon.position();
            let mon_size = mon.size();
            let (min_x, min_y) = (mon_pos.x as f64 + margin, mon_pos.y as f64 + margin);
            let max_x = mon_pos.x as f64 + mon_size.width as f64 - w - margin;
            let max_y = mon_pos.y as f64 + mon_size.height as f64 - h - margin;

            x = (cursor.x + offset).clamp(min_x, max_x.max(min_x));
            y = (cursor.y + offset).clamp(min_y, max_y.max(min_y));
            // 下方放不下就翻到游標上方，避免遮住剛選取的文字
            if cursor.y + offset > max_y {
                y = (cursor.y - offset - h).clamp(min_y, max_y.max(min_y));
            }
        }

        let _ = win.set_position(PhysicalPosition::new(x, y));
    }

    let _ = win.show();
    let _ = win.set_focus();
    let _ = app.emit_to(POPUP_LABEL, "translate-request", &text);
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}
