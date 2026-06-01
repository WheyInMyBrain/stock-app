import React, { useState, useEffect, useRef, useLayoutEffect } from "react";
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

  // 🚀 ISOLATED STATE MAP FOR DISCRETE COLLAPSIBLE TREE OPERATIONS
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>(() => {
    const defaultState: Record<string, boolean> = {};
    node.children?.forEach((childNode: any) => {
      if (childNode?.is_parent && childNode?.parent_id) {
        defaultState[childNode.parent_id] = true;
      }
    });
    return defaultState;
  });

  useEffect(() => {
    const handleToggleEvent = (e: Event) => {
      const customId = (e as CustomEvent).detail;
      if (customId) {
        setCollapsedGroups(prev => ({ ...prev, [customId]: !prev[customId] }));
      }
    };
    window.addEventListener("local-row-toggle", handleToggleEvent);
    return () => window.removeEventListener("local-row-toggle", handleToggleEvent);
  }, []);

  return (
    /* 🚀 ISOLATED VERTICAL FREEZING ENVELOPE Pure CSS overflow frame to lock elements inside a 500px boundary */
    <div className="w-full max-w-full overflow-auto max-h-[500px] scrollbar-thin block clear-both">
      <table key={keyIndex} className={`w-full border-collapse ${node.className || ""}`} style={node.style}>
        {node.headers && (
          <thead className="sticky top-0 z-30">
            <tr className={`border-b ${activeBorder}`}>
              {node.headers.map((header, idx) => (
                <th 
                  key={idx} 
                  className={`py-2 px-3 text-xs font-semibold uppercase tracking-wider font-mono ${idx === 0 ? "sticky left-0 z-40" : ""} ${cardBg} ${activeTextMuted}`}
                  style={{ 
                    backgroundColor: cardBg.includes("bg-[#121212]") ? "#121212" : "#F4F4F5" 
                  }}
                >
                  {header}
                </th>
              ))}
            </tr>
          </thead>
        )}
        <tbody>
          {node.children?.map((childNode, index) => {
            const pId = (childNode as any).parent_id;
            const isChildRow = (childNode as any).is_child;
            if (isChildRow && pId && collapsedGroups[pId]) {
              return null;
            }

            return (
              <PrimitiveCompiler 
                key={index} 
                node={{
                  ...childNode,
                  percentage_height: index,
                  stroke_width: totalChildren 
                }} 
                colors={colors} 
                cardBg={cardBg} 
                keyIndex={index} 
              />
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export function TableRowComponent({ node, colors, cardBg, keyIndex }: TableRendererProps) {
  const activeBorder = colors?.border || "";
  const activeText = colors?.text || "";
  const activeTextMuted = colors?.textMuted || "opacity-40";

  const [hovered, setHovered] = useState(false);
  
  // 🚀 DYNAMIC POSITION STATE STORAGE
  const [popupStyles, setPopupStyles] = useState<React.CSSProperties>({
    visibility: "hidden"
  });
  
  const cellRef = useRef<HTMLTableCellElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);

  const isParentGroup = (node as any).is_parent;
  const currentParentId = (node as any).parent_id;
  const isChildGroup = (node as any).is_child;

  const [isCollapsed, setIsCollapsed] = useState(true);

  const handleMouseEnter = () => {
    setHovered(true);
  };

  useLayoutEffect(() => {
    if (!hovered || !cellRef.current || !popupRef.current) return;

    // 1. Locate the absolute scrolling frame container context of the table
    const scrollContainer = cellRef.current.closest(".overflow-auto");
    if (!scrollContainer) return;

    const cellBounds = cellRef.current.getBoundingClientRect();
    const popupHeight = popupRef.current.offsetHeight;
    const popupWidth = popupRef.current.offsetWidth;
    const containerBounds = scrollContainer.getBoundingClientRect();

    // Default positioning coordinates relative to the bounding box anchor cell
    let topCoordinate = cellBounds.bottom + 4;
    let leftCoordinate = cellBounds.left;

    // 🎯 HARD LIMIT VERTICAL POSITIONING (Strictly bound to the enclosing table frame boundaries)
    const outOfTableBottom = topCoordinate + popupHeight > containerBounds.bottom;
    const outOfViewportBottom = topCoordinate + popupHeight > window.innerHeight;

    if (outOfTableBottom || outOfViewportBottom) {
      topCoordinate = cellBounds.top - popupHeight - 4;
    }

    // Gutter protection rule: stop it from crashing upward past the sticky header row ceiling
    if (topCoordinate < containerBounds.top) {
      topCoordinate = containerBounds.top + 32; // Drops it cleanly right under the header lane
    }

    // 🎯 HARD LIMIT HORIZONTAL POSITIONING (Strictly bound to the enclosing table frame boundaries)
    const outOfTableRight = leftCoordinate + popupWidth > containerBounds.right;
    const outOfViewportRight = leftCoordinate + popupWidth > window.innerWidth;

    if (outOfTableRight || outOfViewportRight) {
      leftCoordinate = cellBounds.right - popupWidth;
    }

    // Left perimeter block lock: ensure it never slips past the table wrapper frame context wall
    if (leftCoordinate < containerBounds.left) {
      leftCoordinate = containerBounds.left + 4;
    }

    // Convert absolute screen layout space tracking metrics directly into table relative parameters
    setPopupStyles({
      position: "absolute",
      top: `${topCoordinate - cellBounds.top}px`,
      left: `${leftCoordinate - cellBounds.left}px`,
      bottom: "auto",
      right: "auto",
      marginTop: "0px",
      marginBottom: "0px",
      visibility: "visible"
    });
  }, [hovered]);

  const handleRowToggleClick = () => {
    if (isParentGroup && currentParentId) {
      setIsCollapsed(!isCollapsed);
      window.dispatchEvent(new CustomEvent("local-row-toggle", { detail: currentParentId }));
    }
  };

  return (
    <tr 
      className={`border-b last:border-b-0 group/row relative ${activeBorder} ${isParentGroup ? "cursor-pointer font-bold bg-neutral-500/5 select-none" : ""} ${isChildGroup ? "opacity-90" : ""} ${node.className || ""}`} 
      style={node.style}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={() => {
        setHovered(false);
        setPopupStyles({ visibility: "hidden" });
      }}
      onClick={handleRowToggleClick}
    >
      {node.cells?.map((cellNode, index) => {
        const safeChildren = cellNode.children || [];
        const totalItems = safeChildren.length;
        const hasHoverData = totalItems > 0;
        const isTargetCell = index === 1 && hasHoverData;

        return (
          <td 
            key={index} 
            ref={isTargetCell ? cellRef : null}
            className={`py-2 px-3 relative ${index === 0 ? "sticky left-0 z-20" : ""} ${index === 0 ? cardBg : ""} ${index === 0 && isChildGroup ? "pl-7" : ""}`}
            style={{ 
              backgroundColor: index === 0 ? (cardBg.includes("bg-[#121212]") ? "#121212" : "#F4F4F5") : undefined 
            }}
          >
            {index === 0 && isParentGroup ? (
              <div className="flex items-center gap-1.5 justify-start w-full">
                <svg 
                  className={`w-3 h-3 transition-transform duration-200 text-neutral-400 shrink-0 group-hover/row:text-neutral-200 ${isCollapsed ? "-rotate-90" : "rotate-0"}`} 
                  fill="none" 
                  viewBox="0 0 24 24" 
                  stroke="currentColor"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M19 9l-7 7-7-7" />
                </svg>
                <PrimitiveCompiler node={cellNode} colors={colors} cardBg={cardBg} keyIndex={index} />
              </div>
            ) : (
              <PrimitiveCompiler node={cellNode} colors={colors} cardBg={cardBg} keyIndex={index} />
            )}

            {/* 🚀 BOUNDS-AWARE POPUP: LOCKED WITH STICKY CONTAINER CALCULATIONS */}
            {isTargetCell && hovered && (
              <div 
                ref={popupRef}
                /* 🎯 HARD TABLE CEILING WIDTH TRACKING:
                   Using grid context wrapper layout with an auto-fit layout blueprint, but clamping 
                   the container to a static width block maxing out directly inside the table frame bounds. */
                className="absolute flex flex-col gap-1.5 p-3 rounded-lg shadow-xl border backdrop-blur-md pointer-events-none z-[999] w-[340px]"
                style={{
                  backgroundColor: cardBg.includes("bg-[#121212]") ? "#121212" : "#F4F4F5",
                  borderColor: colors?.border || "rgba(255,255,255,0.12)",
                  ...popupStyles
                }}
              >
                <div className="text-[9px] font-mono uppercase tracking-widest font-bold opacity-30 border-b border-neutral-500/10 pb-1 mb-0.5 w-full">
                  Corporate Identity Ledger
                </div>
                
                {/* 🎯 AUTONOMOUS RESPONSIVE AUTO-GRID CELL REPACKER */}
                <div className="grid gap-x-3 gap-y-1.5 w-full text-left grid-cols-[repeat(auto-fit,minmax(140px,1fr))]">
                  {safeChildren.map((profileItem, pIdx) => (
                    <div 
                      key={pIdx} 
                      className="flex flex-col gap-0.5 border-b border-dashed border-neutral-500/5 pb-0.5 min-w-0"
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