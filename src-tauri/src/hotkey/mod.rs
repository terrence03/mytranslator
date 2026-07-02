#[cfg(target_os = "linux")]
mod evdev_backend;

use rdev::{listen, Event, EventType, Key};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// 全域開關（設定頁 / 系統匣共用）。監聽執行緒無法重啟，用旗標控制。
pub static HOTKEY_ENABLED: AtomicBool = AtomicBool::new(true);

/// 兩次 Ctrl+C 之間的最大間隔
const DOUBLE_PRESS_WINDOW: Duration = Duration::from_millis(400);
/// 第二次複製後等 OS 寫入剪貼簿的時間
const CLIPBOARD_SETTLE: Duration = Duration::from_millis(150);
/// 超過此長度不觸發翻譯，避免誤觸大檔複製
const MAX_TEXT_LEN: usize = 5000;

/// 後端無關的鍵盤輸入，rdev / evdev 各自轉換成這個型別餵給偵測器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInput {
    ModifierDown,
    ModifierUp,
    Copy,
    Other,
}

/// 監聽後端狀態，供設定頁顯示警告與引導
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatus {
    pub backend: &'static str,
    pub ok: bool,
    pub message: Option<String>,
}

static STATUS: Mutex<HotkeyStatus> = Mutex::new(HotkeyStatus {
    backend: "rdev",
    ok: true,
    message: None,
});

pub fn set_status(backend: &'static str, ok: bool, message: Option<String>) {
    *STATUS.lock().unwrap() = HotkeyStatus {
        backend,
        ok,
        message,
    };
}

pub fn status() -> HotkeyStatus {
    STATUS.lock().unwrap().clone()
}

/// Wayland 下 X11 監聽與視窗定位都不可用，須走降級路徑
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland")
}

/// 雙擊「複製快捷鍵」偵測狀態機。
/// 兩次 Copy 之間夾雜其他按鍵會重置，避免一般連續操作誤觸。
pub struct DoubleCopyDetector {
    modifier_down: bool,
    last_copy: Option<Instant>,
}

impl DoubleCopyDetector {
    pub fn new() -> Self {
        Self {
            modifier_down: false,
            last_copy: None,
        }
    }

    /// 回傳 true 表示偵測到雙擊複製，應觸發翻譯
    pub fn on_input(&mut self, input: KeyInput, now: Instant) -> bool {
        match input {
            KeyInput::ModifierDown => {
                self.modifier_down = true;
                false
            }
            KeyInput::ModifierUp => {
                self.modifier_down = false;
                false
            }
            KeyInput::Copy if self.modifier_down => {
                let triggered = self
                    .last_copy
                    .is_some_and(|prev| now.duration_since(prev) <= DOUBLE_PRESS_WINDOW);
                self.last_copy = if triggered { None } else { Some(now) };
                triggered
            }
            KeyInput::Copy | KeyInput::Other => {
                self.last_copy = None;
                false
            }
        }
    }
}

fn is_copy_modifier(key: Key) -> bool {
    #[cfg(target_os = "macos")]
    return matches!(key, Key::MetaLeft | Key::MetaRight);
    #[cfg(not(target_os = "macos"))]
    return matches!(key, Key::ControlLeft | Key::ControlRight);
}

fn map_rdev(event: &EventType) -> Option<KeyInput> {
    match event {
        EventType::KeyPress(k) if is_copy_modifier(*k) => Some(KeyInput::ModifierDown),
        EventType::KeyRelease(k) if is_copy_modifier(*k) => Some(KeyInput::ModifierUp),
        EventType::KeyPress(Key::KeyC) => Some(KeyInput::Copy),
        EventType::KeyPress(_) => Some(KeyInput::Other),
        _ => None,
    }
}

pub fn spawn_listener(app: AppHandle) {
    #[cfg(target_os = "linux")]
    if is_wayland() {
        evdev_backend::spawn(app);
        return;
    }
    spawn_rdev_listener(app);
}

