import { useState, useEffect } from "react";
import { componentRegistry, defaultLayoutOrder } from "./workspace/registry";

interface WorkspaceProps {
  selectedTicker: string | null;
  isEditing: boolean;
  resetSignal: number; // 🎯 Reactive layout signal identifier
  colors: any;
}

export default function Workspace({ selectedTicker, isEditing, resetSignal, colors }: WorkspaceProps) {
  // 🏛️ PERSISTENCE STORAGE CONFIG KEYS
  const STORAGE_ORDER_KEY = "workspace_layout_order_schema";
  const STORAGE_DIM_KEY = "workspace_layout_dimensions_schema";

  // State initialization pulls directly from abstracted LocalStorage schemas if active
  const [layoutOrder, setLayoutOrder] = useState<string[]>(() => {
    const saved = localStorage.getItem(STORAGE_ORDER_KEY);
    return saved ? JSON.parse(saved) : defaultLayoutOrder;
  });

  const [dimensions, setDimensions] = useState<Record<string, { width?: number; height: number }>>(() => {
    const saved = localStorage.getItem(STORAGE_DIM_KEY);
    return saved ? JSON.parse(saved) : {};
  });

  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
  const [isResizingZone, setIsResizingZone] = useState<string | null>(null);

  // 🚀 AUTOMATED PERSISTENCE INTERCEPTOR
  // Auto-saves layout order and dimensions to cache whenever Edit Mode turns OFF
  useEffect(() => {
    if (!isEditing) {
      localStorage.setItem(STORAGE_ORDER_KEY, JSON.stringify(layoutOrder));
      localStorage.setItem(STORAGE_DIM_KEY, JSON.stringify(dimensions));
    }
  }, [isEditing, layoutOrder, dimensions]);

  // 🚀 DYNAMIC RESET ENGINE
  // Triggers instantly when the header icon dispatch updates the pipeline array
  useEffect(() => {
    if (resetSignal > 0) {
      localStorage.removeItem(STORAGE_ORDER_KEY);
      localStorage.removeItem(STORAGE_DIM_KEY);
      setLayoutOrder(defaultLayoutOrder);
      setDimensions({});
    }
  }, [resetSignal]);

  // 🛠️ Drag & Drop logic
  const handleDragStart = (index: number) => {
    if (!isEditing || isResizingZone) return;
    setDraggedIndex(index);
  };

  const handleDragOver = (e: React.DragEvent, targetIndex: number) => {
    e.preventDefault();
    if (draggedIndex === null || draggedIndex === targetIndex) return;

    const nextLayout = [...layoutOrder];
    const item = nextLayout.splice(draggedIndex, 1)[0];
    nextLayout.splice(targetIndex, 0, item);
    
    setDraggedIndex(targetIndex);
    setLayoutOrder(nextLayout);
  };

  const handleDragEnd = () => {
    setDraggedIndex(null);
  };

  const removeSection = (id: string) => {
    setLayoutOrder(layoutOrder.filter((mId) => mId !== id));
  };

  // 📐 Edge/Corner collision parsing mechanics
  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!isEditing) return;

    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left; 
    const y = e.clientY - rect.top;  
    const pad = 14; 

    const nearRight = x >= rect.width - pad;
    const nearBottom = y >= rect.height - pad;

    if (nearRight && nearBottom) {
      setIsResizingZone("nwse");
    } else if (nearBottom) {
      setIsResizingZone("ns");
    } else if (nearRight) {
      setIsResizingZone("ew");
    } else {
      setIsResizingZone(null); 
    }
  };

  const handleMouseLeave = () => {
    setIsResizingZone(null);
  };

  const startResizeExecution = (e: React.MouseEvent<HTMLDivElement>, id: string) => {
    if (!isResizingZone || !isEditing) return;
    
    e.preventDefault();
    e.stopPropagation();

    const startX = e.clientX;
    const startY = e.clientY;
    const startWidth = dimensions[id]?.width || e.currentTarget.clientWidth;
    const startHeight = dimensions[id]?.height || 220;
    const initialZone = isResizingZone;

    const executeResize = (moveEvent: MouseEvent) => {
      setDimensions((prev) => {
        const updated = { ...prev[id] || { height: 220 } };
        
        if (initialZone === "ns" || initialZone === "nwse") {
          updated.height = Math.max(150, Math.min(800, startHeight + (moveEvent.clientY - startY)));
        }
        if (initialZone === "ew" || initialZone === "nwse") {
          updated.width = Math.max(320, startWidth + (moveEvent.clientX - startX));
        }

        return { ...prev, [id]: updated as { width?: number; height: number } };
      });
    };

    const terminateResize = () => {
      window.removeEventListener("mousemove", executeResize);
      window.removeEventListener("mouseup", terminateResize);
    };

    window.addEventListener("mousemove", executeResize);
    window.addEventListener("mouseup", terminateResize);
  };

  const getCursorClass = () => {
    if (!isEditing) return "";
    if (isResizingZone === "nwse") return "cursor-nwse-resize";
    if (isResizingZone === "ns") return "cursor-ns-resize";
    if (isResizingZone === "ew") return "cursor-ew-resize";
    return "cursor-grab active:cursor-grabbing";
  };

  return (
    <main className="flex-1 h-full w-full p-6 overflow-y-auto select-none bg-transparent">
      {selectedTicker ? (
        <div className="w-full flex flex-col gap-6 max-w-6xl mx-auto">
          
          <div className="flex flex-col gap-6">
            {layoutOrder.map((id, index) => {
              const currentModule = componentRegistry[id];
              if (!currentModule) return null;

              const isBeingDragged = draggedIndex === index;
              const currentHeight = dimensions[id]?.height ?? 220;
              const currentWidth = dimensions[id]?.width ? `${dimensions[id].width}px` : "100%";

              return (
                <div 
                  key={id} 
                  draggable={isEditing && !isResizingZone}
                  onDragStart={() => handleDragStart(index)}
                  onDragOver={(e) => handleDragOver(e, index)}
                  onDragEnd={handleDragEnd}
                  onMouseMove={handleMouseMove}
                  onMouseLeave={handleMouseLeave}
                  onMouseDown={(e) => startResizeExecution(e, id)}
                  className={`relative rounded p-5 flex flex-col transition-shadow duration-150 overflow-hidden ${getCursorClass()} ${
                    isEditing 
                      ? "border border-dashed border-neutral-600/60 bg-neutral-500/[0.02]" 
                      : "border border-transparent"
                  } ${isBeingDragged ? "opacity-30 scale-[0.99]" : "opacity-100"}`}
                  style={{
                    height: `${currentHeight}px`,
                    width: isEditing && dimensions[id]?.width ? currentWidth : "100%",
                    maxWidth: "100%"
                  }}
                >
                  {isEditing && (
                    <div className="absolute top-3 right-3 z-50 flex items-center gap-2 font-mono text-[10px] animate-fadeIn pointer-events-none">
                      <span className={`px-2 py-1 border ${colors.border} rounded bg-neutral-900/80 backdrop-blur-sm text-[9px] opacity-60`}>
                        ⠿ DRAG OR GRAB EDGES TO SHAPE
                      </span>
                      <button
                        onClick={() => removeSection(id)}
                        className="w-6 h-6 rounded border border-red-500/30 bg-red-500 text-white flex items-center justify-center font-bold text-xs cursor-pointer shadow-md hover:bg-red-600 transition-colors pointer-events-auto"
                      >
                        ✕
                      </button>
                    </div>
                  )}

                  <div className="font-mono text-[9px] tracking-[0.2em] uppercase opacity-30 pl-1 mb-2 flex-shrink-0 pointer-events-none">
                    [ {currentModule.name} ]
                  </div>

                  <div className="w-full flex-1 min-h-0 pointer-events-none">
                    {currentModule.render(selectedTicker, colors)}
                  </div>

                  {isEditing && (
                    <div className="absolute bottom-1 right-1 p-0.5 pointer-events-none opacity-40 text-[10px] font-mono">
                      ◢
                    </div>
                  )}
                </div>
              );
            })}
          </div>

        </div>
      ) : (
        <div className="h-full w-full flex flex-col justify-center items-center text-center font-mono">
          <div className="relative flex flex-col items-center gap-2">
            <div className={`text-xs tracking-[0.2em] uppercase ${colors.textMuted} animate-pulse`}>
              // Mount a data container node to proceed
            </div>
          </div>
        </div>
      )}
    </main>
  );
}