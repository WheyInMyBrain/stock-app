import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CatalogItem {
  id: string;
  name: string;
  description: string;
}

interface CustomizationPanelProps {
  isOpen: boolean;
  activeModuleIds: string[];
  onAddModule: (id: string) => void;
  colors: any;
}

export default function CustomizationPanel({ isOpen, activeModuleIds, onAddModule, colors }: CustomizationPanelProps) {
  const [catalog, setCatalog] = useState<CatalogItem[]>([]);
  const cardBg = colors.input.includes("bg-[#0A0A0A]") ? "bg-[#121212]" : "bg-[#F4F4F5]";

  useEffect(() => {
    if (isOpen) {
      invoke<CatalogItem[]>("fetch_component_catalog")
        .then(setCatalog)
        .catch((err) => console.error("Catalog retrieval failed:", err));
    }
  }, [isOpen]);

  return (
    <aside
      className={`h-full border-l ${colors.border} ${colors.bgSidebar} flex flex-col transition-all duration-300 overflow-hidden font-sans select-none flex-shrink-0 z-40 ${
        isOpen ? "w-80 opacity-100" : "w-0 opacity-0 border-l-transparent pointer-events-none"
      }`}
    >
      <div className={`h-16 border-b ${colors.border} px-6 flex items-center justify-between`}>
        <span className="text-xs font-semibold tracking-wider uppercase opacity-60">
          // COMPONENTS VAULT
        </span>
      </div>

      <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-4">
        {catalog.map((item) => {
          const isAlreadyPresent = activeModuleIds.includes(item.id);

          return (
            <div
              key={item.id}
              /* 🎯 FIXED: Replaced standard browser drag listener with mouse down interaction trackers.
                 When held down, it immediately initializes the custom injection routine onto your workspace layout canvas coordinates, matching the precision gravity math we just built. */
              onMouseDown={(e) => {
                if (isAlreadyPresent) return;
                e.preventDefault();
                
                // Immediately drop the component item straight onto the active grid workspace list array
                onAddModule(item.id);

                // Use a microscopic timeout frame pool to let the card mount into the DOM,
                // then find it and instantly transfer mouse tracking over to the high-speed drag handler engine!
                setTimeout(() => {
                  const targetCardElement = document.querySelector(`[data-module-card="${item.id}"]`) as HTMLElement;
                  if (targetCardElement) {
                    // Find our drag handle header or center matrix zone and simulate a fresh mouse click grab
                    const simulatedEvent = new MouseEvent("mousedown", {
                      bubbles: true,
                      cancelable: true,
                      clientX: e.clientX,
                      clientY: e.clientY
                    });
                    targetCardElement.dispatchEvent(simulatedEvent);
                  }
                }, 30);
              }}
              className={`p-4 border rounded-xl flex flex-col justify-between text-left transition-all duration-200 group relative ${
                isAlreadyPresent
                  ? "border-neutral-800 opacity-25 cursor-not-allowed bg-neutral-900/[0.05]"
                  : `${colors.border} ${cardBg} border-solid shadow-sm hover:border-neutral-500/50 cursor-grab active:cursor-grabbing`
              }`}
            >
              <div className="flex flex-col gap-1">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-semibold tracking-tight">{item.name}</span>
                  {!isAlreadyPresent && (
                    <span className="text-[10px] opacity-40 px-1.5 py-0.5 rounded border border-neutral-500/20 font-mono">
                      DRAG OUT
                    </span>
                  )}
                </div>
                <span className="text-xs opacity-50 leading-relaxed font-normal mt-1">{item.description}</span>
              </div>

              {/* Bottom tag block with fine line borders */}
              <div className="mt-4 pt-2.5 border-t border-neutral-700/25 flex items-center justify-between text-[10px] uppercase font-medium opacity-40 font-mono tracking-tight">
                <span>[ MODULE IDENTIFIER ]</span>
                <span>{item.id}</span>
              </div>
            </div>
          );
        })}
      </div>
    </aside>
  );
}