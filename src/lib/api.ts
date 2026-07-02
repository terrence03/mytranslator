import { invoke } from "@tauri-apps/api/core";

export interface TranslateResponse {
  text: string;
  detectedSource: string | null;
  engine: string;
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
}

export interface HotkeyStatus {
  backend: string;
  ok: boolean;
  message: string | null;
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
  validateGeminiKey: (key: string) =>
    invoke<void>("validate_gemini_key", { key }),
  hotkeyStatus: () => invoke<HotkeyStatus>("hotkey_status"),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
};
