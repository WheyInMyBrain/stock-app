import React, { useState, useEffect, useRef } from "react";
import { InteractiveChartViewer } from "./stock_chart";
import { TableComponent, TableRowComponent } from "./tables";

export interface UiPrimitiveNode {
  type: "grid" | "card" | "metric" | "bar_graph" | "text" | "container" | "select" | "vector_canvas" | "vector_path" | "vector_rect" | "popup_workspace" | "chart_viewer" | "table" | "table_row";
  className?: string;
  style?: React.CSSProperties;
  children?: UiPrimitiveNode[];
  title?: string;
  subtitle?: string;
  footer?: string;
  value?: string;
  status?: "up" | "down" | "neutral";
  percentage_height?: number;
  color_token?: string;
  options?: string[];
  default_value?: string;
  action_target?: string;
  d?: string;
  fill?: string;
  stroke?: string;
  stroke_width?: number;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  headers?: string[];
  cells?: UiPrimitiveNode[];
  tooltip?: string;
}

interface PrimitiveCompilerProps {
  node: UiPrimitiveNode;
  colors: {
    border?: string;
    text?: string;
    textMuted?: string;
    inputBg?: string;
    [key: string]: any;
  };
  cardBg: string;
  keyIndex?: number;
}

export default function PrimitiveCompiler({ node, colors, cardBg, keyIndex = 0 }: PrimitiveCompilerProps) {
  if (!node) return null;

  const nestedChildren = node.children?.map((childNode, index) => (
    <PrimitiveCompiler 
      key={index} 
      node={childNode} 
      colors={colors} 
      cardBg={cardBg} 
      keyIndex={index} 
    />
  ));

  const activeBorder = colors?.border || "";
  const activeText = colors?.text || "";
  const activeTextMuted = colors?.textMuted || "opacity-50";

  switch (node.type) {
    case "container": {
      const isGrid = node.className?.includes("grid") || false;
      return (
        <div 
          key={keyIndex} 
          className={`w-full min-w-0 ${node.className || ""}`}
          style={{
            display: "grid",
            gridTemplateColumns: isGrid ? undefined : "repeat(auto-fit, minmax(150px, 1fr))", 
            gap: "1.5rem",
            alignItems: "start",
            ...node.style
          }}
        >
          {nestedChildren}
        </div>
      );
    }

    case "grid":
      return (
        <div key={keyIndex} className={`w-full h-full grid grid-cols-1 md:grid-cols-3 gap-4 text-left ${node.className || ""}`}>
          {nestedChildren}
        </div>
      );

    case "card": {
      const isTailwindClass = !cardBg.startsWith("#") && !cardBg.startsWith("rgb");
      return (
        <div 
          key={keyIndex} 
          className={`p-5 border rounded-lg ${activeBorder} ${isTailwindClass ? cardBg : ""} ${activeText} w-full h-full flex flex-col justify-between overflow-hidden font-sans tracking-normal text-sm ${node.className || ""}`}
          style={{ 
            backgroundColor: isTailwindClass ? undefined : cardBg,
            ...node.style 
          }}
        >
          {(node.title || node.subtitle) && (
            <div className="flex-shrink-0">
              {node.subtitle && <div className={`text-[11px] font-medium tracking-wide uppercase mb-1 ${activeTextMuted}`}>{node.subtitle}</div>}
              {node.title && <div className="text-lg font-semibold tracking-tight leading-snug">{node.title}</div>}
            </div>
          )}
          {nestedChildren}
          {node.footer && <div className={`text-xs opacity-40 mt-4 border-t pt-2.5 ${activeBorder}`}>{node.footer}</div>}
        </div>
      );
    }

    case "metric": {
      const [flashClass, setFlashClass] = useState<string>("");
      const previousValueRef = useRef<string | undefined>(node.value);

      useEffect(() => {
        if (previousValueRef.current !== node.value && previousValueRef.current !== undefined) {
          const prevNum = parseFloat(previousValueRef.current.replace(/[^0-9.-]/g, ""));
          const nextNum = parseFloat((node.value || "").replace(/[^0-9.-]/g, ""));

          if (!isNaN(prevNum) && !isNaN(nextNum)) {
            if (nextNum > prevNum) {
              setFlashClass("bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 p-1 rounded transition-all scale-[1.01]");
            } else if (nextNum < prevNum) {
              setFlashClass("bg-red-500/10 text-red-600 dark:text-red-400 p-1 rounded transition-all scale-[1.01]");
            }
            const timer = setTimeout(() => setFlashClass(""), 1000);
            return () => clearTimeout(timer);
          }
        }
        previousValueRef.current = node.value;
      }, [node.value]);

      return (
        <div key={keyIndex} className="my-auto flex flex-col gap-0.5 font-sans">
          <div className={`text-2xl font-semibold font-mono tracking-tight transition-all duration-300 ${flashClass ? flashClass : activeText}`}>
            {node.value}
          </div>
          {node.title && <div className={`text-[11px] font-medium uppercase tracking-wider ${activeTextMuted}`}>{node.title}</div>}
        </div>
      );
    }

    case "text":
      return (
        <div 
          key={keyIndex} 
          className={`text-xs leading-relaxed font-sans transition-colors duration-200 ${activeText} ${activeBorder} ${node.className || ""}`}
          style={node.style}
        >
          {node.value}
        </div>
      );

    case "bar_graph":
      return (
        <div key={keyIndex} className="w-full flex-1 min-h-[40px] flex items-end gap-1.5 mt-4 pt-4">
          {node.children?.map((bar, barIdx) => {
            const h = bar.percentage_height ?? 10;
            const opacity = bar.color_token === "primary" ? "opacity-90" : bar.color_token === "accent" ? "opacity-50" : "opacity-20";
            return (
              <div 
                key={barIdx} 
                className={`bg-current ${opacity} w-full rounded-t-sm transition-all duration-300`} 
                style={{ height: `${h}%` }} 
                title={`Value point metrics: ${h}%`} 
              />
            );
          })}
        </div>
      );

    case "select":
      return (
        <select
          key={keyIndex}
          defaultValue={node.default_value || ""}
          onChange={(e) => {
            window.dispatchEvent(new CustomEvent("WORKSPACE_TIMEFRAME_OVERRIDE", {
              detail: { moduleId: node.action_target, timeframe: e.target.value }
            }));
          }}
          className={`text-xs px-2 py-1 rounded border outline-none cursor-pointer font-mono font-medium transition-colors duration-200 pointer-events-auto ${activeBorder} ${activeText}`}
          style={{ backgroundColor: cardBg }}
        >
          {node.options?.map((opt: string) => (
            <option key={opt} value={opt} className="bg-white dark:bg-neutral-950 text-neutral-900 dark:text-neutral-200">
              {opt}
            </option>
          ))}
        </select>
      );

    case "vector_canvas":
      return (
        <div key={keyIndex} className="w-full flex-1 flex flex-col justify-end pt-4 min-h-[140px]">
          <svg viewBox="0 0 500 180" className="w-full h-full overflow-visible">
            {nestedChildren}
          </svg>
        </div>
      );

    case "vector_path":
      return (
        <path key={keyIndex} d={node.d} fill={node.fill || "none"} stroke={node.stroke} strokeWidth={node.stroke_width || 1} strokeLinecap="round" strokeLinejoin="round" className={node.className}>
          {node.tooltip && <title>{node.tooltip}</title>}
        </path>
      );

    case "vector_rect":
      return (
        <rect key={keyIndex} x={node.x} y={node.y} width={node.width} height={node.height} fill={node.fill} className={node.className}>
          {node.tooltip && <title>{node.tooltip}</title>}
        </rect>
      );

    case "popup_workspace":
      return (
        <div key={keyIndex} className="w-full h-full flex flex-col gap-4 animate-fadeIn" style={node.style}>
          {nestedChildren}
        </div>
      );

    case "chart_viewer":
      return <InteractiveChartViewer key={keyIndex} node={node as any} chartBg={cardBg} />;

    // 🚀 DELEGATE TO THE SPECIALIZED TABLE SUB-COMPONENTS
    case "table":
      return <TableComponent node={node} colors={colors} cardBg={cardBg} keyIndex={keyIndex} />;

    case "table_row":
      return <TableRowComponent node={node} colors={colors} cardBg={cardBg} keyIndex={keyIndex} />;

    default:
      return null;
  }
}