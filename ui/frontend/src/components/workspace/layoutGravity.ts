import type { ServerModuleConfig } from "./useWorkspaceLayout";

// 🎯 DYNAMIC BIDIRECTIONAL OMNI-COMPACTION ENGINE: Prioritizes horizontal sliding before cascading items downward
export function compactLayoutWithGravity(items: ServerModuleConfig[], movingId: string | null): ServerModuleConfig[] {
  const sorted = [...items].sort((a, b) => a.y - b.y || a.x - b.x);
  const layoutMap: Record<string, ServerModuleConfig> = {};
  sorted.forEach(m => { layoutMap[m.id] = { ...m }; });

  let itemsChanged = true;
  let loopsCounter = 0;

  while (itemsChanged && loopsCounter < 150) {
    itemsChanged = false;
    loopsCounter++;

    for (let i = 0; i < sorted.length; i++) {
      const current = layoutMap[sorted[i].id];
      if (current.id === movingId) continue;

      // 1. 🔺 VERTICAL UPWARD COMPACTION PASS (Top Gravity)
      if (current.y > 0) {
        let canFloatUp = true;
        const proposedY = current.y - 1;

        for (let j = 0; j < sorted.length; j++) {
          if (sorted[j].id === current.id) continue;
          const other = layoutMap[sorted[j].id];

          if (
            current.x < other.x + other.w &&
            current.x + current.w > other.x &&
            proposedY < other.y + other.h &&
            proposedY + current.h > other.y
          ) {
            canFloatUp = false;
            break;
          }
        }

        if (canFloatUp) {
          current.y = proposedY;
          itemsChanged = true;
        }
      }

      // 2. ◀️ HORIZONTAL LEFTWARD COMPACTION PASS (Left Gravity)
      if (current.x > 0) {
        let canFloatLeft = true;
        const proposedX = current.x - 1;

        for (let j = 0; j < sorted.length; j++) {
          if (sorted[j].id === current.id) continue;
          const other = layoutMap[sorted[j].id];

          if (
            proposedX < other.x + other.w &&
            proposedX + current.w > other.x &&
            current.y < other.y + other.h &&
            current.y + current.h > other.y
          ) {
            canFloatLeft = false;
            break;
          }
        }

        if (canFloatLeft) {
          current.x = proposedX;
          itemsChanged = true;
        }
      }

      // 3. 🗺️ SMART HORIZONTAL-FIRST COLLISION RESOLUTION
      for (let j = 0; j < sorted.length; j++) {
        if (sorted[j].id === current.id) continue;
        const other = layoutMap[sorted[j].id];

        const collides = (
          current.x < other.x + other.w &&
          current.x + current.w > other.x &&
          current.y < other.y + other.h &&
          current.y + current.h > other.y
        );

        if (collides) {
          itemsChanged = true;

          const canSlideNeighborRight = (other.x >= current.x) && (other.x + other.w + 1 <= 12);

          if (current.id === movingId) {
            const cameFromLeft = current.x <= other.x;
            
            if (cameFromLeft && canSlideNeighborRight) {
              other.x = Math.max(other.x + 1, current.x + current.w);
            } else {
              other.y = current.y + current.h;
            }
          } else if (other.id === movingId) {
            const cameFromLeft = other.x <= current.x;
            const canSlideCurrentRight = (current.x >= other.x) && (current.x + current.w + 1 <= 12);

            if (cameFromLeft && canSlideCurrentRight) {
              current.x = Math.max(current.x + 1, other.x + other.w);
            } else {
              current.y = other.y + other.h;
            }
          } else {
            if (current.x + current.w <= 12 && canSlideNeighborRight) {
              other.x = current.x + current.w;
            } else if (current.y <= other.y) {
              other.y = current.y + current.h;
            } else {
              current.y = other.y + other.h;
            }
          }
        }
      }
    }
  }

  return sorted.map(item => layoutMap[item.id]);
}