export interface Language {
  code: string;
  name: string;
}

export const TARGET_LANGS: Language[] = [
  { code: "zh-TW", name: "繁體中文" },
  { code: "zh-CN", name: "简体中文" },
  { code: "en", name: "English" },
  { code: "ja", name: "日本語" },
  { code: "ko", name: "한국어" },
  { code: "fr", name: "Français" },
  { code: "de", name: "Deutsch" },
  { code: "es", name: "Español" },
];

const DETECTED_NAMES: Record<string, string> = Object.fromEntries(
  TARGET_LANGS.map((l) => [l.code, l.name]),
);

export function langName(code: string | null): string {
  if (!code) return "自動偵測";
  return DETECTED_NAMES[code] ?? code;
}
