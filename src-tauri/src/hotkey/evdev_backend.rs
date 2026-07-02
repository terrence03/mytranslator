//! Wayland session 的鍵盤監聽後端。
//!
//! Wayland 不開放全域鍵盤事件給一般 client，X11 的 XRecord（rdev）只能看到
//! XWayland 應用的按鍵，因此直接讀 kernel 輸入層 /dev/input/event*。
//! 需要使用者屬於 `input` 群組才有讀取權限。

use evdev::{Device, EventSummary, KeyCode};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Instant;
use tauri::AppHandle;

use super::{handle_trigger, set_status, DoubleCopyDetector, KeyInput, HOTKEY_ENABLED};

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || run(app));
}

fn run(app: AppHandle) {
    let keyboards: Vec<Device> = evdev::enumerate()
        .map(|(_, dev)| dev)
        .filter(is_keyboard)
        .collect();

    if keyboards.is_empty() {
        eprintln!("[hotkey] evdev backend: no readable keyboard in /dev/input");
        set_status(
            "evdev",
            false,
            Some(
                "無法讀取鍵盤裝置（/dev/input），Wayland 下快捷鍵無法運作。\
                 請執行 sudo usermod -aG input $USER，重新登入後再啟動本程式。"
                    .to_string(),
            ),
        );
        return;
    }
    eprintln!(
        "[hotkey] evdev backend: listening on {} keyboard device(s)",
        keyboards.len()
    );
    set_status("evdev", true, None);

    let (tx, rx) = mpsc::channel::<(KeyInput, Instant)>();
    for mut dev in keyboards {
        let tx = tx.clone();
        std::thread::spawn(move || loop {
            // fetch_events 阻塞等待；純讀取，不攔截也不影響按鍵送達其他應用
            match dev.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        if let Some(input) = map_event(&ev) {
                            if tx.send((input, Instant::now())).is_err() {
                                return;
                            }
                        }
                    }
                }
                // 裝置拔除或讀取失敗就結束此執行緒；重新插入需重啟 app
                Err(_) => return,
            }
        });
    }
    drop(tx);

    let mut detector = DoubleCopyDetector::new();
    for (input, at) in rx {
        if !HOTKEY_ENABLED.load(Ordering::Relaxed) {
            continue;
        }
        if detector.on_input(input, at) {
            let app = app.clone();
            std::thread::spawn(move || handle_trigger(app));
        }
    }
}

/// 只挑真正的鍵盤：支援 EV_KEY 且同時有 C 鍵與左 Ctrl（排除滑鼠、電源鍵等）
fn is_keyboard(dev: &Device) -> bool {
    dev.supported_keys()
        .is_some_and(|keys| keys.contains(KeyCode::KEY_C) && keys.contains(KeyCode::KEY_LEFTCTRL))
}

fn map_event(ev: &evdev::InputEvent) -> Option<KeyInput> {
    // value: 1 = 按下, 0 = 放開, 2 = 長按重複（忽略）
    match ev.destructure() {
        EventSummary::Key(_, code, value) => match (code, value) {
            (KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL, 1) => Some(KeyInput::ModifierDown),
            (KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL, 0) => Some(KeyInput::ModifierUp),
            (KeyCode::KEY_C, 1) => Some(KeyInput::Copy),
            (_, 1) => Some(KeyInput::Other),
            _ => None,
        },
        _ => None,
    }
}
