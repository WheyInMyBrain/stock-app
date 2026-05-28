import { useState } from "react";
import WorkspaceCard from "./workspace/WorkspaceCard";
import CustomizationPanel from "./CustomizationPanel";
import { useWorkspaceLayout } from "./workspace/useWorkspaceLayout";
import { useWorkspaceResize } from "./workspace/useWorkspaceResize";
import { useWorkspaceDrag } from "./workspace/useWorkspaceDrag";

interface WorkspaceProps {
  selectedTicker: string | null;
  isEditing: boolean;
  resetSignal: number;
  colors: any;
}

export default function Workspace({ selectedTicker, isEditing, resetSignal, colors }: WorkspaceProps) {
  const cardBg = colors.input.includes("bg-[#0A0A0A]") ? "bg-[#121212]" : "bg-[#F4F4F5]";

  const { activeModules, setActiveModules, appendCatalogItemToGrid, removeModule } = 
    useWorkspaceLayout(selectedTicker, isEditing, resetSignal);

  const { isResizingZone, setIsResizingZone, handleMouseMove, startResizeExecution } = 
    useWorkspaceResize(isEditing, setActiveModules);

  const { draggedId, dragCoords, clickOffset, startDragExecution } = 
    useWorkspaceDrag(isEditing, activeModules, setActiveModules);

  const draggedModule = activeModules.find((m) => m.id === draggedId);
  const isPanelOpen = isEditing && selectedTicker !== null;

  return (
    <div className="flex-1 w-full h-full flex overflow-hidden font-sans relative">
      
      <main
        onDragOver={(e) => isEditing && e.preventDefault()}
        onDrop={(e) => {
          if (!isEditing) return;
          const moduleId = e.dataTransfer.getData("text/plain");
          if (moduleId) appendCatalogItemToGrid(moduleId);
        }}
        className="flex-1 h-full p-6 overflow-y-auto select-none bg-transparent"
      >
        {selectedTicker ? (
          <div className="w-full px-2 flex flex-col md:flex-row flex-nowrap items-start gap-6 transition-all duration-300 ease-out">
            {activeModules.map((module, index) => {
              const isBeingDragged = draggedId === module.id;

              return (
                <div
                  key={module.id}
                  data-module-index={index}
                  className="transition-all duration-300 ease-out"
                  style={{
                    width: module.width ? `${module.width}px` : "100%",
                    flex: module.width ? `0 1 ${module.width}px` : "1 1 0%",
                    minWidth: "180px",
                  }}
                >
                  {isBeingDragged ? (
                    <div 
                      className="rounded-xl border border-dashed border-neutral-500/40 bg-neutral-500/[0.01] transition-all duration-300 animate-pulse"
                      style={{ height: `${module.height ?? 220}px`, width: "100%" }}
                    />
                  ) : (
                    <WorkspaceCard
                      id={module.id}
                      title={module.title}
                      height={module.height ?? 220}
                      width={module.width}
                      rootNode={module.root_node}
                      index={index}
                      isEditing={isEditing}
                      isResizingZone={isResizingZone}
                      isBeingDragged={false}
                      colors={colors}
                      cardBg={cardBg}
                      onDragStart={() => {}} 
                      onDragOver={() => {}}
                      onDragEnd={() => {}}
                      /* 🎯 FIXED: Passed mouse listeners explicitly down to the core layout box */
                      onMouseMove={handleMouseMove}
                      onMouseLeave={() => setIsResizingZone(null)}
                      onMouseDown={(e, id) => {
                        if (isResizingZone) {
                          startResizeExecution(e, id);
                        } else {
                          startDragExecution(e, id, index);
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

      {/* Hardware Accelerated Floating Overlay Tile remains completely responsive */}
      {isEditing && draggedId && draggedModule && (
        <div
          className="fixed pointer-events-none z-[999] opacity-85 mix-blend-normal transform scale-[1.01] shadow-2xl transition-transform duration-75 ease-out"
          style={{
            left: `${dragCoords.x - clickOffset.x}px`,
            top: `${dragCoords.y - clickOffset.y}px`,
            width: draggedModule.width ? `${draggedModule.width}px` : "auto",
            minWidth: draggedModule.width ? "unset" : "calc(100vw - 26rem)",
            maxWidth: "calc(100vw - 6rem)",
          }}
        >
          <WorkspaceCard
            id={draggedModule.id}
            title={draggedModule.title}
            height={draggedModule.height ?? 220}
            width={draggedModule.width}
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