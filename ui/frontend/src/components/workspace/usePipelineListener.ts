import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface PipelineInvalidatedPayload {
  module_id: string;
  ticker: string;
}

export function usePipelineListener(
  currentModuleId: string,
  currentTicker: string,
  onRefreshTriggered: () => void // 🎯 THE FIX: Pass a clean execution callback straight from the card!
) {
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    const setupListener = async () => {
      const unlisten = await listen<PipelineInvalidatedPayload>('pipeline-invalidated', (event) => {
        const { module_id, ticker } = event.payload;

        if (
          module_id === currentModuleId && 
          ticker.toUpperCase() === currentTicker.toUpperCase()
        ) {
          console.log(`📡 [LIVE UPDATE]: Card [${currentModuleId}] caught dedicated data update signature for [${ticker.toUpperCase()}].`);
          
          // 🎯 FIRE REFRESH DIRECTLY
          onRefreshTriggered();
        }
      });

      unlistenFn = unlisten;
    };

    setupListener();

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [currentModuleId, currentTicker, onRefreshTriggered]);
}