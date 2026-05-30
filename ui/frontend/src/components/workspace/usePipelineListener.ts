// stock-app/ui/frontend/src/components/workspace/usePipelineListener.ts

import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface PipelineInvalidatedPayload {
  module_id: string;
  ticker: string;
}

/**
 * Custom hook to subscribe a workspace layout engine card 
 * directly to the backend background updater daemon signals.
 */
export function usePipelineListener(
  currentModuleId: string,
  currentTicker: string
) {
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    const setupListener = async () => {
      // Subscribe natively to Tauri's global micro-message bridge
      const unlisten = await listen<PipelineInvalidatedPayload>('pipeline-invalidated', (event) => {
        const { module_id, ticker } = event.payload;

        // 🎯 EXACT CHECKPOINT EVALUATION
        // Block mismatching ticker tokens completely from triggering layout flashes
        if (
          module_id === currentModuleId && 
          ticker.toUpperCase() === currentTicker.toUpperCase()
        ) {
          console.log(`📡 [LIVE UPDATE]: Card [${currentModuleId}] caught dedicated ticker data change for [${ticker.toUpperCase()}].`);
          
          window.dispatchEvent(
            new CustomEvent("HOT_RELOAD_MODULE_PIPELINE", {
              detail: { moduleId: currentModuleId }
            })
          );
        }
      });

      unlistenFn = unlisten;
    };

    setupListener();

    // 🧼 Clean cleanup lifecycle prevents event listener duplication and memory memory leaks
    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [currentModuleId, currentTicker]);
}