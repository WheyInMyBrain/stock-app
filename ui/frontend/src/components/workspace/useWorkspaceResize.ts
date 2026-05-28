import React, { useState } from "react";
import type { ServerModuleConfig } from "./useWorkspaceLayout";

export function useWorkspaceResize(isEditing: boolean, setActiveModules: React.Dispatch<React.SetStateAction<ServerModuleConfig[]>>) {
  const [isResizingZone, setIsResizingZone] = useState<string | null>(null);

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!isEditing) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const pad = 20;

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

    let startWidth = 0;
    let startHeight = 220;

    setActiveModules((prev) => {
      const target = prev.find((m) => m.id === targetId);
      if (target) {
        startWidth = target.width || e.currentTarget.clientWidth;
        startHeight = target.height || 220;
      }
      return prev;
    });

    const initialZone = isResizingZone;

    const executeResize = (moveEvent: MouseEvent) => {
      setActiveModules((prev) =>
        prev.map((m) => {
          if (m.id !== targetId) return m;
          const updated = { ...m };
          if (initialZone === "ns" || initialZone === "nwse") {
            updated.height = Math.max(150, Math.min(800, startHeight + (moveEvent.clientY - startY)));
          }
          if (initialZone === "ew" || initialZone === "nwse") {
            updated.width = Math.max(320, startWidth + (moveEvent.clientX - startX));
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