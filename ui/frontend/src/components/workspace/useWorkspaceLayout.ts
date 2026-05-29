import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UiPrimitiveNode } from "./PrimitiveCompiler";
import { useWorkspaceResize } from "./useWorkspaceResize";
import { compactLayoutWithGravity } from "./layoutGravity";

export interface ServerModuleConfig {
  id: string;
  title: string;
  x: number;
  y: number;
  w: number;
  h: number;
  root_node: UiPrimitiveNode;
}

interface CatalogItem {
  id: string;
  name: string;
  description: string;
}

// 🎯 HELPER FUNCTION: Mounts card content off-screen for a split second to compute its natural height units
async function calculateIdealHeightUnits(rootNode: UiPrimitiveNode, targetWCols: number): Promise<number> {
  const scratchpad = document.createElement("div");
  scratchpad.style.position = "absolute";
  scratchpad.style.visibility = "hidden";
  scratchpad.style.top = "-9999px";
  scratchpad.style.left = "-9999px";

  const mainCanvas = document.querySelector("main") as HTMLElement;
  const canvasRect = mainCanvas?.getBoundingClientRect();
  const gridUnitWidth = canvasRect ? (canvasRect.width - 48) / 12 : 90;
  
  scratchpad.style.width = `${gridUnitWidth * targetWCols}px`;
  document.body.appendChild(scratchpad);

  try {
    const ReactDOM = await import("react-dom/client");
    const React = await import("react");
    const PrimitiveCompiler = (await import("./PrimitiveCompiler")).default;
    
    const root = ReactDOM.createRoot(scratchpad);
    root.render(
      React.createElement(PrimitiveCompiler, {
        node: rootNode,
        colors: { border: "border-neutral-800", textMuted: "text-neutral-500" },
        cardBg: "bg-[#0A0A0B]"
      })
    );

    // Give browser a tiny frame window to calculate typography constraints
    await new Promise((resolve) => setTimeout(resolve, 25));
    const naturalPixelHeight = scratchpad.scrollHeight;

    root.unmount();
    scratchpad.remove();

    const gridUnitHeight = 20;
    // Base card chrome/padding needs roughly 40-60px extra headroom
    return Math.max(8, Math.ceil((naturalPixelHeight + 50) / gridUnitHeight));
  } catch (err) {
    scratchpad.remove();
    return 14; // Clean fallback if calculation fails
  }
}

