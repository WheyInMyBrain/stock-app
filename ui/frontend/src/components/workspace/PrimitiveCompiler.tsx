import React from "react";

export interface UiPrimitiveNode {
  type: "grid" | "card" | "metric" | "bar_graph" | "text" | "container";
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
    case "container":
      return (
        <div key={keyIndex} className={`w-full h-full flex flex-col justify-between ${node.className || ""}`}>
          {nestedChildren}
        </div>
      );

    case "grid":
      return (
        <div key={keyIndex} className={`w-full h-full grid grid-cols-1 md:grid-cols-3 gap-4 text-left ${node.className || ""}`}>
          {nestedChildren}
        </div>
      );

    case "card":
      return (
        <div key={keyIndex} className={`p-5 border rounded-lg ${colors.border} ${cardBg} w-full h-full flex flex-col justify-between overflow-hidden font-sans tracking-normal text-sm ${node.className || ""}`}>
          {(node.title || node.subtitle) && (
            <div className="flex-shrink-0">
              {node.subtitle && <div className={`text-[11px] font-medium tracking-wide uppercase mb-1 opacity-50 ${colors.textMuted}`}>{node.subtitle}</div>}
              {node.title && <div className="text-lg font-semibold tracking-tight leading-snug">{node.title}</div>}
            </div>
          )}
          {nestedChildren}
          {node.footer && <div className="text-xs opacity-40 mt-4 border-t pt-2.5 border-neutral-700/20">{node.footer}</div>}
        </div>
      );

    case "metric":
      return (
        <div key={keyIndex} className="my-auto flex flex-col gap-0.5 font-sans">
          <div className="text-2xl font-semibold font-mono tracking-tight">{node.value}</div>
          {node.title && <div className={`text-[11px] font-medium uppercase tracking-wider opacity-45 ${colors.textMuted}`}>{node.title}</div>}
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

    case "text":
      return <div key={keyIndex} className={`text-xs leading-relaxed font-sans opacity-60 ${node.className || ""}`}>{node.value}</div>;

    default:
      return null;
  }
}