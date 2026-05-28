import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UiPrimitiveNode } from "./PrimitiveCompiler";
import { useWorkspaceResize } from "./useWorkspaceResize";

export interface ServerModuleConfig {
  id: string;
  title: string;
  x: number;
  y: number;
  w: number;
  h: number;
  root_node: UiPrimitiveNode;
}

// 🎯 UNIVERSAL 4-WAY COMPACTION ENGINE: Enforces both Top-Gravity and Left-Gravity with multi-directional shifting
function compactLayoutWithGravity(items: ServerModuleConfig[], movingId: string | null): ServerModuleConfig[] {
  // Sort items primarily from top-to-bottom (Y), and secondarily left-to-right (X)
  const sorted = [...items].sort((a, b) => a.y - b.y || a.x - b.x);
  const layoutMap: Record<string, ServerModuleConfig> = {};
  sorted.forEach(m => { layoutMap[m.id] = { ...m }; });

  let itemsChanged = true;
  let loopsCounter = 0;

  // Run the physics loops until all components stabilize into their tightest geometric configuration
  while (itemsChanged && loopsCounter < 150) {
    itemsChanged = false;
    loopsCounter++;

    for (let i = 0; i < sorted.length; i++) {
      const current = layoutMap[sorted[i].id];
      if (current.id === movingId) continue; // Allow the item under the user's cursor to glide freely

      // 1. 🔺 VERTICAL UPWARD COMPACTION PASS (Top Gravity)
      if (current.y > 0) {
        let canFloatUp = true;
        const proposedY = current.y - 1;

        for (let j = 0; j < sorted.length; j++) {
          if (sorted[j].id === current.id) continue;
          const other = layoutMap[sorted[j].id];

          if (
            current.x < other.x + other.w &&
            current.x + current.w > other.x &&
            proposedY < other.y + other.h &&
            proposedY + current.h > other.y
          ) {
            canFloatUp = false;
            break;
          }
        }

        if (canFloatUp) {
          current.y = proposedY;
          itemsChanged = true;
        }
      }

      // 2. ◀️ HORIZONTAL LEFTWARD COMPACTION PASS (Left Gravity)
      if (current.x > 0) {
        let canFloatLeft = true;
        const proposedX = current.x - 1;

        for (let j = 0; j < sorted.length; j++) {
          if (sorted[j].id === current.id) continue;
          const other = layoutMap[sorted[j].id];

          if (
            proposedX < other.x + other.w &&
            proposedX + current.w > other.x &&
            current.y < other.y + other.h &&
            current.y + current.h > other.y
          ) {
            canFloatLeft = false;
            break;
          }
        }

        if (canFloatLeft) {
          current.x = proposedX;
          itemsChanged = true;
        }
      }

      // 3. 🗺️ OMNIDIRECTIONAL COLLISION PASS (Handles Left, Right, Top, and Bottom overlaps natively)
      for (let j = 0; j < sorted.length; j++) {
        if (sorted[j].id === current.id) continue;
        const other = layoutMap[sorted[j].id];

        const collides = (
          current.x < other.x + other.w &&
          current.x + current.w > other.x &&
          current.y < other.y + other.h &&
          current.y + current.h > other.y
        );

        if (collides) {
          itemsChanged = true;

          // If one of the overlapping blocks is the item actively dragged, it has physical priority
          if (current.id === movingId) {
            // Determine the overlap direction based on previous matrix indices
            const cameFromLeft = current.x <= other.x;
            
            // If dragging into an item from its left side, push the neighbor cleanly to the right
            if (cameFromLeft && other.x + other.w < 12) {
              other.x = Math.min(12 - other.w, other.x + 1);
            } else {
              // Otherwise, cascade the element downward out of the active cursor frame
              other.y = current.y + current.h;
            }
          } else if (other.id === movingId) {
            const cameFromLeft = other.x <= current.x;

            if (cameFromLeft && current.x + current.w < 12) {
              current.x = Math.min(12 - current.w, current.x + 1);
            } else {
              current.y = other.y + other.h;
            }
          } else {
            // Static background grid structures default to a clean sequential waterfall cascade
            if (current.y <= other.y) {
              other.y = current.y + current.h;
            } else {
              current.y = other.y + other.h;
            }
          }
        }
      }
    }
  }

  return sorted.map(item => layoutMap[item.id]);
}

export function useWorkspaceLayout(selectedTicker: string | null, isEditing: boolean, resetSignal: number) {
  const [activeModules, setActiveModules] = useState<ServerModuleConfig[]>([]);
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [dragCoords, setDragCoords] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [clickOffset, setClickOffset] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const wasEditingRef = useRef(isEditing);

  // Wire up the single-source resize hook context link
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
        let savedBlocks: Record<string, { x: number; y: number; w: number; h: number }> = {
          "overview_metrics": { x: 0, y: 0, w: 6, h: 12 },
          "performance_chart": { x: 6, y: 0, w: 6, h: 16 }
        };

        if (rawJson && rawJson !== "{}" && rawJson.trim() !== "") {
          const parsed = JSON.parse(rawJson);
          if (parsed.blocks) savedBlocks = parsed.blocks;
        }

        const compileTasks = Object.keys(savedBlocks).map(async (moduleId) => {
          try {
            const res: any = await invoke("fetch_component_telemetry", { ticker: tickerStr, moduleId });
            const coords = savedBlocks[moduleId];
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
  }, [selectedTicker]);

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

  // 🚀 INTERACTIVE DYNAMIC PUSH AND FLUID REORDER GESTURE HANDLING ENGINE
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

      // Map positions smoothly against the tight 12-column limits
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
      const highestY = activeModules.reduce((max, m) => Math.max(max, m.y + m.h), 0);
      setActiveModules((prev) => compactLayoutWithGravity([
        ...prev, 
        { id: res.id, title: res.title, x: 0, y: highestY, w: 4, h: 12, root_node: res.root_node }
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