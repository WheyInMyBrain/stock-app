import { useState } from "react";

interface SidebarProps {
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
  tickers: string[];
  selected: string | null;
  onSelect: (ticker: string | null) => void;
  colors: any;
}

export default function Sidebar({ isOpen, setIsOpen, tickers, selected, onSelect, colors }: SidebarProps) {
  const [search, setSearch] = useState("");
  const [isHovered, setIsHovered] = useState(false);

  const filtered = tickers.filter(t => t.toLowerCase().includes(search.toLowerCase()));

  const customSearchBg = colors.input.includes("bg-[#0A0A0A]")
    ? "bg-[#1E1E1E] border-[#2E2E2E] text-white placeholder-[#737373]" 
    : "bg-[#E4E4E7] border-[#D4D4D8] text-black placeholder-[#71717A]"; 

  const handleTickerClick = (ticker: string) => {
    if (selected === ticker) {
      onSelect(null);
    } else {
      onSelect(ticker);
    }
  };

  return (
    /* 🏛️ UNIFIED SINGLE SIDEBAR PANEL
       Locks frame scale exactly between 56px and 296px */
    <div 
      className={`h-full border-r ${colors.bgSidebar} ${colors.border} flex relative transition-all duration-300 ease-in-out flex-shrink-0 select-none z-40`}
      style={{ width: isOpen ? "256px" : "56px" }}
    >
      
      {/* 🏛️ SLIDING CONTENT LAYER
          Now utilizes a perfectly proportioned 240px bounds for ideal layout breathing space */}
      <div 
        className="h-full flex flex-col font-mono transition-all duration-300 ease-in-out overflow-hidden absolute left-0 top-0 z-30"
        style={{ 
          width: isOpen ? "240px" : "0px", 
          opacity: isOpen ? 1 : 0,
          paddingLeft: isOpen ? "16px" : "0px" 
        }}
      >
        <div className="w-[240px] flex flex-col h-full">
          {/* Header Area */}
          <div className="pt-5 h-28 flex flex-col justify-between pb-4 pr-2">
            <div className="flex items-center h-8">
              <div className="text-[10px] tracking-[0.2em] uppercase font-bold opacity-80">
                History Vault
              </div>
            </div>

            {/* Search Box */}
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search tickers..."
              className={`w-full h-9 px-3 text-xs rounded border outline-none tracking-tight font-sans transition-all duration-150 focus:border-neutral-500 shadow-inner ${customSearchBg}`}
            />
          </div>

          {/* Clean Interior Separation Line */}
          <div className="h-[1px] w-full pr-2">
            <div className={`h-[1px] w-full border-b ${colors.border}`} />
          </div>

          {/* Scrollable File Directory Entries */}
          <div className="flex-1 overflow-y-auto pt-2 pb-2 pr-2 flex flex-col gap-0.5">
            {filtered.length === 0 ? (
              <div className={`text-[11px] text-center py-6 ${colors.textMuted}`}>Empty Index</div>
            ) : (
              filtered.map(ticker => (
                <button
                  key={ticker}
                  onClick={() => handleTickerClick(ticker)}
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
      </div>

      {/* 🎛️ TRAVELING TOGGLE BUTTON
          Maintains absolute right rim alignment relative to the layout bounds */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        className={`absolute top-5 z-50 w-8 h-8 rounded flex flex-col justify-center items-center gap-[5px] border cursor-pointer backdrop-blur-md transition-all duration-300 ease-in-out ${colors.border} ${colors.hover}`}
        style={{
          left: isOpen ? "212px" : "12px",
        }}
        aria-label="Toggle Sidebar"
      >
        {/* Top Line */}
        <span 
          className="h-[1.5px] bg-current transition-all duration-300"
          style={{
            width: isHovered ? "12px" : "16px",
            transform: isHovered 
              ? (isOpen ? "rotate(-45deg) translate(-3px, 2px)" : "rotate(45deg) translate(3px, 2px)") 
              : "none"
          }}
        />
        {/* Middle Line */}
        <span 
          className="h-[1.5px] bg-current transition-all duration-300"
          style={{
            width: "16px",
            transform: isHovered 
              ? (isOpen ? "translate(2px, 0px)" : "translate(-2px, 0px)") 
              : "none"
          }}
        />
        {/* Bottom Line */}
        <span 
          className="h-[1.5px] bg-current transition-all duration-300"
          style={{
            width: isHovered ? "12px" : "16px",
            transform: isHovered 
              ? (isOpen ? "rotate(45deg) translate(-3px, -2px)" : "rotate(-45deg) translate(3px, -2px)") 
              : "none"
          }}
        />
      </button>
    </div>
  );
}