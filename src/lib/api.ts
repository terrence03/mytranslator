import { invoke } from "@tauri-apps/api/core";
import type { ThemeMode } from "./theme";

export interface TranslateResponse {
  text: string;
  detectedSource: string | null;
  engine: string;
  usageWarning: string | null;
}

/** 本機估算的用量；週期以太平洋時間切分，與 Google 一致 */
export interface EngineUsage {
  month: string;
  monthChars: number;
  monthTokens: number;
  day: string;
  dayRequests: number;
}

export interface EngineInfo {
  id: string;
  name: string;
  requiresKey: boolean;
}

export interface AppSettings {
  defaultEngine: string;
  targetLang: string;
  hotkeyEnabled: boolean;
  geminiModel: string;
  theme: ThemeMode;
  /** 0 表示不限制 */
  googleCloudMonthlyCharLimit: number;
  /** 0 表示不限制 */
  geminiDailyRequestLimit: number;
}

export interface HotkeyStatus {
  backend: string;
  ok: boolean;
  message: string | null;
}

export interface UpdateInfo {
  current: string;
  latest: string;
  hasUpdate: boolean;
  url: string;
}

export const api = {
  translate: (engineId: string, text: string, target: string) =>
    invoke<TranslateResponse>("translate", { engineId, text, target }),
  listEngines: () => invoke<EngineInfo[]>("list_engines"),
  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: (settings: AppSettings) =>
    invoke<void>("update_settings", { settings }),
  setApiKey: (engineId: string, key: string) =>
    invoke<void>("set_api_key", { engineId, key }),
  hasApiKey: (engineId: string) => invoke<boolean>("has_api_key", { engineId }),
  validateApiKey: (engineId: string, key: string) =>
    invoke<void>("validate_api_key", { engineId, key }),
  getUsage: (engineId: string) =>
    invoke<EngineUsage>("get_usage", { engineId }),
  resetUsage: (engineId: string) => invoke<void>("reset_usage", { engineId }),
  hotkeyStatus: () => invoke<HotkeyStatus>("hotkey_status"),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  appVersion: () => invoke<string>("app_version"),
  checkForUpdate: () => invoke<UpdateInfo>("check_for_update"),
};