fn spawn_rdev_listener(app: AppHandle) {
    set_status("rdev", true, None);
    std::thread::spawn(move || {
        let mut detector = DoubleCopyDetector::new();
        let callback = move |event: Event| {
            if !HOTKEY_ENABLED.load(Ordering::Relaxed) {
                return;
            }
            let Some(input) = map_rdev(&event.event_type) else {
                return;
            };
            if detector.on_input(input, Instant::now()) {
                let app = app.clone();
                std::thread::spawn(move || handle_trigger(app));
            }
        };
        // listen() 阻塞當前執行緒；被動監聽不攔截事件，Ctrl+C 複製行為不受影響
        if let Err(e) = listen(callback) {
            eprintln!("[hotkey] keyboard listener failed: {e:?}");
            #[cfg(target_os = "macos")]
            let message = "鍵盤監聽啟動失敗。請在「系統設定 → 隱私權與安全性 → 輔助使用」\
                           允許 MyTranslator 後重新啟動程式。"
                .to_string();
            #[cfg(not(target_os = "macos"))]
            let message = format!("鍵盤監聽啟動失敗：{e:?}");
            set_status("rdev", false, Some(message));
        }
    });
}

pub(crate) fn handle_trigger(app: AppHandle) {
    std::thread::sleep(CLIPBOARD_SETTLE);

    let text = match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[hotkey] clipboard read failed: {e}");
            return;
        }
    };

    let text = text.trim();
    if text.is_empty() || text.chars().count() > MAX_TEXT_LEN {
        return;
    }

    crate::window::show_popup_with_text(&app, text.to_string());
}

#[cfg(test)]
mod tests {
    use super::KeyInput::{Copy, ModifierDown, ModifierUp, Other};
    use super::*;

    #[test]
    fn triggers_on_double_copy_within_window() {
        let mut d = DoubleCopyDetector::new();
        let t0 = Instant::now();
        assert!(!d.on_input(ModifierDown, t0));
        assert!(!d.on_input(Copy, t0));
        assert!(d.on_input(Copy, t0 + Duration::from_millis(300)));
    }

    #[test]
    fn does_not_trigger_when_too_slow() {
        let mut d = DoubleCopyDetector::new();
        let t0 = Instant::now();
        d.on_input(ModifierDown, t0);
        assert!(!d.on_input(Copy, t0));
        assert!(!d.on_input(Copy, t0 + Duration::from_millis(600)));
        // 但這第二次按下成為新的起點，再快速按一次就會觸發
        assert!(d.on_input(Copy, t0 + Duration::from_millis(700)));
    }

    #[test]
    fn does_not_trigger_without_modifier() {
        let mut d = DoubleCopyDetector::new();
        let t0 = Instant::now();
        assert!(!d.on_input(Copy, t0));
        assert!(!d.on_input(Copy, t0));
    }

    #[test]
    fn other_key_between_copies_resets() {
        let mut d = DoubleCopyDetector::new();
        let t0 = Instant::now();
        d.on_input(ModifierDown, t0);
        assert!(!d.on_input(Copy, t0));
        assert!(!d.on_input(Other, t0));
        assert!(!d.on_input(Copy, t0 + Duration::from_millis(100)));
    }

    #[test]
    fn releasing_modifier_between_copies_still_triggers() {
        // 使用者可能按 Ctrl+C、放開、再快速按一次 Ctrl+C
        let mut d = DoubleCopyDetector::new();
        let t0 = Instant::now();
        d.on_input(ModifierDown, t0);
        assert!(!d.on_input(Copy, t0));
        d.on_input(ModifierUp, t0);
        d.on_input(ModifierDown, t0 + Duration::from_millis(150));
        assert!(d.on_input(Copy, t0 + Duration::from_millis(200)));
    }

    #[test]
    fn third_copy_does_not_retrigger_immediately() {
        let mut d = DoubleCopyDetector::new();
        let t0 = Instant::now();
        d.on_input(ModifierDown, t0);
        d.on_input(Copy, t0);
        assert!(d.on_input(Copy, t0 + Duration::from_millis(100)));
        // 觸發後重置，第三下只是新序列的第一下
        assert!(!d.on_input(Copy, t0 + Duration::from_millis(200)));
    }

    #[test]
    fn copy_without_modifier_resets_sequence() {
        // Ctrl+C 後放開 Ctrl 再單獨按 C（打字），不應累積成雙擊
        let mut d = DoubleCopyDetector::new();
        let t0 = Instant::now();
        d.on_input(ModifierDown, t0);
        assert!(!d.on_input(Copy, t0));
        d.on_input(ModifierUp, t0);
        assert!(!d.on_input(Copy, t0 + Duration::from_millis(100)));
        d.on_input(ModifierDown, t0 + Duration::from_millis(150));
        assert!(!d.on_input(Copy, t0 + Duration::from_millis(200)));
    }
}
