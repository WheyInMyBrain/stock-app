// stock-app/ui/frontend/src/components/workspace/stock_chart.tsx

import React, { useEffect, useRef, useState } from "react";

interface ChartViewerProps {
  node: {
    seriesData?: any;
    [key: string]: any;
  };
  chartBg: string;
}

export function InteractiveChartViewer({ node, chartBg }: ChartViewerProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  
  // 🚀 STABLE REAL-TIME AGGREGATION: Manage candles in local state so new delta drops don't snap the layout
  const [candles, setCandles] = useState<any[]>(Array.isArray(node.seriesData) ? node.seriesData : []);
  // 🚀 STATUS TRACKER: Captures the exact moment the backend pushes fresh pipeline telemetry matrices down the websocket
  const [lastUpdatedTime, setLastUpdatedTime] = useState<string>("INITIALIZED");

  const [candleWidth, setCandleWidth] = useState<number>(8);
  const [scrollOffset, setScrollOffset] = useState<number>(0); 
  const [mousePos, setMousePos] = useState<{ x: number; y: number } | null>(null);

  const mousePosRef = useRef<{ x: number; y: number } | null>(null);
  useEffect(() => {
    mousePosRef.current = mousePos;
  }, [mousePos]);

  // 🚀 HOT-RELOAD SYNC TRACKER: Intercepts property changes from the backend pipeline
  useEffect(() => {
    if (Array.isArray(node.seriesData)) {
      setCandles(node.seriesData);
      
      // Stamp the exact current clock time of your machine when the frame is pushed
      const now = new Date();
      const timeString = now.toLocaleTimeString("en-IN", {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false
      });
      setLastUpdatedTime(`LIVE UPDATED: ${timeString}`);
    }
  }, [node.seriesData]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleNativeWheel = (e: WheelEvent) => {
      e.preventDefault();

      if (e.ctrlKey || Math.abs(e.deltaY) > 0 && Math.abs(e.deltaX) === 0) {
        const zoomIntensity = 0.05;
        const direction = e.deltaY > 0 ? -1 : 1;
        
        setCandleWidth((prevWidth) => {
          let nextWidth = prevWidth * (1 + direction * zoomIntensity);
          nextWidth = Math.max(3, Math.min(nextWidth, 50)); 

          const rect = canvasRef.current?.getBoundingClientRect();
          const currentMouse = mousePosRef.current;
          
          if (rect && currentMouse) {
            const chartWidth = rect.width - 70;
            if (currentMouse.x >= 0 && currentMouse.x <= chartWidth) {
              const mouseXFromRight = chartWidth - currentMouse.x;
              const newOffset = ((scrollOffset + mouseXFromRight) / prevWidth) * nextWidth - mouseXFromRight;
              setScrollOffset(Math.max(0, newOffset));
            }
          }
          return nextWidth;
        });
      } else {
        const horizontalVelocity = e.deltaX !== 0 ? e.deltaX : -e.deltaY;
        setScrollOffset((prev) => Math.max(0, prev + horizontalVelocity * 0.85));
      }
    };

    container.addEventListener("wheel", handleNativeWheel, { passive: false });
    return () => container.removeEventListener("wheel", handleNativeWheel);
  }, [scrollOffset]);

  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const width = container.clientWidth;
    const height = container.clientHeight;
    canvas.width = width;
    canvas.height = height;

    const ctx = canvas.getContext("2d");
    if (!ctx || candles.length === 0) return;

    ctx.clearRect(0, 0, width, height);

    const paddingRight = 70;   
    const paddingBottom = 28;  
    const chartWidth = width - paddingRight;
    const chartHeight = height - paddingBottom;

    const gapRatio = 0.22;
    const bodyWidth = Math.max(1.5, candleWidth * (1 - gapRatio));

    const totalCandles = candles.length;
    const maxVisibleCandles = Math.ceil(chartWidth / candleWidth);

    const rightmostIndex = Math.max(
      0,
      totalCandles - 1 - Math.floor(scrollOffset / candleWidth)
    );
    const leftmostIndex = Math.max(0, rightmostIndex - maxVisibleCandles);

    const visibleCandles = candles.slice(leftmostIndex, rightmostIndex + 1);
    if (visibleCandles.length === 0) return;

    let maxPrice = Math.max(...visibleCandles.map(c => c.high || 0));
    let minPrice = Math.min(...visibleCandles.map(c => c.low || Infinity));
    
    if (maxPrice === minPrice) { maxPrice += 1.0; minPrice -= 1.0; }
    
    const rangeBuffer = (maxPrice - minPrice) * 0.06;
    maxPrice += rangeBuffer;
    minPrice -= rangeBuffer;
    const priceRange = maxPrice - minPrice;

    const getXCoordinate = (arrayIndex: number) => {
      const indexFromRight = (totalCandles - 1) - arrayIndex;
      const pixelXFromRight = indexFromRight * candleWidth - scrollOffset;
      return chartWidth - pixelXFromRight - (candleWidth / 2);
    };

    const getYCoordinate = (price: number) => {
      return chartHeight - ((price - minPrice) / priceRange) * chartHeight;
    };

    // =================================================================
    // 1. DYNAMIC TRADINGVIEW GRID GENERATION (UNSPLINTERED PRICES)
    // =================================================================
    ctx.font = "10px monospace";
    ctx.lineWidth = 1;

    const targetLines = 35;
    const rawStep = priceRange / targetLines;
    
    const allowedSteps = [0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0];
    let priceStep = allowedSteps[allowedSteps.length - 1];
    for (let i = 0; i < allowedSteps.length; i++) {
      if (allowedSteps[i] >= rawStep) {
        priceStep = allowedSteps[i];
        break;
      }
    }

    const startPrice = Math.ceil(minPrice / priceStep) * priceStep;

    let currentGridPrice = startPrice;
    while (currentGridPrice <= maxPrice) {
      const gy = getYCoordinate(currentGridPrice);
      
      if (gy >= 0 && gy <= chartHeight) {
        ctx.strokeStyle = "rgba(43, 43, 43, 0.45)";
        ctx.beginPath();
        ctx.moveTo(0, gy);
        ctx.lineTo(chartWidth, gy);
        ctx.stroke();

        ctx.fillStyle = "#888888";
        ctx.textAlign = "left";
        ctx.fillText(currentGridPrice.toFixed(2), chartWidth + 8, gy + 4);
      }
      currentGridPrice += priceStep;
    }

    // =================================================================
    // 2. RENDER SHIFTING VERTICAL TIMELINES & CANDLE METRICS
    // =================================================================
    let lastTimestampX = -Infinity;
    
    let strideStep = 5;
    if (candleWidth < 4) strideStep = 30;
    else if (candleWidth < 8) strideStep = 15;
    else if (candleWidth < 15) strideStep = 10;

    for (let idx = leftmostIndex; idx <= rightmostIndex; idx++) {
      const candle = candles[idx];
      if (!candle) continue;

      const cx = getXCoordinate(idx);
      if (cx < 0 || cx > chartWidth) continue;

      if (idx % strideStep === 0) {
        if (cx - lastTimestampX > 75) {
          ctx.strokeStyle = "rgba(43, 43, 43, 0.35)";
          ctx.beginPath();
          ctx.moveTo(cx, 0);
          ctx.lineTo(cx, chartHeight);
          ctx.stroke();

          ctx.fillStyle = "#888888";
          ctx.textAlign = "center";
          const dateObj = new Date(candle.time);
          const labelStr = dateObj.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
          ctx.fillText(labelStr, cx, chartHeight + 16);
          lastTimestampX = cx;
        }
      }
    }

    for (let idx = leftmostIndex; idx <= rightmostIndex; idx++) {
      const candle = candles[idx];
      if (!candle) continue;

      const cx = getXCoordinate(idx);
      if (cx < -candleWidth || cx > chartWidth + candleWidth) continue;

      const yo = getYCoordinate(candle.open || 0);
      const yc = getYCoordinate(candle.close || 0);
      const yh = getYCoordinate(candle.high || 0);
      const yl = getYCoordinate(candle.low || 0);

      const isBullish = (candle.close || 0) >= (candle.open || 0);
      const color = isBullish ? "#10b981" : "#ef4444";

      ctx.strokeStyle = color;
      ctx.fillStyle = color;

      ctx.beginPath();
      ctx.moveTo(cx, yh);
      ctx.lineTo(cx, yl);
      ctx.stroke();

      const rx = cx - bodyWidth / 2;
      const ry = Math.min(yo, yc);
      const rh = Math.max(Math.abs(yo - yc), 1.0);
      ctx.fillRect(rx, ry, bodyWidth, rh);
    }

    // Draw solid inner pane borders over the clipping frame paths
    ctx.strokeStyle = "#1f1f1f";
    ctx.beginPath();
    ctx.moveTo(chartWidth, 0);
    ctx.lineTo(chartWidth, chartHeight);
    ctx.moveTo(0, chartHeight);
    ctx.lineTo(chartWidth, chartHeight);
    ctx.stroke();

    // =================================================================
    // 3. 🚀 HIGH-DENSITY "LAST UPDATED" TELEMETRY BADGE ON CANVAS
    // =================================================================
    ctx.save();
    ctx.font = "9px monospace";
    ctx.textAlign = "right";
    ctx.fillStyle = "rgba(163, 163, 163, 0.45)"; // Soft muted gray overlay text
    // Fixed coordinate anchor relative to your right padding gutter boundaries
    ctx.fillText(lastUpdatedTime, chartWidth - 12, chartHeight - 12);
    ctx.restore();

    // =================================================================
    // 4. CROSSHAIR TRACKING LINES
    // =================================================================
    if (mousePos && mousePos.x >= 0 && mousePos.x <= chartWidth && mousePos.y >= 0 && mousePos.y <= chartHeight) {
      ctx.strokeStyle = "rgba(140, 140, 140, 0.45)";
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);

      ctx.beginPath();
      ctx.moveTo(mousePos.x, 0);
      ctx.lineTo(mousePos.x, chartHeight);
      ctx.stroke();

      ctx.beginPath();
      ctx.moveTo(0, mousePos.y);
      ctx.lineTo(chartWidth, mousePos.y);
      ctx.stroke();
      ctx.setLineDash([]);

      const rightEdgeDistance = chartWidth - mousePos.x;
      const currentIndexOffset = Math.floor((rightEdgeDistance + scrollOffset) / candleWidth);
      const targetIndex = totalCandles - 1 - currentIndexOffset;

      const hoveredCandle = candles[targetIndex];

      if (hoveredCandle) {
        ctx.fillStyle = "#e5e5e5";
        ctx.font = "bold 11px monospace";
        ctx.textAlign = "left";
        
        const timeStr = new Date(hoveredCandle.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
        const hudText = `TIME: ${timeStr}  O: ${hoveredCandle.open.toFixed(2)}  H: ${hoveredCandle.high.toFixed(2)}  L: ${hoveredCandle.low.toFixed(2)}  C: ${hoveredCandle.close.toFixed(2)}  V: ${hoveredCandle.volume}`;
        ctx.fillText(hudText, 12, 18);

        ctx.fillStyle = "#262626";
        const hoverPrice = maxPrice - (mousePos.y / chartHeight) * priceRange;
        ctx.fillRect(chartWidth + 2, mousePos.y - 8, 64, 16);
        ctx.fillStyle = "#ffffff";
        ctx.font = "9px monospace";
        ctx.fillText(hoverPrice.toFixed(2), chartWidth + 6, mousePos.y + 3);

        ctx.fillStyle = "#262626";
        ctx.fillRect(mousePos.x - 25, chartHeight + 2, 50, 14);
        ctx.fillStyle = "#ffffff";
        ctx.textAlign = "center";
        const crosshairTimeStr = new Date(hoveredCandle.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        ctx.fillText(crosshairTimeStr, mousePos.x, chartHeight + 12);
      }
    }
  }, [candles, candleWidth, scrollOffset, mousePos, lastUpdatedTime]);

  return (
    <div 
      ref={containerRef} 
      className="w-full h-full min-h-[520px] flex-1 bg-[#060606] relative rounded-lg border border-neutral-900 overflow-hidden mt-2 select-none pointer-events-auto"
      style={{ cursor: "crosshair", userSelect: "none" }}
      onMouseDown={(e: React.MouseEvent<HTMLDivElement>) => {
        const rect = canvasRef.current?.getBoundingClientRect();
        if (rect && e.clientX - rect.left > rect.width - 70) return; 

        const startX = e.clientX;
        const startOffset = scrollOffset;

        const onMouseMoveDrag = (moveEvent: globalThis.MouseEvent) => {
          const currentDeltaX = moveEvent.clientX - startX;
          setScrollOffset(Math.max(0, startOffset - currentDeltaX));
        };

        const onMouseUpClose = () => {
          window.removeEventListener("mousemove", onMouseMoveDrag);
          window.removeEventListener("mouseup", onMouseUpClose);
        };

        window.addEventListener("mousemove", onMouseMoveDrag);
        window.addEventListener("mouseup", onMouseUpClose);
      }}
      onMouseMove={(e: React.MouseEvent<HTMLDivElement>) => {
        const rect = canvasRef.current?.getBoundingClientRect();
        if (rect) {
          setMousePos({ x: e.clientX - rect.left, y: e.clientY - rect.top });
        }
      }}
      onMouseLeave={() => setMousePos(null)}
    >
      <canvas ref={canvasRef} className="absolute inset-0 w-full h-full" />
    </div>
  );
}

if (typeof window !== "undefined") {
  (window as any).InteractiveChartViewerComponent = InteractiveChartViewer;
}