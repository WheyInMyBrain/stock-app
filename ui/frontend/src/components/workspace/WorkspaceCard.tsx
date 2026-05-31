// stock-app/ui/frontend/src/components/workspace/WorkspaceCard.tsx

import React, { useEffect, useState, useCallback } from "react"; 
import { invoke } from "@tauri-apps/api/core"; 
import PrimitiveCompiler from "./PrimitiveCompiler";
import type { UiPrimitiveNode } from "./PrimitiveCompiler";
import { usePipelineListener } from "./usePipelineListener";

interface WorkspaceCardProps {
  id: string; 
  title: string;
  ticker: string; 
  height: number;
  width?: number;
  rootNode: UiPrimitiveNode; 
  index: number;
  isEditing: boolean;
  isResizingZone: string | null;
  isBeingDragged: boolean;
  colors: any;
  cardBg: string; 
  onDragStart: (index: number) => void;
  onDragOver: (e: React.DragEvent, index: number) => void;
  onDragEnd: () => void;
  onMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseLeave: () => void;
  onMouseDown: (e: React.MouseEvent<HTMLDivElement>, id: string) => void;
  onRemove: (id: string) => void;
}

export default function WorkspaceCard({
  id,
  title,
  ticker,
  height,
  width,
  rootNode,
  isEditing,
  isResizingZone,
  isBeingDragged,
  colors,
  cardBg,
  onMouseMove,
  onMouseLeave,
  onMouseDown,
  onRemove,
}: WorkspaceCardProps) {
  // 🚀 LOCAL TIMEFRAME STATE: Tracks selection strictly for this isolated card instance
  const [activeTimeframe, setActiveTimeframe] = useState<string>("");
  const [currentNode, setCurrentNode] = useState<UiPrimitiveNode>(rootNode);

  // Sync state if the master layout grid changes down from parent context
  useEffect(() => {
    setCurrentNode(rootNode);
  }, [rootNode]);

  // 🎯 THE OPTIMIZED REFECTH: Updates data values in-memory instantly
  const handleLiveTelemetryRefresh = useCallback(async () => {
    try {
      const freshTelemetry: any = await invoke("fetch_component_telemetry", {
        ticker: ticker,
        moduleId: id,
        timeframe: activeTimeframe // 🚀 Updated to pass this card's custom timeframe preference state
      });

      if (freshTelemetry && freshTelemetry.root_node) {
        setCurrentNode(freshTelemetry.root_node);
      }
    } catch (err) {
      console.error(`❌ [CARD REFRESH ERROR]: Refetch task failed for [${id}]:`, err);
    }
  }, [id, ticker, activeTimeframe]);

  // 🚀 ISOLATED INTERNAL INTERCEPTOR HOOK: Listens for time changes meant only for this component instance
  useEffect(() => {
    const listenToIntervalOverrides = async (event: Event) => {
      const customEvent = event as CustomEvent<{ moduleId: string; timeframe: string }>;
      const { moduleId, timeframe } = customEvent.detail;

      // 🎯 SURGICAL CARD FILTER: Only pull new data if it matches this exact card's ID
      if (moduleId === id) {
        try {
          const freshData: any = await invoke("fetch_component_telemetry", {
            ticker: ticker,
            moduleId: id,
            timeframe: timeframe
          });

          if (freshData && freshData.root_node) {
            setActiveTimeframe(timeframe); // Lock in the preference string state locally
            setCurrentNode(freshData.root_node); // Swap data lines safely
          }
        } catch (err) {
          console.error(`❌ [LOCAL INTERVAL ERROR]: Dropdown reload failed for card [${id}]:`, err);
        }
      }
    };

    window.addEventListener("WORKSPACE_TIMEFRAME_OVERRIDE", listenToIntervalOverrides);
    return () => {
      window.removeEventListener("WORKSPACE_TIMEFRAME_OVERRIDE", listenToIntervalOverrides);
    };
  }, [id, ticker]);

  // Bind refresh action directly to the background thread bridge
  usePipelineListener(id, ticker, handleLiveTelemetryRefresh);

  const currentWidth = width ? `${width}px` : "100%";
  const cursorClass = !isEditing ? "" : 
    isResizingZone === "nwse" ? "cursor-nwse-resize" :
    isResizingZone === "ns" ? "cursor-ns-resize" :
    isResizingZone === "ew" ? "cursor-ew-resize" : "cursor-grab active:cursor-grabbing";

  return (
    <div
      onMouseDown={(e) => isEditing && onMouseDown(e, id)}
      onMouseMove={isEditing ? onMouseMove : undefined}
      onMouseLeave={isEditing ? onMouseLeave : undefined}
      className={`relative rounded-xl p-5 flex flex-col overflow-hidden transition-all duration-200 ease-out select-none border ${cursorClass} ${
        isEditing 
          ? "border-neutral-500/30 bg-neutral-500/[0.02]" 
          : colors?.border || "border-neutral-200 dark:border-neutral-800"
      } ${isBeingDragged ? "opacity-25 scale-[0.98] bg-neutral-500/[0.06] border-dashed border-neutral-500/50 shadow-inner" : "opacity-100 scale-100"}`}
      style={{ 
        height: `${height}px`, 
        width: currentWidth, 
        maxWidth: "100%",
        backgroundColor: cardBg 
      }}
    >
      <div className={`font-sans text-[10px] font-medium tracking-widest uppercase opacity-50 pl-1 mb-2.5 flex-shrink-0 ${colors?.textMuted || "text-neutral-500"}`}>
        {title}
      </div>

      <div className="w-full h-full flex-1 min-h-0">
        <PrimitiveCompiler node={currentNode} colors={colors} cardBg={cardBg} />
      </div>

      {isEditing && (
        <>
          <div className="absolute top-3 right-3 flex items-center gap-2 font-sans text-[10px] animate-fadeIn pointer-events-auto z-50">
            <span className={`px-2 py-1 border ${colors?.border || "border-neutral-800"} rounded-md bg-neutral-900/80 backdrop-blur-sm text-[10px] text-white opacity-60 font-medium tracking-wide`}>
              ⠿ PRESS & DRAG CARD CENTER
            </span>
            <button
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onRemove(id);
              }}
              className="w-6 h-6 rounded-md border border-red-500/30 bg-red-500 text-white flex items-center justify-center font-bold text-xs cursor-pointer shadow-md hover:bg-red-600 active:scale-95 transition-all"
            >
              ✕
            </button>
          </div>
          <div className="absolute bottom-1.5 right-1.5 p-0.5 opacity-45 text-xs font-sans selection:bg-transparent pointer-events-none text-neutral-400">
            ◢
          </div>
        </>
      )}
    </div>
  );
}