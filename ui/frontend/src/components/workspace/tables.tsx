import React, { useState } from "react";
import PrimitiveCompiler, { type UiPrimitiveNode } from "./PrimitiveCompiler";

interface TableRendererProps {
  node: UiPrimitiveNode;
  colors: {
    border?: string;
    text?: string;
    textMuted?: string;
    [key: string]: any;
  };
  cardBg: string;
  keyIndex: number;
}

export function TableComponent({ node, colors, cardBg, keyIndex }: TableRendererProps) {
  const activeBorder = colors?.border || "";
  const activeTextMuted = colors?.textMuted || "opacity-50";
  const totalChildren = node.children?.length || 0;

  return (
    <table key={keyIndex} className={`w-full border-collapse ${node.className || ""}`} style={node.style}>
      {node.headers && (
        <thead>
          <tr className={`border-b ${activeBorder}`}>
            {node.headers.map((header, idx) => (
              <th key={idx} className={`py-2 px-3 text-xs font-semibold uppercase tracking-wider font-mono ${activeTextMuted}`}>
                {header}
              </th>
            ))}
          </tr>
        </thead>
      )}
      <tbody>
        {node.children?.map((childNode, index) => (
          <PrimitiveCompiler 
            key={index} 
            node={{
              ...childNode,
              // Inject total row positions into the pipeline loop context to assist the vertical anchor flips
              percentage_height: index,
              stroke_width: totalChildren 
            }} 
            colors={colors} 
            cardBg={cardBg} 
            keyIndex={index} 
          />
        ))}
      </tbody>
    </table>
  );
}

export function TableRowComponent({ node, colors, cardBg, keyIndex }: TableRendererProps) {
  const activeBorder = colors?.border || "";
  const activeText = colors?.text || "";
  const activeTextMuted = colors?.textMuted || "opacity-40";

  const [hovered, setHovered] = useState(false);
  const [flipLeft, setFlipLeft] = useState(false);
  const [flipVerticalUp, setFlipVerticalUp] = useState(false);

  const handleMouseEnter = (e: React.MouseEvent<HTMLTableRowElement>) => {
    const bounds = e.currentTarget.getBoundingClientRect();
    const windowWidth = window.innerWidth;
    
    // Horizontal alignment track check
    if (bounds.left + bounds.width / 2 > windowWidth / 2) {
      setFlipLeft(true);
    } else {
      setFlipLeft(false);
    }

    // 🚀 DETERMINISTIC VERTICAL FLIP MATRIX
    // Reads row index and total sibling layout boundaries computed up in TableComponent
    const currentRowIdx = node.percentage_height ?? 0;
    const totalTableRows = node.stroke_width ?? 1;

    if (currentRowIdx > totalTableRows / 2) {
      setFlipVerticalUp(true); // Hovering on bottom rows -> open popup above mouse row
    } else {
      setFlipVerticalUp(false); // Hovering on top rows -> open popup below mouse row
    }

    setHovered(true);
  };

  return (
    <tr 
      className={`border-b last:border-b-0 group/row relative ${activeBorder} ${node.className || ""}`} 
      style={node.style}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={() => setHovered(false)}
    >
      {node.cells?.map((cellNode, index) => {
        const safeChildren = cellNode.children || [];
        const hasHoverData = safeChildren.length > 0;

        return (
          <td key={index} className="py-2 px-3 relative">
            <PrimitiveCompiler 
              node={cellNode} 
              colors={colors} 
              cardBg={cardBg} 
              keyIndex={index} 
            />

            {/* 🚀 PERFECTLY BOUNDED DOCK OVERLAY CARD CONTAINER */}
            {index === 1 && hasHoverData && hovered && (
              <div 
                className="absolute flex flex-col gap-1.5 p-3 rounded-lg shadow-xl border backdrop-blur-md pointer-events-none w-[380px] z-[999]"
                style={{
                  backgroundColor: cardBg.startsWith("#") ? cardBg : "rgba(13, 13, 13, 0.98)",
                  borderColor: colors?.border || "rgba(255,255,255,0.12)",
                  
                  // 🎯 DYNAMIC ACCURATE FLIP TRACKING ANCHORS
                  top: flipVerticalUp ? "auto" : "100%",
                  bottom: flipVerticalUp ? "100%" : "auto",
                  left: flipLeft ? "auto" : "12%",
                  right: flipLeft ? "12%" : "auto",
                  
                  marginTop: flipVerticalUp ? "0px" : "4px",
                  marginBottom: flipVerticalUp ? "4px" : "0px"
                }}
              >
                <div className="text-[9px] font-mono uppercase tracking-widest font-bold opacity-30 border-b border-neutral-500/10 pb-1 mb-0.5">
                  Corporate Identity Ledger
                </div>
                
                {/* 🚀 TIGHT DUAL COMPACT GRID PACKING FOR FLUSH CARD FITTING */}
                <div className="grid grid-cols-2 gap-x-3 gap-y-1 w-full text-left">
                  {safeChildren.map((profileItem, pIdx) => (
                    <div 
                      key={pIdx} 
                      className="flex flex-col gap-0.5 border-b border-dashed border-neutral-500/5 pb-0.5"
                    >
                      <span className={`text-[9px] uppercase tracking-wider font-mono truncate ${activeTextMuted}`}>
                        {profileItem.title}
                      </span>
                      <span className={`text-[11px] font-medium tracking-tight font-sans leading-tight break-words ${activeText || "text-neutral-200"}`}>
                        {profileItem.value}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </td>
        );
      })}
    </tr>
  );
}