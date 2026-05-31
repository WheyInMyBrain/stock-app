// stock-app/ui/frontend/src/components/workspace/PopupCompiler.tsx

import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import PrimitiveCompiler from "./PrimitiveCompiler"; 

export default function PopupCompiler() {
  const [layoutData, setLayoutData] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);

  const popupColors = {
    border: "border-neutral-800/80",
    text: "text-neutral-200",
    textMuted: "text-neutral-500",
    inputBg: "bg-[#121212]"
  };
  const popupCardBg = "bg-[#0F0F0F]";

  useEffect(() => {
    try {
      // Parse the cleanly formatted route query parameters string safely
      const queryString = window.location.hash.split("?")[1];
      if (!queryString) throw new Error("Missing parameters matrix.");

      const params = new URLSearchParams(queryString);
      const moduleId = params.get("module");
      const ticker = params.get("ticker");

      if (!moduleId || !ticker) throw new Error("Malformed query identifiers.");

      invoke("compile_popup_telemetry", { moduleId, ticker })
        .then((response) => setLayoutData(response))
        .catch((err) => setError(String(err)));
        
    } catch (err: any) {
      setError(err.message || "Unknown Routing Failure");
    }
  }, []);

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