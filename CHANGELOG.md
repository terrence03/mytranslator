# 更新日誌

本檔案記錄 MyTranslator 各版本的重大變更。格式參考
[Keep a Changelog](https://keepachangelog.com/zh-TW/1.1.0/)，版本號遵循
[語意化版本 (SemVer)](https://semver.org/lang/zh-TW/)。

## [Unreleased]

### 新增

- 新增「Google Cloud 翻譯」引擎（官方 Cloud Translation API），比免費的 Google
  翻譯端點穩定，每月前 50 萬字元免費；可在設定頁填入並驗證 API key。
- 設定頁顯示 Google Cloud 翻譯（本月字元數）與 Gemini（今日請求數、本月
  tokens）的本機估算用量，並可設定上限：達到上限時擋下請求，達 80% 時翻譯彈窗
  顯示提醒。Google Cloud 翻譯預設上限為免費額度 500,000 字元／月。

### 移除

- 移除非官方的 Google 翻譯免費網頁端點（常被限流或斷線）。原本預設使用它的
  設定會自動改為「Google Cloud 翻譯」引擎，需先在設定頁填入 API key。

## [0.1.2] - 2026-07-06

### 新增

- 設定頁新增「關於」區塊：顯示目前版本，並可手動檢查更新（比對 GitHub 最新
  Release，發現新版時提供下載頁連結）。
- 設定頁新增外觀模式選項，可選擇淺色／深色／系統，並即時套用到主視窗與翻譯彈窗。

## [0.1.1] - 2026-07-05

### 新增

- 翻譯彈窗新增可拖曳的分隔線，可調整原文欄高度。

### 修正

- Gemini 引擎改善網路穩定性，並將預設模型切換為 gemini-3.1-flash-lite。

## [0.1.0] - 2026-07-03

### 新增

- 首次發佈：以 Tauri + React 打造的桌面翻譯工具，支援全域 Ctrl+C+C／Cmd+C+C
  快捷鍵快速翻譯選取文字。
