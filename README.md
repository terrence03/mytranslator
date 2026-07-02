# MyTranslator

跨平台（Windows / macOS / Linux）桌面翻譯工具。在任何應用中選取文字後快速按兩次複製快捷鍵（Windows/Linux: `Ctrl+C+C`，macOS: `Cmd+C+C`），翻譯結果即彈出在游標旁 —— 體驗對標 DeepL 桌面版。

## 功能

- **雙擊複製觸發翻譯**：被動監聽全域鍵盤事件（不攔截、不影響正常複製），400ms 內兩次 Ctrl+C 觸發。Windows/macOS/Linux X11 走 `rdev`，Linux Wayland 走 `evdev`（讀 `/dev/input`）
- **可插拔翻譯引擎**：
  - Google 翻譯（免費網頁端點，無需 API key）
  - Gemini AI（自備 API key，可於 [Google AI Studio](https://aistudio.google.com/apikey) 免費取得）
- API key 儲存於作業系統憑證庫（Windows 憑證管理員 / macOS 鑰匙圈）
- 常駐系統匣、單一實例、可設定開機自啟

## 技術棧

Tauri 2（Rust 後端）+ React 19 + TypeScript + Tailwind CSS v4

```
src/                    # 前端（依 window label 路由）
  windows/popup/        # 翻譯彈窗（無邊框、置頂、跟隨游標）
  windows/settings/     # 設定頁
  lib/                  # invoke 封裝、語言清單
src-tauri/src/
  hotkey/               # 雙擊偵測狀態機（含單元測試）+ rdev / evdev 監聽後端
  engines/              # TranslationEngine trait + google / gemini 實作
  settings.rs           # tauri-plugin-store + keyring
  window.rs             # popup 游標定位（多螢幕 / DPI 夾取）
  tray.rs               # 系統匣選單
```

## 開發

前置需求：Node.js 20+、pnpm、Rust stable。

```bash
pnpm install
pnpm tauri dev      # 開發模式
pnpm tauri build    # 打包（Windows: NSIS；macOS: .app/.dmg；Linux: AppImage）
```

測試：

```bash
cd src-tauri && cargo test        # 引擎解析 + 雙擊偵測狀態機
pnpm build                        # 前端型別檢查 + 建置
```

### 平台注意事項

- **Windows 原生**：需安裝 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（C++ 工作負載）+ [Rust](https://rustup.rs/)，無需其他權限。
- **macOS**：全域鍵盤監聽需要「系統設定 → 隱私權與安全性 → 輔助使用」授權，首次啟動會自動跳出系統提示；授權後需重啟 app。未簽名的打包產物需 `xattr -cr MyTranslator.app` 解除隔離（正式發佈需 Apple Developer 簽名 + 公證）。
- **Linux**：建置依賴：

  ```bash
  sudo apt-get install -y build-essential pkg-config libssl-dev \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev libxdo-dev libx11-dev libxi-dev libxtst-dev libdbus-1-dev
  ```

  | Session | 快捷鍵 | popup 位置 | 額外設定 |
  |---|---|---|---|
  | X11 | ✅ rdev | 跟隨游標 | 無 |
  | Wayland | ✅ evdev | 由合成器擺放（通常置中） | `sudo usermod -aG input $USER` 後重新登入 |

  其他注意事項：API key 存 GNOME Keyring / KWallet（需 secret-service，桌面環境內建）；GNOME 顯示系統匣需安裝 AppIndicator 擴充套件；Wayland 下無權限時設定頁會顯示引導警告。

- **WSL2**：WSLg 可跑 Linux 版做冒煙測試與編譯/單元測試，但 WSLg 鍵盤輸入走 RDP 不經 evdev、也攔不到 Windows 原生按鍵，`Ctrl+C+C` 全流程請在 Windows 原生或實機 Linux 驗證。

## 已知限制

- Google 免費端點為非官方介面，可能被限流（429）；遇到時可切換 Gemini 引擎
- 超過 5000 字元的複製內容不觸發翻譯（避免大檔複製誤觸）