export function useWorkspaceLayout(selectedTicker: string | null, isEditing: boolean, resetSignal: number) {
  const [activeModules, setActiveModules] = useState<ServerModuleConfig[]>([]);
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [dragCoords, setDragCoords] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [clickOffset, setClickOffset] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const wasEditingRef = useRef(isEditing);

  const { isResizingZone, setIsResizingZone, handleMouseMove, startResizeExecution } = 
    useWorkspaceResize(isEditing, (updater) => {
      setActiveModules((prev) => {
        const next = typeof updater === "function" ? updater(prev) : updater;
        return compactLayoutWithGravity(next, null);
      });
    });

  const loadLayoutForTicker = (tickerStr: string) => {
    invoke<string>("load_workspace_layout")
      .then(async (rawJson) => {
        let savedBlocks: Record<string, { x: number; y: number; w: number; h: number }> = {};
        let isFreshDefaultLayout = false;

        if (rawJson && rawJson !== "{}" && rawJson.trim() !== "") {
          const parsed = JSON.parse(rawJson);
          if (parsed.blocks) savedBlocks = parsed.blocks;
        }

        let activeModuleIds = Object.keys(savedBlocks);
        if (activeModuleIds.length === 0) {
          try {
            isFreshDefaultLayout = true;
            const catalog = await invoke<CatalogItem[]>("fetch_component_catalog");
            catalog.forEach((item, index) => {
              const xPos = (index % 2) * 6;
              const yPos = Math.floor(index / 2) * 14;
              // Provide a safe temporary width of 6 columns (half screen) or full width (12) for profiles
              const targetW = item.id.includes("profile") ? 12 : 6;
              savedBlocks[item.id] = { x: xPos, y: yPos, w: targetW, h: 10 };
            });
            activeModuleIds = Object.keys(savedBlocks);
          } catch (catalogErr) {
            console.error("Failed to fetch layout catalog:", catalogErr);
          }
        }

        const compileTasks = activeModuleIds.map(async (moduleId) => {
          try {
            const res: any = await invoke("fetch_component_telemetry", { ticker: tickerStr, moduleId });
            let coords = savedBlocks[moduleId];

            // 🎯 AUTO-CALCULATE IF FRESH: If there's no saved config, measure the height automatically right now!
            if (isFreshDefaultLayout) {
              const perfectH = await calculateIdealHeightUnits(res.root_node, coords.w);
              coords = { ...coords, h: perfectH };
            }

            return {
              id: res.id,
              title: res.title,
              x: coords.x,
              y: coords.y,
              w: coords.w,
              h: coords.h,
              root_node: res.root_node,
            };
          } catch {
            return null;
          }
        });

        const rawList = (await Promise.all(compileTasks)).filter(Boolean) as ServerModuleConfig[];
        setActiveModules(compactLayoutWithGravity(rawList, null));
      })
      .catch((err) => console.error(err));
  };

  useEffect(() => {
    if (selectedTicker) loadLayoutForTicker(selectedTicker);
  }, [selectedTicker, resetSignal]);

  useEffect(() => {
    if (wasEditingRef.current && !isEditing && activeModules.length > 0) {
      const blocks: Record<string, any> = {};
      activeModules.forEach((m) => {
        blocks[m.id] = { x: m.x, y: m.y, w: m.w, h: m.h };
      });
      invoke("save_workspace_layout", { layoutJson: JSON.stringify({ blocks }) }).catch(err => console.error(err));
    }
    wasEditingRef.current = isEditing;
  }, [isEditing, activeModules]);

  const startDragExecution = (e: React.MouseEvent, targetId: string) => {
    const target = e.target as HTMLElement;
    if (!isEditing || isResizingZone || target.closest('button')) return;

    e.preventDefault();
    const cardElement = target.closest("[data-module-card]");
    
    let localOffsetX = 0;
    let localOffsetY = 0;

    if (cardElement) {
      const rect = cardElement.getBoundingClientRect();
      localOffsetX = e.clientX - rect.left;
      localOffsetY = e.clientY - rect.top;
      
      setClickOffset({ x: localOffsetX, y: localOffsetY });
      setDragCoords({ x: e.clientX, y: e.clientY });
    }

    setDraggedId(targetId);

    const mainCanvas = document.querySelector("main") as HTMLElement;
    const canvasRect = mainCanvas?.getBoundingClientRect();
    const gridUnitWidth = canvasRect ? (canvasRect.width - 48) / 12 : 90;
    const gridUnitHeight = 20;

    const executeDragMove = (moveEvent: MouseEvent) => {
      setDragCoords({ x: moveEvent.clientX, y: moveEvent.clientY });
      if (!canvasRect) return;
      
      const relativeX = moveEvent.clientX - canvasRect.left - localOffsetX;
      const relativeY = moveEvent.clientY - canvasRect.top - localOffsetY + mainCanvas.scrollTop;

      const targetX = Math.max(0, Math.min(11, Math.round(relativeX / gridUnitWidth)));
      const targetY = Math.max(0, Math.round(relativeY / gridUnitHeight));

      setActiveModules((prev) => {
        const currentCard = prev.find(m => m.id === targetId);
        if (!currentCard) return prev;

        const boundedX = Math.min(targetX, 12 - currentCard.w);

        if (currentCard.x === boundedX && currentCard.y === targetY) return prev;

        const modifiedList = prev.map((m) => {
          if (m.id !== targetId) return m;
          return { ...m, x: boundedX, y: targetY };
        });

        return compactLayoutWithGravity(modifiedList, targetId);
      });
    };

    const terminateDrag = () => {
      setDraggedId(null);
      setActiveModules(prev => compactLayoutWithGravity(prev, null));
      window.removeEventListener("mousemove", executeDragMove);
      window.removeEventListener("mouseup", terminateDrag);
    };

    window.addEventListener("mousemove", executeDragMove);
    window.addEventListener("mouseup", terminateDrag);
  };

  const appendCatalogItemToGrid = async (moduleId: string) => {
    if (!selectedTicker || activeModules.some((m) => m.id === moduleId)) return;
    try {
      const res: any = await invoke("fetch_component_telemetry", { ticker: selectedTicker, moduleId });
      
      // 🎯 AUTO-CALCULATE ON APPEND: Set ideal width span based on card type, then auto-measure exact height
      const idealW = moduleId.includes("profile") ? 12 : 6;
      const perfectH = await calculateIdealHeightUnits(res.root_node, idealW);

      const highestY = activeModules.reduce((max, m) => Math.max(max, m.y + m.h), 0);
      setActiveModules((prev) => compactLayoutWithGravity([
        ...prev, 
        { id: res.id, title: res.title, x: 0, y: highestY, w: idealW, h: perfectH, root_node: res.root_node }
      ], null));
    } catch (err) {
      console.error(err);
    }
  };

  const removeModule = (targetId: string) => {
    setActiveModules((prev) => compactLayoutWithGravity(prev.filter((m) => m.id !== targetId), null));
  };

  return {
    activeModules,
    draggedId,
    dragCoords,
    clickOffset,
    isResizingZone,
    setIsResizingZone,
    handleMouseMove,
    startResizeExecution,
    startDragExecution,
    appendCatalogItemToGrid,
    removeModule
  };
}