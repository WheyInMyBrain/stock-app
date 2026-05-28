interface HeaderProps {
  selectedTicker: string | null;
  isDark: boolean;
  onToggleTheme: () => void;
  isEditing: boolean;
  onToggleEdit: () => void;
  onResetLayout: () => void;
  colors: any;
}

export default function Header({ 
  selectedTicker, 
  isDark, 
  onToggleTheme, 
  isEditing, 
  onToggleEdit, 
  onResetLayout, 
  colors 
}: HeaderProps) {
  return (
    <header className={`h-16 border-b ${colors.border} px-6 flex items-center justify-between font-mono select-none`}>
      
      {/* Dynamic Environment Section Header */}
      <div className="flex items-center pl-10 h-full transition-all duration-300">
        <span className="text-xs font-bold tracking-[0.3em] uppercase transition-all duration-200">
          {selectedTicker ? (
            <span className="animate-fadeIn">
              // ACTIVE // <span className={isDark ? "text-white" : "text-black"}>{selectedTicker}</span>
            </span>
          ) : (
            <span className={`${colors.textMuted} animate-fadeIn`}>STOCK APP</span>
          )}
        </span>
      </div>

      {/* Action Controller Buttons Group */}
      <div className="flex items-center gap-2">
        
        {/* 🚀 THE LAYOUT RESET BUTTON
            Only mounts if an active corporate directory is working and configuration mode is engaged */}
        {selectedTicker && isEditing && (
          <button
            onClick={onResetLayout}
            className={`w-8 h-8 flex items-center justify-center border cursor-pointer rounded transition-all duration-150 text-neutral-400 hover:text-red-400 hover:border-red-500/40 bg-transparent`}
            title="Reset Grid Layout Parameters to Default"
          >
            {/* Minimalist Counter-Clockwise Rotation Loop SVG */}
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="w-4 h-4 transition-transform duration-300 active:-rotate-180"
            >
              <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
              <path d="M3 3v5h5" />
            </svg>
          </button>
        )}
        
        {/* The Configuration Wrench Icon */}
        {selectedTicker && (
          <button
            onClick={onToggleEdit}
            className={`w-8 h-8 flex items-center justify-center border cursor-pointer rounded transition-all duration-200 ${colors.border} ${
              isEditing 
                ? "bg-red-500/10 border-red-500 text-red-500" 
                : colors.hover
            }`}
            title={isEditing ? "Lock Layout Structure & Save" : "Customize Component Positions"}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="w-4 h-4"
            >
              <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
            </svg>
          </button>
        )}

        {/* Minimalist Theme Toggle Button */}
        <button
          onClick={onToggleTheme}
          className={`w-8 h-8 flex items-center justify-center border cursor-pointer rounded transition-all duration-200 ${colors.border} ${colors.hover}`}
          title={isDark ? "Switch to Light Mode" : "Switch to Dark Mode"}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill={isDark ? "none" : "currentColor"}
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="w-4 h-4"
          >
            <path d="M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A5 5 0 0 0 8 8c0 1.3.5 2.6 1.5 3.5.8.8 1.3 1.5 1.5 2.5" />
            <path d="M9 18h6" />
            <path d="M10 22h4" />
          </svg>
        </button>
      </div>
    </header>
  );
}