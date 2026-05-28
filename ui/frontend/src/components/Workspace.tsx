import { useState } from "react";
import WorkspaceCard from "./workspace/WorkspaceCard";
import CustomizationPanel from "./CustomizationPanel";
import { useWorkspaceLayout } from "./workspace/useWorkspaceLayout";
import { useWorkspaceResize } from "./workspace/useWorkspaceResize";

interface WorkspaceProps {
  selectedTicker: string | null;
  isEditing: boolean;
  resetSignal: number;
  colors: any;
}

export default function Workspace({ selectedTicker, isEditing, resetSignal, colors }: WorkspaceProps) {
  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
  const cardBg = colors.input.includes("bg-[#0A0A0A]") ? "bg-[#121212]" : "bg-[#F4F4F5]";

  // 🪐 Leverage our custom abstract module hooks
  const { activeModules, setActiveModules, appendCatalogItemToGrid, removeModule } = 
    useWorkspaceLayout(selectedTicker, isEditing, resetSignal);

  const { isResizingZone, setIsResizingZone, handleMouseMove, startResizeExecution } = 
    useWorkspaceResize(isEditing, setActiveModules);

  // Drag and drop grid ordering sequence
  const handleDragOver = (e: React.DragEvent, index: number) => {
    e.preventDefault();
    if (draggedIndex === null || draggedIndex === index) return;
    const next = [...activeModules];
    const item = next.splice(draggedIndex, 1)[0];
    next.splice(index, 0, item);
    setDraggedIndex(index);
    setActiveModules(next);
  };

  return (
    <div className="flex-1 w-full h-full flex overflow-hidden font-sans">
      
      {/* 📊 Central Data Canvas Station Drop Area */}
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
          <div className="w-full flex flex-col gap-6 max-w-6xl mx-auto">
            <div className="flex flex-col gap-6">
              {activeModules.map((module, index) => (
                <WorkspaceCard
                  key={module.id}
                  id={module.id}
                  title={module.title}
                  height={module.height ?? 220}
                  width={module.width}
                  rootNode={module.root_node}
                  index={index}
                  isEditing={isEditing}
                  isResizingZone={isResizingZone}
                  isBeingDragged={draggedIndex === index}
                  colors={colors}
                  cardBg={cardBg}
                  onDragStart={(idx) => setDraggedIndex(idx)}
                  onDragOver={handleDragOver}
                  onDragEnd={() => setDraggedIndex(null)}
                  onMouseMove={handleMouseMove}
                  onMouseLeave={() => setIsResizingZone(null)}
                  onMouseDown={startResizeExecution}
                  onRemove={removeModule}
                />
              ))}
            </div>
          </div>
        ) : (
          <div className="h-full w-full flex flex-col justify-center items-center text-center tracking-wide">
            <div className={`text-xs font-medium uppercase tracking-widest ${colors.textMuted} animate-pulse`}>
              Select a ticker from the data directory to initialize workspace
            </div>
          </div>
        )}
      </main>

      {/* 🧭 Component Drawer Vault Panel Column */}
      <CustomizationPanel
        isOpen={isEditing && selectedTicker !== null}
        activeModuleIds={activeModules.map((m) => m.id)}
        onAddModule={appendCatalogItemToGrid}
        colors={colors}
      />
    </div>
  );
}