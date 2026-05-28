import { useState } from "react";

interface SidebarProps {
  isOpen: boolean;
  tickers: string[];
  selected: string | null;
  onSelect: (ticker: string) => void;
  colors: any;
}

export default function Sidebar({ isOpen, tickers, selected, onSelect, colors }: SidebarProps) {
  const [search, setSearch] = useState("");
  if (!isOpen) return null;

  const filtered = tickers.filter(t => t.toLowerCase().includes(search.toLowerCase()));

  return (
    <div className={`w-[260px] h-full border-r ${colors.bgSidebar} ${colors.border} flex flex-col font-mono select-none`}>
      {/* Search Header Container */}
      <div className={`p-4 border-b ${colors.border} flex flex-col gap-2`}>
        <div className="text-[10px] tracking-[0.2em] uppercase font-bold opacity-60">History Vault</div>
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Filter modules..."
          className={`w-full px-2.5 py-1.5 text-xs rounded border outline-none tracking-tight font-sans transition-all duration-150 focus:border-neutral-500 ${colors.input}`}
        />
      </div>

      {/* Ticker Row Mapping Feed */}
      <div className="flex-1 overflow-y-auto p-2 flex flex-col gap-0.5">
        {filtered.length === 0 ? (
          <div className={`text-[11px] text-center py-6 ${colors.textMuted}`}>Empty Index</div>
        ) : (
          filtered.map(ticker => (
            <button
              key={ticker}
              onClick={() => onSelect(ticker)}
              className={`w-full text-left px-3 py-2 text-xs uppercase tracking-widest rounded transition-all duration-100 ${
                selected === ticker ? colors.activeItem : `${colors.textMuted} ${colors.hover}`
              }`}
            >
              // {ticker}
            </button>
          ))
        )}
      </div>
    </div>
  );
}