import { useEffect, useState } from "react";
import {
  api,
  type AppSettings,
  type EngineInfo,
  type HotkeyStatus,
} from "../../lib/api";
import { TARGET_LANGS } from "../../lib/languages";

const GEMINI_MODEL_SUGGESTIONS = [
  "gemini-3.1-flash-lite",
  "gemini-flash-lite-latest",
  "gemini-3.5-flash",
];

const IS_MAC = navigator.userAgent.includes("Mac");
const HOTKEY_LABEL = IS_MAC ? "Cmd+C+C" : "Ctrl+C+C";

type KeyStatus =
  | { kind: "idle" }
  | { kind: "busy"; msg: string }
  | { kind: "ok"; msg: string }
  | { kind: "err"; msg: string };

export default function Settings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [engines, setEngines] = useState<EngineInfo[]>([]);
  const [autostart, setAutostartState] = useState(false);
  const [geminiKey, setGeminiKey] = useState("");
  const [hasGeminiKey, setHasGeminiKey] = useState(false);
  const [keyStatus, setKeyStatus] = useState<KeyStatus>({ kind: "idle" });
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings).catch(console.error);
    api.listEngines().then(setEngines).catch(console.error);
    api.hasApiKey("gemini").then(setHasGeminiKey).catch(console.error);
    api.getAutostart().then(setAutostartState).catch(console.error);
    api.hotkeyStatus().then(setHotkeyStatus).catch(console.error);
  }, []);

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
    <div className="h-full overflow-auto bg-zinc-950 text-zinc-100">
      <div className="mx-auto max-w-xl space-y-6 px-6 py-6">
        <header>
          <h1 className="text-lg font-semibold">MyTranslator 設定</h1>
          <p className="mt-1 text-sm text-zinc-400">
            在任何應用中選取文字後快速按兩次 {HOTKEY_LABEL}
            ，翻譯結果會出現在游標旁。
          </p>
        </header>

        <Section title="引擎與語言">
          <Field label="預設翻譯引擎">
            <select
              value={settings.defaultEngine}
              onChange={(e) => update({ defaultEngine: e.target.value })}
              className="w-52 rounded-md border border-zinc-700 bg-zinc-800 px-2 py-1.5 text-sm outline-none focus:border-zinc-500"
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
              className="w-52 rounded-md border border-zinc-700 bg-zinc-800 px-2 py-1.5 text-sm outline-none focus:border-zinc-500"
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
            <div className="rounded-md border border-amber-600/50 bg-amber-950/40 px-3 py-2 text-xs leading-relaxed text-amber-300">
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
                className="selectable min-w-0 flex-1 rounded-md border border-zinc-700 bg-zinc-800 px-2 py-1.5 text-sm outline-none focus:border-zinc-500"
              />
              <button
                onClick={saveGeminiKey}
                disabled={keyStatus.kind === "busy" || (!geminiKey && !hasGeminiKey)}
                className="rounded-md border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm hover:bg-zinc-700 disabled:opacity-40"
              >
                儲存
              </button>
              <button
                onClick={validateGeminiKey}
                disabled={keyStatus.kind === "busy" || (!geminiKey && !hasGeminiKey)}
                className="rounded-md border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm hover:bg-zinc-700 disabled:opacity-40"
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
              className="selectable w-52 rounded-md border border-zinc-700 bg-zinc-800 px-2 py-1.5 text-sm outline-none focus:border-zinc-500"
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
                  ? "text-red-400"
                  : keyStatus.kind === "ok"
                    ? "text-emerald-400"
                    : "text-zinc-400"
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

        <Section title="系統">
          <Toggle label="開機時自動啟動" checked={autostart} onChange={toggleAutostart} />
          <p className="text-xs text-zinc-500">
            關閉此視窗不會結束程式，可從系統匣圖示重新開啟設定或結束。
          </p>
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
    <section className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-900 p-4">
      <h2 className="text-sm font-medium text-zinc-300">{title}</h2>
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
      <span className="shrink-0 text-sm text-zinc-400">{label}</span>
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
      <span className="text-sm text-zinc-400">{label}</span>
      <button
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative h-5 w-9 shrink-0 rounded-full transition-colors ${
          checked ? "bg-emerald-500" : "bg-zinc-700"
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
