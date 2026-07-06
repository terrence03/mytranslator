import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  api,
  type AppSettings,
  type EngineInfo,
  type HotkeyStatus,
} from "../../lib/api";
import { applyTheme, type ThemeMode } from "../../lib/theme";
import { TARGET_LANGS } from "../../lib/languages";

const GEMINI_MODEL_SUGGESTIONS = [
  "gemini-3.1-flash-lite",
  "gemini-flash-lite-latest",
  "gemini-3.5-flash",
];

const THEME_OPTIONS: { value: ThemeMode; label: string }[] = [
  { value: "light", label: "淺色" },
  { value: "dark", label: "深色" },
  { value: "system", label: "系統" },
];

const IS_MAC = navigator.userAgent.includes("Mac");
const HOTKEY_LABEL = IS_MAC ? "Cmd+C+C" : "Ctrl+C+C";

type KeyStatus =
  | { kind: "idle" }
  | { kind: "busy"; msg: string }
  | { kind: "ok"; msg: string }
  | { kind: "err"; msg: string };

type UpdateCheckState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "up-to-date" }
  | { kind: "available"; latest: string; url: string }
  | { kind: "err"; msg: string };

export default function Settings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [engines, setEngines] = useState<EngineInfo[]>([]);
  const [autostart, setAutostartState] = useState(false);
  const [geminiKey, setGeminiKey] = useState("");
  const [hasGeminiKey, setHasGeminiKey] = useState(false);
  const [keyStatus, setKeyStatus] = useState<KeyStatus>({ kind: "idle" });
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus | null>(null);
  const [appVersion, setAppVersion] = useState("");
  const [updateCheck, setUpdateCheck] = useState<UpdateCheckState>({
    kind: "idle",
  });

  useEffect(() => {
    api.getSettings().then(setSettings).catch(console.error);
    api.listEngines().then(setEngines).catch(console.error);
    api.hasApiKey("gemini").then(setHasGeminiKey).catch(console.error);
    api.getAutostart().then(setAutostartState).catch(console.error);
    api.hotkeyStatus().then(setHotkeyStatus).catch(console.error);
    api.appVersion().then(setAppVersion).catch(console.error);
  }, []);

  useEffect(() => {
    if (settings) applyTheme(settings.theme);
  }, [settings?.theme]);

  const checkForUpdate = async () => {
    setUpdateCheck({ kind: "checking" });
    try {
      const info = await api.checkForUpdate();
      setUpdateCheck(
        info.hasUpdate
          ? { kind: "available", latest: info.latest, url: info.url }
          : { kind: "up-to-date" },
      );
    } catch (e) {
      setUpdateCheck({ kind: "err", msg: String(e) });
    }
  };

  const update = (patch: Partial<AppSettings>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    api.updateSettings(next).catch(console.error);
  };

  const toggleAutostart = (enabled: boolean) => {
    setAutostartState(enabled);
    api.setAutostart(enabled).catch(console.error);
  };

  const saveGeminiKey = async () => {
    setKeyStatus({ kind: "busy", msg: "儲存中…" });
    try {
      await api.setApiKey("gemini", geminiKey);
      const saved = geminiKey.length > 0;
      setHasGeminiKey(saved);
      setGeminiKey("");
      setKeyStatus({
        kind: "ok",
        msg: saved ? "已儲存到系統憑證庫" : "已清除 API key",
      });
    } catch (e) {
      setKeyStatus({ kind: "err", msg: String(e) });
    }
  };

  const validateGeminiKey = async () => {
    setKeyStatus({ kind: "busy", msg: "驗證中…" });
    try {
      await api.validateGeminiKey(geminiKey);
      setKeyStatus({ kind: "ok", msg: "驗證成功，key 可用 ✓" });
    } catch (e) {
      setKeyStatus({ kind: "err", msg: String(e) });
    }
  };

  if (!settings) return null;

  return (
    <div className="h-full overflow-auto bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <div className="mx-auto max-w-xl space-y-6 px-6 py-6">
        <header>
          <h1 className="text-lg font-semibold">MyTranslator 設定</h1>
          <p className="mt-1 text-sm text-zinc-500 dark:text-zinc-400">
            在任何應用中選取文字後快速按兩次 {HOTKEY_LABEL}
            ，翻譯結果會出現在游標旁。
          </p>
        </header>

        <Section title="引擎與語言">
          <Field label="預設翻譯引擎">
            <select
              value={settings.defaultEngine}
              onChange={(e) => update({ defaultEngine: e.target.value })}
              className="w-52 rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm outline-none focus:border-zinc-500 dark:border-zinc-700 dark:bg-zinc-800"
            >
              {engines.map((e) => (
                <option key={e.id} value={e.id}>
                  {e.name}
                  {e.requiresKey ? "（需 API key）" : ""}
                </option>
              ))}
            </select>
          </Field>
          <Field label="目標語言">
            <select
              value={settings.targetLang}
              onChange={(e) => update({ targetLang: e.target.value })}
              className="w-52 rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm outline-none focus:border-zinc-500 dark:border-zinc-700 dark:bg-zinc-800"
            >
              {TARGET_LANGS.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.name}
                </option>
              ))}
            </select>
          </Field>
        </Section>

        <Section title="快捷鍵">
          <Toggle
            label={`啟用 ${HOTKEY_LABEL} 翻譯`}
            checked={settings.hotkeyEnabled}
            onChange={(v) => update({ hotkeyEnabled: v })}
          />
          {hotkeyStatus && !hotkeyStatus.ok && (
            <div className="rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-xs leading-relaxed text-amber-800 dark:border-amber-600/50 dark:bg-amber-950/40 dark:text-amber-300">
              {hotkeyStatus.message ?? "鍵盤監聽未能啟動。"}
            </div>
          )}
          {IS_MAC && (
            <p className="text-xs text-zinc-500">
              macOS 需要在「系統設定 → 隱私權與安全性 → 輔助使用」中允許
              MyTranslator 監聽鍵盤事件，授權後需重新啟動本程式。
            </p>
          )}
        </Section>

        <Section title="Gemini AI">
          <Field label="API key">
            <div className="flex w-full gap-2">
              <input
                type="password"
                value={geminiKey}
                onChange={(e) => setGeminiKey(e.target.value)}
                placeholder={hasGeminiKey ? "已設定（輸入以更換）" : "AIza…"}
                className="selectable min-w-0 flex-1 rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm outline-none focus:border-zinc-500 dark:border-zinc-700 dark:bg-zinc-800"
              />
              <button
                onClick={saveGeminiKey}
                disabled={keyStatus.kind === "busy" || (!geminiKey && !hasGeminiKey)}
                className="rounded-md border border-zinc-300 bg-zinc-100 px-3 py-1.5 text-sm hover:bg-zinc-200 disabled:opacity-40 dark:border-zinc-700 dark:bg-zinc-800 dark:hover:bg-zinc-700"
              >
                儲存
              </button>
              <button
                onClick={validateGeminiKey}
                disabled={keyStatus.kind === "busy" || (!geminiKey && !hasGeminiKey)}
                className="rounded-md border border-zinc-300 bg-zinc-100 px-3 py-1.5 text-sm hover:bg-zinc-200 disabled:opacity-40 dark:border-zinc-700 dark:bg-zinc-800 dark:hover:bg-zinc-700"
              >
                驗證
              </button>
            </div>
          </Field>
          <Field label="模型">
            <input
              list="gemini-models"
              value={settings.geminiModel}
              onChange={(e) => update({ geminiModel: e.target.value })}
              className="selectable w-52 rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm outline-none focus:border-zinc-500 dark:border-zinc-700 dark:bg-zinc-800"
            />
            <datalist id="gemini-models">
              {GEMINI_MODEL_SUGGESTIONS.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
          </Field>
          <p className="text-xs text-zinc-500">
            建議使用 gemini-3.1-flash-lite：回應最快（約 1
            秒內）、免費層可用；推理模型（gemma-4、gemini-3.5-flash）翻譯前會先思考，速度慢數倍。
          </p>
          {keyStatus.kind !== "idle" && (
            <p
              className={`text-xs ${
                keyStatus.kind === "err"
                  ? "text-red-600 dark:text-red-400"
                  : keyStatus.kind === "ok"
                    ? "text-emerald-600 dark:text-emerald-400"
                    : "text-zinc-500 dark:text-zinc-400"
              }`}
            >
              {keyStatus.msg}
            </p>
          )}
          <p className="text-xs text-zinc-500">
            API key 儲存在作業系統憑證庫（Windows 憑證管理員 / macOS
            鑰匙圈），不會以明文寫入磁碟。可到 Google AI Studio 免費取得。
          </p>
        </Section>

        <Section title="外觀">
          <Field label="主題">
            <div className="flex overflow-hidden rounded-md border border-zinc-300 dark:border-zinc-700">
              {THEME_OPTIONS.map(({ value, label }) => (
                <button
                  key={value}
                  onClick={() => update({ theme: value })}
                  className={`px-3 py-1.5 text-sm ${
                    settings.theme === value
                      ? "bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900"
                      : "bg-white text-zinc-600 hover:bg-zinc-100 dark:bg-zinc-800 dark:text-zinc-300 dark:hover:bg-zinc-700"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </Field>
        </Section>

        <Section title="系統">
          <Toggle label="開機時自動啟動" checked={autostart} onChange={toggleAutostart} />
          <p className="text-xs text-zinc-500">
            關閉此視窗不會結束程式，可從系統匣圖示重新開啟設定或結束。
          </p>
        </Section>

        <Section title="關於">
          <Field label="目前版本">
            <span className="text-sm text-zinc-700 dark:text-zinc-300">
              {appVersion ? `v${appVersion}` : "…"}
            </span>
          </Field>
          <div className="flex items-center justify-between gap-4">
            <button
              onClick={checkForUpdate}
              disabled={updateCheck.kind === "checking"}
              className="rounded-md border border-zinc-300 bg-zinc-100 px-3 py-1.5 text-sm hover:bg-zinc-200 disabled:opacity-40 dark:border-zinc-700 dark:bg-zinc-800 dark:hover:bg-zinc-700"
            >
              {updateCheck.kind === "checking" ? "檢查中…" : "檢查更新"}
            </button>
            {updateCheck.kind === "up-to-date" && (
              <span className="text-xs text-emerald-600 dark:text-emerald-400">
                已是最新版本 ✓
              </span>
            )}
            {updateCheck.kind === "err" && (
              <span className="text-xs text-red-600 dark:text-red-400">
                {updateCheck.msg}
              </span>
            )}
          </div>
          {updateCheck.kind === "available" && (
            <div className="flex items-center justify-between gap-4 rounded-md border border-emerald-300 bg-emerald-50 px-3 py-2 text-xs text-emerald-800 dark:border-emerald-600/50 dark:bg-emerald-950/40 dark:text-emerald-300">
              <span>發現新版本 v{updateCheck.latest}</span>
              <button
                onClick={() => openUrl(updateCheck.url)}
                className="shrink-0 rounded-md border border-emerald-300 px-2 py-1 hover:bg-emerald-100 dark:border-emerald-600/60 dark:hover:bg-emerald-900/40"
              >
                前往下載
              </button>
            </div>
          )}
        </Section>
      </div>
    </div>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-3 rounded-xl border border-zinc-200 bg-white p-4 dark:border-zinc-800 dark:bg-zinc-900">
      <h2 className="text-sm font-medium text-zinc-600 dark:text-zinc-300">
        {title}
      </h2>
      {children}
    </section>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="shrink-0 text-sm text-zinc-500 dark:text-zinc-400">
        {label}
      </span>
      {children}
    </div>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center justify-between gap-4">
      <span className="text-sm text-zinc-500 dark:text-zinc-400">{label}</span>
      <button
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative h-5 w-9 shrink-0 rounded-full transition-colors ${
          checked ? "bg-emerald-500" : "bg-zinc-300 dark:bg-zinc-700"
        }`}
      >
        <span
          className={`absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all ${
            checked ? "left-4.5" : "left-0.5"
          }`}
        />
      </button>
    </label>
  );
}
