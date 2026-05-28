interface WorkspaceProps {
  selectedTicker: string | null;
  colors: any;
}

export default function Workspace({ selectedTicker, colors }: WorkspaceProps) {
  return (
    <main className="flex-1 h-full flex flex-col justify-center items-center p-8 text-center select-none font-mono">
      {selectedTicker ? (
        <div className="tracking-tight max-w-xl animate-fadeIn">
          <div className={`text-[10px] tracking-[0.4em] uppercase mb-4 ${colors.textMuted}`}>Target Directory Initialized</div>
          <h2 className="text-7xl font-black tracking-tighter mb-4">{selectedTicker}</h2>
          <div className={`h-[1px] w-12 mx-auto my-6 border-b ${colors.border}`}></div>
          <p className={`text-xs ${colors.textMuted} tracking-widest leading-relaxed uppercase`}>
            Awaiting instructions for binary sidecar tasks.
          </p>
        </div>
      ) : (
        <div className="flex flex-col items-center">
          <div className={`text-xs tracking-[0.2em] uppercase ${colors.textMuted}`}>
            // Mount a data container node to proceed
          </div>
        </div>
      )}
    </main>
  );
}