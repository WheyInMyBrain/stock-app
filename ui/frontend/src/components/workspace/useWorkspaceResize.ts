import React, { useState } from "react";
import type { ServerModuleConfig } from "./useWorkspaceLayout";

export function useWorkspaceResize(isEditing: boolean, setActiveModules: React.Dispatch<React.SetStateAction<ServerModuleConfig[]>>) {
  const [isResizingZone, setIsResizingZone] = useState<string | null>(null);

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!isEditing) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const pad = 24; 

    const nearRight = x >= rect.width - pad;
    const nearBottom = y >= rect.height - pad;

    if (nearRight && nearBottom) setIsResizingZone("nwse");
    else if (nearBottom) setIsResizingZone("ns");
    else if (nearRight) setIsResizingZone("ew");
    else setIsResizingZone(null);
  };

  const startResizeExecution = (e: React.MouseEvent<HTMLDivElement>, targetId: string) => {
    if (!isResizingZone || !isEditing) return;
    
    e.preventDefault();
    e.stopPropagation();

    const startX = e.clientX;
    const startY = e.clientY;
    
    let startW = 1;
    let startH = 2;

    setActiveModules((prev) => {
      const target = prev.find(m => m.id === targetId);
      if (target) {
        startW = target.w;
        startH = target.h;
      }
      return prev;
    });

    const initialZone = isResizingZone;
    const mainCanvas = document.querySelector("main") as HTMLElement;
    
    // 🎯 FIXED: Synced calculations to the new smooth 12-column and 20px cell block settings
    const gridUnitWidth = mainCanvas ? (mainCanvas.clientWidth - 48) / 12 : 90;
    const gridUnitHeight = 20; 

    const executeResize = (moveEvent: MouseEvent) => {
      const deltaX = moveEvent.clientX - startX;
      const deltaY = moveEvent.clientY - startY;

      setActiveModules((prev) =>
        prev.map((m) => {
          if (m.id !== targetId) return m;
          const updated = { ...m };
          
          if (initialZone === "ns" || initialZone === "nwse") {
            const rowSteps = Math.round(deltaY / gridUnitHeight);
            // Height tracking bounds: minimum 2 units (40px) up to 60 units high
            updated.h = Math.max(2, Math.min(60, startH + rowSteps));
          }
          if (initialZone === "ew" || initialZone === "nwse") {
            const colSteps = Math.round(deltaX / gridUnitWidth);
            // 🎯 FIXED: Broadened horizontal scale cap up to the full 12 columns
            updated.w = Math.max(1, Math.min(12 - m.x, startW + colSteps));
          }
          return updated;
        })
      );
    };

    const terminateResize = () => {
      window.removeEventListener("mousemove", executeResize);
      window.removeEventListener("mouseup", terminateResize);
    };

    window.addEventListener("mousemove", executeResize);
    window.addEventListener("mouseup", terminateResize);
  };

  return { isResizingZone, setIsResizingZone, handleMouseMove, startResizeExecution };
}