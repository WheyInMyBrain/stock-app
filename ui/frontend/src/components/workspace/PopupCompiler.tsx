// stock-app/ui/frontend/src/components/workspace/PopupCompiler.tsx

import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import PrimitiveCompiler from "./PrimitiveCompiler"; 
import { usePipelineListener } from "./usePipelineListener"; 
export default function PopupCompiler() {
  const [layoutData, setLayoutData] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);

  // 🎯 State constants to feed the pipeline listener stably
  const [activeModule, setActiveModule] = useState<string>("");
  const [activeTicker, setActiveTicker] = useState<string>("");

  const popupColors = {
    border: "border-neutral-800/80",
    text: "text-neutral-200",
    textMuted: "text-neutral-500",
    inputBg: "bg-[#121212]"
  };
  const popupCardBg = "bg-[#0F0F0F]";

  // 🚀 RE-USABLE REFRESH ENGINE: Re-compiled payload fetches straight from the backend
  const handleLiveTelemetryRefresh = useCallback(async () => {
    if (!activeModule || !activeTicker) return;
    try {
      const freshTelemetry = await invoke("compile_popup_telemetry", { 
        moduleId: activeModule, 
        ticker: activeTicker 
      });
      if (freshTelemetry) {
        setLayoutData(freshTelemetry);
      }
    } catch (err) {
      console.error(`❌ [POPUP REFRESH FAULT]: Refetch failed for module [${activeModule}]:`, err);
    }
  }, [activeModule, activeTicker]);

  // Initial routing mounting pass execution logic
  useEffect(() => {
    try {
      const queryString = window.location.hash.split("?")[1];
      if (!queryString) throw new Error("Missing parameters matrix.");

      const params = new URLSearchParams(queryString);
      const moduleId = params.get("module");
      const ticker = params.get("ticker");

      if (!moduleId || !ticker) throw new Error("Malformed query identifiers.");

      // Store them stably inside states
      setActiveModule(moduleId);
      setActiveTicker(ticker);

      invoke("compile_popup_telemetry", { moduleId, ticker })
        .then((response) => setLayoutData(response))
        .catch((err) => setError(String(err)));
        
    } catch (err: any) {
      setError(err.message || "Unknown Routing Failure");
    }
  }, []);

  // 🚀 🎯 PLUG IN YOUR CUSTOM ABSTRACT PIPELINE HOOK
  // Watches the global window webview event pipe and hits the callback trigger automatically
  usePipelineListener(
    activeModule,
    activeTicker,
    handleLiveTelemetryRefresh
  );

  if (error) return <div className="p-6 text-red-400 font-mono text-xs">❌ {error}</div>;
  if (!layoutData) return <div className="p-6 text-neutral-500 font-mono text-xs animate-pulse">Loading Viewport Matrices...</div>;

  return (
    <div className="w-full h-screen bg-[#0a0a0a] text-white p-6 flex flex-col overflow-x-hidden overflow-y-auto font-sans">
      <PrimitiveCompiler 
        node={layoutData} 
        colors={popupColors} 
        cardBg={popupCardBg} 
      />
    </div>
  );
}