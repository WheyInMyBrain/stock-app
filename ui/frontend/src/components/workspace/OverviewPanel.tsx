interface SubModuleProps {
  ticker: string;
  colors: any;
}

export default function OverviewPanel({ ticker, colors }: SubModuleProps) {
  const cardBg = colors.input.includes("bg-[#0A0A0A]") ? "bg-[#121212]" : "bg-[#F4F4F5]";
  
  return (
    // 🎯 Set to h-full to bind exactly to the parent widget's height adjustments
    <div className="w-full h-full grid grid-cols-1 md:grid-cols-3 gap-4 font-mono text-left animate-fadeIn">
      
      {/* 📊 LEFT DATA CARD */}
      <div className={`p-4 border rounded ${colors.border} ${cardBg} h-full flex flex-col justify-between`}>
        <div>
          <div className={`text-[10px] tracking-wider uppercase mb-1 ${colors.textMuted}`}>// CAPTURE GRID</div>
          <div className="text-xl font-black uppercase tracking-tight">{ticker} STABLE</div>
        </div>
        <div className="text-[11px] opacity-50 mt-4 border-t pt-2 border-neutral-700/30">
          Status: Parquet Active
        </div>
      </div>

      {/* 📊 RIGHT EXPANDING CHART MATRIX CARD */}
      <div className={`p-4 border rounded ${colors.border} ${cardBg} md:col-span-2 h-full flex flex-col justify-between`}>
        <div className="flex-shrink-0">
          <div className={`text-[10px] tracking-wider uppercase mb-1 ${colors.textMuted}`}>// ANALYTICAL GRAPH MATRIX</div>
          <div className="text-xs opacity-70">Pipeline telemetry scales smoothly to match container geometry.</div>
        </div>
        
        {/* 🎯 DYNAMIC CHART AREA: flex-1 and h-full ensures this block scales vertically when resized */}
        <div className="w-full flex-1 min-h-[40px] flex items-end gap-1.5 mt-4 pt-4">
          <div className="bg-current opacity-20 w-full h-[40%] rounded-t-sm transition-all duration-150" />
          <div className="bg-current opacity-40 w-full h-[75%] rounded-t-sm transition-all duration-150" />
          <div className="bg-current opacity-90 w-full h-[95%] rounded-t-sm transition-all duration-150" />
          <div className="bg-current opacity-30 w-full h-[55%] rounded-t-sm transition-all duration-150" />
          <div className="bg-current opacity-60 w-full h-[85%] rounded-t-sm transition-all duration-150" />
        </div>
      </div>
      
    </div>
  );
}