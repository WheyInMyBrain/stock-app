interface HeaderProps {
  isDark: boolean;
  onToggleTheme: () => void;
  colors: any;
}

export default function Header({ isDark, onToggleTheme, colors }: HeaderProps) {
  return (
    <header className={`h-16 border-b ${colors.border} px-6 flex items-center justify-between font-mono select-none`}>
      {/* Retain padding to compensate for the sliding panel button space natively */}
      <div className="flex items-center pl-10">
        <span className="text-xs font-bold tracking-[0.3em] uppercase">Core Engine Panel</span>
      </div>

      {/* Minimalist Theme Toggle Button */}
      <button
        onClick={onToggleTheme}
        className={`w-8 h-8 flex items-center justify-center border cursor-pointer rounded transition-all duration-200 ${colors.border} ${colors.hover}`}
        title={isDark ? "Switch to Light Mode" : "Switch to Dark Mode"}
      >
        {/* Stark Vector Lightbulb Icon */}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill={isDark ? "none" : "currentColor"} /* Fills flat black in light mode, stays outlined in dark mode */
          stroke="currentColor" /* Inherits inverted text color automatically */
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="w-4 h-4 transition-transform duration-200 hover:scale-105 active:scale-95"
        >
          <path d="M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A5 5 0 0 0 8 8c0 1.3.5 2.6 1.5 3.5.8.8 1.3 1.5 1.5 2.5" />
          <path d="M9 18h6" />
          <path d="M10 22h4" />
        </svg>
      </button>
    </header>
  );
}