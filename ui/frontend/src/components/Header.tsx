interface HeaderProps {
  sidebarOpen: boolean;
  setSidebarOpen: (open: boolean) => void;
  isDark: boolean;
  onToggleTheme: () => void;
  colors: any;
}

export default function Header({ sidebarOpen, setSidebarOpen, isDark, onToggleTheme, colors }: HeaderProps) {
  return (
    <header className={`h-12 border-b ${colors.border} px-4 flex items-center justify-between font-mono select-none`}>
      <div className="flex items-center gap-4">
        <button
          onClick={() => setSidebarOpen(!sidebarOpen)}
          className={`text-[11px] px-2.5 py-1 border ${colors.border} ${colors.hover} transition-all duration-150 rounded`}
        >
          {sidebarOpen ? "[-]" : "[+]"}
        </button>
        <span className="text-xs font-bold tracking-[0.3em] uppercase">Core Engine Panel</span>
      </div>

      <button
        onClick={onToggleTheme}
        className={`text-[10px] tracking-widest px-3 py-1 border ${colors.border} ${colors.hover} rounded transition-all duration-150`}
      >
        {isDark ? "STARK LIGHT" : "JET BLACK"}
      </button>
    </header>
  );
}