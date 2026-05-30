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
  currentTicker: string,
  onRefreshNeeded: () => void
) {
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    const setupListener = async () => {
      // Subscribe natively to Tauri's global micro-message bridge
      const unlisten = await listen<PipelineInvalidatedPayload>('pipeline-invalidated', (event) => {
        const { module_id, ticker } = event.payload;

        // 🎯 EXACT CHECKPOINT EVALUATION
        // Only trigger a re-compile if the background update event 
        // matches this specific card ID and active tracking ticker token
        if (
          module_id === currentModuleId && 
          ticker.toUpperCase() === currentTicker.toUpperCase()
        ) {
          console.log(`🔄 [PIPELINE WATCHER]: Module '${module_id}' changed on disk. Re-fetching compile trees...`);
          onRefreshNeeded();
        }
      });

      unlistenFn = unlisten;
    };

    setupListener();

    // 🧼 Clean cleanup lifecycle prevents memory leaks on view changes
    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [currentModuleId, currentTicker, onRefreshNeeded]);
}