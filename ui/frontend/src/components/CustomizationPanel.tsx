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
              draggable={!isAlreadyPresent}
              onDragStart={(e) => {
                e.dataTransfer.setData("text/plain", item.id);
                e.dataTransfer.effectAllowed = "copy";
              }}
              className={`p-4 border rounded-xl flex flex-col justify-between text-left transition-all duration-200 group relative ${
                isAlreadyPresent
                  ? "border-neutral-800 opacity-25 cursor-not-allowed bg-neutral-900/[0.05]"
                  : `${colors.border} ${cardBg} border-solid shadow-sm hover:border-neutral-500/50 cursor-grab active:cursor-grabbing`
              }`}
            >
              <div className="flex flex-col gap-1">
                <span className="text-sm font-semibold tracking-tight">{item.name}</span>
                <span className="text-xs opacity-50 leading-relaxed font-normal">{item.description}</span>
              </div>

              {/* Bottom tag block with fine line borders as requested */}
              <div className="mt-4 pt-2.5 border-t border-neutral-700/25 flex items-center justify-between text-[10px] uppercase font-medium opacity-40 font-mono tracking-tight">
                <span>[ MODULE IDENTIFIER ]</span>
                <span>{item.id}</span>
              </div>

              {!isAlreadyPresent && (
                <button
                  onClick={() => onAddModule(item.id)}
                  className="absolute top-3 right-3 w-5 h-5 rounded-md border border-neutral-500/30 bg-neutral-800 text-white text-[10px] items-center justify-center hidden group-hover:flex hover:bg-neutral-700 active:scale-95 transition-all cursor-pointer"
                >
                  ＋
                </button>
              )}
            </div>
          );
        })}
      </div>
    </aside>
  );
}