import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UiPrimitiveNode } from "./PrimitiveCompiler";

export interface ServerModuleConfig {
  id: string;
  title: string;
  height?: number;
  width?: number;
  root_node: UiPrimitiveNode;
}

export function useWorkspaceLayout(selectedTicker: string | null, isEditing: boolean, resetSignal: number) {
  const [activeModules, setActiveModules] = useState<ServerModuleConfig[]>([]);
  const wasEditingRef = useRef(isEditing);

  const loadLayoutForTicker = (tickerStr: string) => {
    invoke<string>("load_workspace_layout")
      .then(async (rawJson) => {
        let orderMap: string[] = ["overview_metrics", "performance_chart"];
        let savedDims: Record<string, { width?: number; height: number }> = {};

        if (rawJson && rawJson !== "{}" && rawJson.trim() !== "") {
          const parsed = JSON.parse(rawJson);
          if (parsed.order) orderMap = parsed.order;
          if (parsed.dims) savedDims = parsed.dims;
        }

        const compileTasks = orderMap.map(async (moduleId) => {
          try {
            const res: any = await invoke("fetch_component_telemetry", { ticker: tickerStr, moduleId });
            return {
              id: res.id,
              title: res.title,
              height: savedDims[res.id]?.height ?? 220,
              width: savedDims[res.id]?.width,
              root_node: res.root_node,
            };
          } catch {
            return null;
          }
        });

        setActiveModules((await Promise.all(compileTasks)).filter(Boolean) as ServerModuleConfig[]);
      })
      .catch((err) => console.error("Pipeline recovery failed:", err));
  };

  useEffect(() => {
    if (selectedTicker) loadLayoutForTicker(selectedTicker);
  }, [selectedTicker]);

  useEffect(() => {
    if (wasEditingRef.current && !isEditing && activeModules.length > 0) {
      const order = activeModules.map((m) => m.id);
      const dims: Record<string, any> = {};
      activeModules.forEach((m) => {
        dims[m.id] = { height: m.height, width: m.width };
      });
      invoke("save_workspace_layout", { layoutJson: JSON.stringify({ order, dims }) }).catch((err) =>
        console.error(err)
      );
    }
    wasEditingRef.current = isEditing;
  }, [isEditing, activeModules]);

  useEffect(() => {
    if (resetSignal > 0 && selectedTicker) {
      invoke("save_workspace_layout", { layoutJson: JSON.stringify({ order: ["overview_metrics", "performance_chart"], dims: {} }) })
        .then(() => loadLayoutForTicker(selectedTicker));
    }
  }, [resetSignal]);

  const appendCatalogItemToGrid = async (moduleId: string) => {
    if (!selectedTicker || activeModules.some((m) => m.id === moduleId)) return;
    try {
      const res: any = await invoke("fetch_component_telemetry", { ticker: selectedTicker, moduleId });
      setActiveModules((prev) => [...prev, { id: res.id, title: res.title, height: 220, root_node: res.root_node }]);
    } catch (err) {
      console.error("Failed to append dynamic block:", err);
    }
  };

  const removeModule = (targetId: string) => {
    setActiveModules((prev) => prev.filter((m) => m.id !== targetId));
  };

  return { activeModules, setActiveModules, appendCatalogItemToGrid, removeModule };
}