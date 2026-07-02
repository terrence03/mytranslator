import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, type EngineInfo } from "../../lib/api";
import { langName } from "../../lib/languages";

export default function Popup() {
  const [engines, setEngines] = useState<EngineInfo[]>([]);
  const [engineId, setEngineId] = useState("google");
  const [target, setTarget] = useState("zh-TW");
  const [sourceText, setSourceText] = useState("");
  const [result, setResult] = useState("");
  const [detected, setDetected] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  // 只顯示最後一次請求的結果，避免慢的舊請求蓋掉新結果
  const requestSeq = useRef(0);

  const runTranslate = useCallback(
    async (text: string, engine: string, targetLang: string) => {
      const seq = ++requestSeq.current;
      setLoading(true);
      setError(null);
      setCopied(false);
      try {
        const res = await api.translate(engine, text, targetLang);
        if (seq !== requestSeq.current) return;
        setResult(res.text);
        setDetected(res.detectedSource);
      } catch (e) {
        if (seq !== requestSeq.current) return;
        setResult("");
        setError(String(e));
      } finally {
        if (seq === requestSeq.current) setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    api.listEngines().then(setEngines).catch(console.error);

    const unlisten = listen<string>("translate-request", async (event) => {
      const text = event.payload;
      setSourceText(text);
      // 每次觸發都重讀設定，讓設定頁的變更即時生效
      let engine = engineId;
      let targetLang = target;
      try {
        const s = await api.getSettings();
        engine = s.defaultEngine;
        targetLang = s.targetLang;
        setEngineId(engine);
        setTarget(targetLang);
      } catch (e) {
        console.error(e);
      }
      runTranslate(text, engine, targetLang);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (!focused) void win.hide();
    });
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void win.hide();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  const switchEngine = (id: string) => {
    setEngineId(id);
    if (sourceText) runTranslate(sourceText, id, target);
  };

  const copyResult = async () => {
    if (!result) return;
    await api.copyText(result);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 text-zinc-100">
      <header
        data-tauri-drag-region
        className="flex items-center gap-2 border-b border-zinc-800 px-3 py-2"
      >
        <span data-tauri-drag-region className="text-xs text-zinc-400">
          {langName(detected)} → {langName(target)}
        </span>
        <select
          value={engineId}
          onChange={(e) => switchEngine(e.target.value)}
          className="ml-auto rounded-md border border-zinc-700 bg-zinc-800 px-2 py-0.5 text-xs outline-none focus:border-zinc-500"
        >
          {engines.map((e) => (
            <option key={e.id} value={e.id}>
              {e.name}
            </option>
          ))}
        </select>
        <button
          onClick={copyResult}
          disabled={!result || loading}
          className="rounded-md border border-zinc-700 bg-zinc-800 px-2 py-0.5 text-xs hover:bg-zinc-700 disabled:opacity-40"
        >
          {copied ? "已複製 ✓" : "複製"}
        </button>
        <button
          onClick={() => void getCurrentWindow().hide()}
          className="rounded-md px-1.5 py-0.5 text-xs text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
          aria-label="關閉"
        >
          ✕
        </button>
      </header>

      <div className="selectable flex-1 overflow-auto whitespace-pre-wrap px-3 py-2 text-sm leading-relaxed">
        {loading ? (
          <div className="animate-pulse space-y-2 pt-1">
            <div className="h-3 w-4/5 rounded bg-zinc-700" />
            <div className="h-3 w-3/5 rounded bg-zinc-700" />
          </div>
        ) : error ? (
          <p className="text-red-400">{error}</p>
        ) : (
          result
        )}
      </div>

      <footer className="truncate border-t border-zinc-800 px-3 py-1.5 text-xs text-zinc-500">
        {sourceText}
      </footer>
    </div>
  );
}
