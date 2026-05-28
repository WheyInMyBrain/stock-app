import React, { useState } from "react";
import type { ServerModuleConfig } from "./useWorkspaceLayout";

export function useWorkspaceDrag(
  isEditing: boolean,
  activeModules: ServerModuleConfig[],
  setActiveModules: React.Dispatch<React.SetStateAction<ServerModuleConfig[]>>
) {
  const [draggedId, setDraggedId] = useState<string | null>(null);
  
  // 🎯 Track the live position offsets of the cursor for hardware-accelerated transforms
  const [dragCoords, setDragCoords] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  // 🎯 Track where inside the card the user originally clicked so it doesn't snap awkwardly
  const [clickOffset, setClickOffset] = useState<{ x: number; y: number }>({ x: 0, y: 0 });

  const startDragExecution = (e: React.MouseEvent, targetId: string, currentIndex: number) => {
    const target = e.target as HTMLElement;
    if (!isEditing || target.closest('[title*="Shape"], button, .cursor-nwse-resize, .cursor-ns-resize, .cursor-ew-resize')) {
      return;
    }

    e.preventDefault();
    
    // Find the bounding box of the card that was clicked
    const cardElement = target.closest("[data-module-index]");
    if (cardElement) {
      const rect = cardElement.getBoundingClientRect();
      setClickOffset({
        x: e.clientX - rect.left,
        y: e.clientY - rect.top
      });
      setDragCoords({ x: e.clientX, y: e.clientY });
    }

    setDraggedId(targetId);
    let lastActiveIndex = currentIndex;

    const executeDragMove = (moveEvent: MouseEvent) => {
      // 🎯 Update the coordinates on every single mouse movement frame
      setDragCoords({ x: moveEvent.clientX, y: moveEvent.clientY });

      const elementsUnderCursor = document.elementsFromPoint(moveEvent.clientX, moveEvent.clientY);
      const hoveredContainer = elementsUnderCursor.find(el => el.hasAttribute("data-module-index"));
      
      if (hoveredContainer) {
        const targetIndex = parseInt(hoveredContainer.getAttribute("data-module-index") || "-1", 10);
        
        if (targetIndex !== -1 && targetIndex !== lastActiveIndex) {
          setActiveModules((prev) => {
            const updated = [...prev];
            const sourceIdx = prev.findIndex(m => m.id === targetId);
            if (sourceIdx === -1) return prev;
            
            const [movedItem] = updated.splice(sourceIdx, 1);
            updated.splice(targetIndex, 0, movedItem);
            return updated;
          });
          lastActiveIndex = targetIndex;
        }
      }
    };

    const terminateDrag = () => {
      setDraggedId(null);
      window.removeEventListener("mousemove", executeDragMove);
      window.removeEventListener("mouseup", terminateDrag);
    };

    window.addEventListener("mousemove", executeDragMove);
    window.addEventListener("mouseup", terminateDrag);
  };

  return { draggedId, dragCoords, clickOffset, startDragExecution };
}