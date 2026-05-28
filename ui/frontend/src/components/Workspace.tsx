import { useState } from "react";
import WorkspaceCard from "./workspace/WorkspaceCard";
import CustomizationPanel from "./CustomizationPanel";
import { useWorkspaceLayout } from "./workspace/useWorkspaceLayout";

interface WorkspaceProps {
  selectedTicker: string | null;
  isEditing: boolean;
  resetSignal: number;
  colors: any;
}

export default function Workspace({ selectedTicker, isEditing, resetSignal, colors }: WorkspaceProps) {
  const cardBg = colors.input.includes("bg-[#0A0A0A]") ? "bg-[#121212]" : "bg-[#F4F4F5]";

  const {
    activeModules,
    draggedId,
    dragCoords,
    clickOffset,
    isResizingZone,
    handleMouseMove,
    startResizeExecution,
    startDragExecution,
    appendCatalogItemToGrid,
    removeModule
  } = useWorkspaceLayout(selectedTicker, isEditing, resetSignal);

  const draggedModule = activeModules.find((m) => m.id === draggedId);
  const isPanelOpen = isEditing && selectedTicker !== null;

  // 🎯 FIXED: Balanced vertical grid heights calculation using high-density 20px intervals
  const totalGridRows = activeModules.reduce((max, m) => Math.max(max, m.y + m.h), 0);
  const calculatedCanvasHeight = Math.max(500, totalGridRows * 20 + 40);

  // 🎯 TRACK CANVAS CONTAINER PIXELS DIRECTLY: 
  // We locate the real main DOM canvas wrapper layout bounds to handle width calculations dynamically
  const mainCanvasElement = document.querySelector("main");
  const canvasPixelWidth = mainCanvasElement ? mainCanvasElement.clientWidth - 48 : 800;

  return (
    <div className="flex-1 w-full h-full flex overflow-hidden font-sans relative">
      
      {/* Primary Workspace Scroll Section */}
      <main className="flex-1 h-full p-6 overflow-y-auto select-none bg-transparent relative">
        {selectedTicker ? (
          <div 
            className="w-full relative transition-all duration-200"
            style={{ height: `${calculatedCanvasHeight}px` }}
          >
            {activeModules.map((module) => {
              const isBeingDragged = draggedId === module.id;

              // Compute high-density absolute style matrices
              const leftPct = (module.x * 100) / 12;
              const widthPct = (module.w * 100) / 12;
              const topPx = module.y * 20;
              const heightPx = module.h * 20 - 12;

              return (
                <div
                  key={module.id}
                  data-module-card={module.id}
                  className="absolute transition-all duration-75 ease-out px-3"
                  style={{
                    left: `${leftPct}%`,
                    top: `${topPx}px`,
                    width: `${widthPct}%`,
                    height: `${heightPx}px`,
                    zIndex: isBeingDragged ? 10 : 1,
                  }}
                >
                  {isBeingDragged ? (
                    <div 
                      className="rounded-xl border border-dashed border-neutral-500/40 bg-neutral-500/[0.01] w-full h-full animate-pulse"
                    />
                  ) : (
                    <WorkspaceCard
                      id={module.id}
                      title={module.title}
                      height={heightPx}
                      width={undefined} 
                      rootNode={module.root_node}
                      index={-1}
                      isEditing={isEditing}
                      isResizingZone={isResizingZone}
                      isBeingDragged={false}
                      colors={colors}
                      cardBg={cardBg}
                      onDragStart={() => {}} 
                      onDragOver={() => {}}
                      onDragEnd={() => {}}
                      onMouseMove={handleMouseMove}
                      onMouseLeave={() => {}}
                      onMouseDown={(e, id) => {
                        if (isResizingZone) {
                          startResizeExecution(e, id);
                        } else {
                          startDragExecution(e, id);
                        }
                      }}
                      onRemove={removeModule}
                    />
                  )}
                </div>
              );
            })}
          </div>
        ) : (
          <div className="h-full w-full flex flex-col justify-center items-center text-center tracking-wide">
            <div className={`text-xs font-medium uppercase tracking-widest ${colors.textMuted} animate-pulse`}>
              Select a ticker from the data directory to initialize workspace
            </div>
          </div>
        )}
      </main>

      {/* Hardware Accelerated Floating Overlay Layer */}
      {isEditing && draggedId && draggedModule && (
        <div
          className="fixed pointer-events-none z-[999] opacity-75 transform scale-[1.005] shadow-2xl transition-transform duration-75 ease-out"
          style={{
            left: `${dragCoords.x - clickOffset.x}px`,
            top: `${dragCoords.y - clickOffset.y}px`,
            
            /* 🎯 THE CRITICAL FIXED WIDTH: Multiply the canvas container's exact pixel width 
               by the card's column span ratio. This matches its real dashboard layout slot pixel-for-pixel! */
            width: `${(canvasPixelWidth * draggedModule.w) / 12}px`,
            height: `${draggedModule.h * 20 - 12}px`,
          }}
        >
          <WorkspaceCard
            id={draggedModule.id}
            title={draggedModule.title}
            height={draggedModule.h * 20 - 12}
            width={undefined}
            rootNode={draggedModule.root_node}
            index={-1}
            isEditing={isEditing}
            isResizingZone={null}
            isBeingDragged={true}
            colors={colors}
            cardBg={cardBg}
            onDragStart={() => {}}
            onDragOver={() => {}}
            onDragEnd={() => {}}
            onMouseMove={() => {}}
            onMouseLeave={() => {}}
            onMouseDown={() => {}}
            onRemove={() => {}}
          />
        </div>
      )}

      <CustomizationPanel
        isOpen={isPanelOpen}
        activeModuleIds={activeModules.map((m) => m.id)}
        onAddModule={appendCatalogItemToGrid}
        colors={colors}
      />

    </div>
  );
}