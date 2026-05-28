import React from "react";
import PrimitiveCompiler from "./PrimitiveCompiler";
import type { UiPrimitiveNode } from "./PrimitiveCompiler";

interface WorkspaceCardProps {
  id: string;
  title: string;
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
  height,
  width,
  rootNode,
  index,
  isEditing,
  isResizingZone,
  isBeingDragged,
  colors,
  cardBg,
  onDragStart,
  onDragOver,
  onDragEnd,
  onMouseMove,
  onMouseLeave,
  onMouseDown,
  onRemove,
}: WorkspaceCardProps) {
  const currentWidth = width ? `${width}px` : "100%";

  const cursorClass = !isEditing ? "" : 
    isResizingZone === "nwse" ? "cursor-nwse-resize" :
    isResizingZone === "ns" ? "cursor-ns-resize" :
    isResizingZone === "ew" ? "cursor-ew-resize" : "cursor-grab active:cursor-grabbing";

  return (
    <div
      draggable={isEditing && !isResizingZone}
      onDragStart={() => onDragStart(index)}
      onDragOver={(e) => onDragOver(e, index)}
      onDragEnd={onDragEnd}
      onMouseMove={onMouseMove}
      onMouseLeave={onMouseLeave}
      onMouseDown={(e) => onMouseDown(e, id)}
      className={`relative rounded-xl p-5 flex flex-col overflow-hidden ${cursorClass} ${
        isEditing 
          ? "border border-dashed border-neutral-600/60 bg-neutral-500/[0.02]" 
          : "border border-transparent"
      } ${isBeingDragged ? "opacity-30 scale-[0.99]" : "opacity-100"}`}
      style={{ 
        height: `${height}px`, 
        width: currentWidth, 
        maxWidth: "100%" 
      }}
    >
      <div className="font-sans text-[10px] font-medium tracking-widest uppercase opacity-40 pl-1 mb-2.5 flex-shrink-0">
        {title}
      </div>

      <div className="w-full h-full flex-1 min-h-0 pointer-events-none">
        <PrimitiveCompiler node={rootNode} colors={colors} cardBg={cardBg} />
      </div>

      {isEditing && (
        <>
          {/* 🎯 FIXED: Cleaned up the rogue semicolon syntax error here */}
          <div className="absolute top-3 right-3 flex items-center gap-2 font-sans text-[10px] animate-fadeIn pointer-events-auto z-50">
            <span className={`px-2 py-1 border ${colors.border} rounded-md bg-neutral-900/80 backdrop-blur-sm text-[10px] opacity-60 font-medium tracking-wide`}>
              ⠿ DRAG OR SHAPE BY EDGES
            </span>
            <button
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onRemove(id);
              }}
              className="w-6 h-6 rounded-md border border-red-500/30 bg-red-500 text-white flex items-center justify-center font-bold text-xs cursor-pointer shadow-md hover:bg-red-600 active:scale-95 transition-all pointer-events-auto"
              title="Remove Module Component from Workspace"
            >
              ✕
            </button>
          </div>
          
          <div className="absolute bottom-1.5 right-1.5 p-0.5 opacity-45 text-xs font-sans selection:bg-transparent pointer-events-none">
            ◢
          </div>
        </>
      )}
    </div>
  );
}