import React from "react";

export interface UiPrimitiveNode {
  type: "grid" | "card" | "metric" | "bar_graph" | "text" | "container" | "select" | "vector_canvas" | "vector_path" | "vector_rect";
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
}

interface PrimitiveCompilerProps {
  node: UiPrimitiveNode;
  colors: any;
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

  switch (node.type) {
    case "container": {
      // Determine if the backend explicitly declared a custom grid structure
      const isGrid = node.className?.includes("grid") || false;
      
      return (
        /* 🎯 FIXED: Deepened backgrounds, added elegant contour borders, internal padding,
           and rounded corners to build a stunning, professional dark terminal visual aesthetic. */
        <div 
          key={keyIndex} 
          className={`w-full min-w-0 bg-neutral-950/40 border border-neutral-900 rounded-xl p-4 shadow-sm ${node.className || ""}`}
          style={{
            display: "grid",
            gridTemplateColumns: isGrid 
              ? undefined // Respect backend templates if explicitly forced
              : "repeat(auto-fit, minmax(150px, 1fr))", 
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
        <div key={keyIndex} className={`w-full grid gap-6 ${node.className || "grid-cols-1 md:grid-cols-3"}`}>
          {nestedChildren}
        </div>
      );

    case "card":
      return (
        /* 🎯 FIXED: Clean terminal slate surface with zero forced horizontal layout bounds */
        <div key={keyIndex} className={`p-6 border border-neutral-800 bg-[#0A0A0B] rounded-xl w-full h-full flex flex-col justify-between font-sans select-text ${node.className || ""}`}>
          {(node.title || node.subtitle) && (
            <div className="flex-shrink-0 mb-5 border-b border-neutral-900 pb-3">
              {node.subtitle && <div className="text-[10px] font-bold tracking-widest uppercase mb-1 text-neutral-500 font-mono">{node.subtitle}</div>}
              {node.title && <div className="text-base font-bold tracking-tight text-neutral-100">{node.title}</div>}
            </div>
          )}
          
          <div className="flex-1 w-full min-w-0 flex flex-col gap-6">
            {nestedChildren}
          </div>
          
          {node.footer && <div className="text-[11px] font-mono text-neutral-500 mt-5 border-t pt-3 border-neutral-900 flex-shrink-0 tracking-wide">{node.footer}</div>}
        </div>
      );

    case "metric":
      return (
        /* 🎯 FIXED: Clean, high-contrast metric element blocks */
        <div key={keyIndex} className="flex flex-col gap-1.5 font-sans min-w-0 py-1">
          <div className="text-xl font-bold font-mono tracking-tight text-white">{node.value}</div>
          {node.title && (
            <div className="text-[11px] font-semibold uppercase tracking-wider text-neutral-400 leading-snug">
              {node.title}
            </div>
          )}
        </div>
      );

    case "text":
      return (
        <div key={keyIndex} className={`text-xs leading-relaxed text-neutral-300 font-sans ${node.className || ""}`}>
          {node.value}
        </div>
      );

    case "bar_graph":
      return (
        <div key={keyIndex} className="w-full flex-1 min-h-[60px] flex items-end gap-2 mt-2">
          {node.children?.map((bar, barIdx) => {
            const h = bar.percentage_height ?? 10;
            const barBgColor = bar.color_token === "primary" ? "bg-emerald-500" : bar.color_token === "accent" ? "bg-blue-500" : "bg-neutral-700";
            return (
              <div 
                key={barIdx} 
                className={`${barBgColor} w-full rounded-t-sm transition-all duration-300`} 
                style={{ height: `${h}%` }} 
                title={`Value: ${h}%`} 
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
          className="text-xs px-2.5 py-1.5 rounded-md border border-neutral-800 bg-neutral-900 text-neutral-200 font-mono font-medium outline-none cursor-pointer hover:border-neutral-700 transition-all"
        >
          {node.options?.map((opt: string) => (
            <option key={opt} value={opt} className="bg-neutral-950 text-neutral-200">{opt}</option>
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
      return <path key={keyIndex} d={node.d} fill={node.fill || "none"} stroke={node.stroke} strokeWidth={node.stroke_width || 1} strokeLinecap="round" strokeLinejoin="round" className={node.className} />;

    case "vector_rect":
      return <rect key={keyIndex} x={node.x} y={node.y} width={node.width} height={node.height} fill={node.fill} className={node.className} />;

    default:
      return null;
  }
}