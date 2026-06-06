import React, { useState, useRef } from "react";

// 📊 Internal, fully contained TypeScript definitions for local documentation
interface ChartSeries {
  key: string;         // e.g., "price", "var99"
  label: string;       // e.g., "Spot Price"
  stroke: string;      // e.g., "#ef4444"
  strokeWidth?: number;
  strokeDash?: string;
}

export function InteractiveChart({ node }: { node: any }) {
  const data: Record<string, any>[] = node?.data || [];
  const seriesList: ChartSeries[] = node?.series || [];
  const xAxisKey: string = node?.xAxisKey || "date";
  
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const [tooltipPos, setTooltipPos] = useState({ x: 0, y: 0 });
  const containerRef = useRef<HTMLDivElement>(null);

  if (data.length === 0 || seriesList.length === 0) {
    return (
      <div className="text-neutral-500 p-4 text-xs font-mono border border-neutral-900 bg-neutral-950/20 rounded">
        Telemetry Stream Incomplete: Empty chart matrix fields.
      </div>
    );
  }

  // 📐 Local Canvas Boundaries Configuration
  const canvasW = node?.viewBoxWidth || 500;
  const canvasH = node?.viewBoxHeight || 160;

  // 📊 SCALE CALCULATION: Scans all series attributes dynamically using standard JS floating limits
  let globalMax = -Infinity;
  let globalMin = Infinity;

  data.forEach((row) => {
    seriesList.forEach((series) => {
      const val = parseFloat(row[series.key]);
      if (!isNaN(val) && isFinite(val)) {
        if (val > globalMax) globalMax = val;
        if (val < globalMin) globalMin = val;
      }
    });
  });

  // Safe checks for completely flat data or parsing failures
  if (globalMax === -Infinity) globalMax = 100.0;
  if (globalMin === Infinity) globalMin = 0.0;

  const valueRange = globalMax - globalMin;
  const highBuffer = globalMax + (valueRange * 0.03);
  const lowBuffer = Math.max(0.0, globalMin - (valueRange * 0.03));
  const activeRange = highBuffer - lowBuffer === 0 ? 1.0 : highBuffer - lowBuffer;

  // 📊 COORDINATE MATRIX TRANSFORMATIONS
  const mappedPoints = data.map((row, idx) => {
    const x = data.length > 1 ? (idx / (data.length - 1)) * canvasW : canvasW / 2;
    
    const yPositions: Record<string, number> = {};
    seriesList.forEach((series) => {
      const val = parseFloat(row[series.key]) || 0.0;
      yPositions[series.key] = canvasH - ((val - lowBuffer) / activeRange) * canvasH;
    });

    return { x, yPositions, rawRow: row };
  });

  // 🎯 CURSOR INTERSECTION HANDLER
  const handleMouseMove = (e: React.MouseEvent<SVGSVGElement>) => {
    if (!containerRef.current) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    
    const percentageX = Math.max(0, Math.min(1, mouseX / rect.width));
    const targetIndex = Math.round(percentageX * (data.length - 1));

    if (targetIndex >= 0 && targetIndex < data.length) {
      setActiveIndex(targetIndex);
      
      // Bounding protection: flips tooltip box to left side if cursor nears the right edge
      const tooltipX = percentageX > 0.72 ? mouseX - 165 : mouseX + 15;
      setTooltipPos({ x: tooltipX, y: e.clientY - rect.top - 15 });
    }
  };

  const activePoint = activeIndex !== null ? mappedPoints[activeIndex] : null;

  return (
    <div ref={containerRef} className="w-full flex flex-col pt-2 relative select-none transform-gpu [will-change:transform] backface-hidden">
      
      {/* 🚀 RESPONSIVE FLOATING TOOLTIP BOX */}
      {activePoint && (
        <div
          className="absolute z-50 bg-neutral-900/95 border border-neutral-800 p-2.5 rounded shadow-xl text-[10px] font-mono leading-normal pointer-events-none backdrop-blur-sm min-w-[150px] transition-all duration-75 ease-out"
          style={{ left: `${tooltipPos.x}px`, top: `${tooltipPos.y}px` }}
        >
          <div className="text-neutral-400 font-bold border-b border-neutral-800 pb-1 mb-1.5">
            {String(activePoint.rawRow[xAxisKey])}
          </div>
          <div className="flex flex-col gap-1">
            {seriesList.map((series) => {
              const rawValue = parseFloat(activePoint.rawRow[series.key]) || 0.0;
              return (
                <div key={series.key} className="flex justify-between items-center gap-4">
                  <span className="text-neutral-400 flex items-center gap-1.5">
                    <span className="w-1.5 h-1.5 rounded-full inline-block" style={{ backgroundColor: series.stroke }} />
                    {series.label}:
                  </span>
                  <span className="text-neutral-100 font-bold">
                    {rawValue % 1 === 0 ? rawValue.toLocaleString() : rawValue.toFixed(2)}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* 📈 ACTIVE SVG CHART LAYER */}
      <svg
        viewBox={`0 0 ${canvasW} ${canvasH}`}
        className="w-full h-full overflow-visible cursor-crosshair"
        onMouseMove={handleMouseMove}
        onMouseLeave={() => setActiveIndex(null)}
      >
        {/* Horizontal Background Alignment Wireframes */}
        {[0.25, 0.5, 0.75].map((pct, idx) => (
          <line key={idx} x1="0" y1={canvasH * pct} x2={canvasW} y2={canvasH * pct} stroke="#1f2937" strokeDasharray="2,4" strokeWidth="0.75" />
        ))}

        {/* Path Generation Matrix */}
        {seriesList.map((series) => {
          const pathString = mappedPoints
            .map((pt, idx) => `${idx === 0 ? "M" : "L"} ${pt.x.toFixed(1)} ${pt.yPositions[series.key].toFixed(1)}`)
            .join(" ");

          return (
            <path
              key={series.key}
              d={pathString}
              fill="none"
              stroke={series.stroke}
              strokeWidth={series.strokeWidth || 1.25}
              strokeDasharray={series.strokeDash}
              strokeLinecap="round"
              strokeLinejoin="round"
              className="transition-all duration-300 opacity-80"
            />
          );
        })}

        {/* Interactive Highlight Tracking Rings */}
        {activePoint && (
          <g>
            <line x1={activePoint.x} y1="0" x2={activePoint.x} y2={canvasH} stroke="#4b5563" strokeWidth="1" strokeDasharray="3,3" />
            {seriesList.map((series) => (
              <circle
                key={series.key}
                cx={activePoint.x}
                cy={activePoint.yPositions[series.key]}
                r="3"
                fill={series.stroke}
                stroke="#0a0a0a"
                strokeWidth="1"
              />
            ))}
          </g>
        )}
      </svg>
    </div>
  );
}