import { getCurrentWindow } from "@tauri-apps/api/window";

export type ThemeMode = "light" | "dark" | "system";

const media = window.matchMedia("(prefers-color-scheme: dark)");
let mode: ThemeMode = "system";
let listening = false;

function resolve(): "light" | "dark" {
  return mode === "system" ? (media.matches ? "dark" : "light") : mode;
}

function sync() {
  document.documentElement.classList.toggle("dark", resolve() === "dark");
  void getCurrentWindow().setTheme(mode === "system" ? null : mode);
}

/** 依設定切換 .dark class；"system" 時額外跟隨系統偏好變化即時更新 */
export function applyTheme(next: ThemeMode) {
  mode = next;
  sync();
  if (!listening) {
    listening = true;
    media.addEventListener("change", () => {
      if (mode === "system") sync();
    });
  }
}
