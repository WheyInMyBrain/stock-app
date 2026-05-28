import OverviewPanel from "./workspace/OverviewPanel";

interface WorkspaceProps {
  selectedTicker: string | null;
  colors: any;
}

export default function Workspace({ selectedTicker, colors }: WorkspaceProps) {
  
  // 🎯 THE REGISTRY ENGINE: 
  // When you build new charts, log frames, or tables later, just drop them in this array!
  const registryModules = [
    {
      id: "overview_metrics",
      title: "Data Matrix Repository",
      render: (ticker: string) => <OverviewPanel ticker={ticker} colors={colors} />
    },
    // Future expansion slots are fully abstract:
    // { id: "charts", title: "Financial Charts", render: (t) => <YourFutureChartModule ticker={t} colors={colors} /> },
    // { id: "logs", title: "Pipeline Logs", render: (t) => <YourRustScriptConsole ticker={t} colors={colors} /> },
  ];

  return (
    <main className="flex-1 h-full w-full p-6 overflow-y-auto select-none bg-transparent">
      {selectedTicker ? (
        /* 🪐 ACTIVE SCREEN VIEWPORT CANVAS */
        <div className="w-full flex flex-col gap-6 max-w-6xl mx-auto">
          
          {/* Section Header Frame */}
          <div className={`flex items-center justify-between border-b ${colors.border} pb-4 font-mono`}>
            <div className="flex flex-col gap-0.5">
              <span className={`text-[10px] tracking-[0.3em] uppercase ${colors.textMuted}`}>Workspace Module Initialization</span>
              <h2 className="text-2xl font-black uppercase tracking-tight">Active Matrix Hub: {selectedTicker}</h2>
            </div>
            <div className={`text-[10px] border ${colors.border} px-2 py-0.5 rounded font-mono ${colors.textMuted}`}>
              ENV // RECOGNIZED
            </div>
          </div>

          {/* ⚡ THE AUTOMATED COMPILER CONTAINER
              Iterates over your abstract registry list and lays out your tools perfectly */}
          <div className="flex flex-col gap-6">
            {registryModules.map((module) => (
              <section key={module.id} className="w-full flex flex-col gap-2">
                <div className="font-mono text-[10px] tracking-[0.2em] uppercase opacity-40 pl-1">
                  [ {module.title} ]
                </div>
                <div className="w-full">
                  {module.render(selectedTicker)}
                </div>
              </section>
            ))}
          </div>

        </div>
      ) : (
        /* 🗺️ EMPTY SCREEN VIEWPORT HOME STATE */
        <div className="h-full w-full flex flex-col justify-center items-center text-center font-mono">
          <div className="relative flex flex-col items-center gap-2">
            <div className={`text-xs tracking-[0.2em] uppercase ${colors.textMuted} animate-pulse`}>
              // Mount a data container node to proceed
            </div>
            <div className="text-[10px] opacity-30 mt-1 uppercase max-w-[280px] leading-relaxed">
              Select a ticker from the history vault list to feed processing parameters to the runtime view.
            </div>
          </div>
        </div>
      )}
    </main>
  );
}