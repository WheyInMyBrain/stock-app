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
    <header className={`h-16 border-b ${colors.border} px-6 flex items-center justify-between font-sans tracking-normal select-none`}>
      
      {/* Calm Typography Section Header Mapping */}
      <div className="flex items-center pl-10 h-full transition-all duration-300">
        <span className="text-xs font-medium tracking-wider uppercase transition-all duration-200">
          {selectedTicker ? (
            <span className="animate-fadeIn">
              // ACTIVE // <span className={`font-semibold ${isDark ? "text-white" : "text-black"}`}>{selectedTicker}</span>
            </span>
          ) : (
            <span className={`${colors.textMuted} animate-fadeIn`}>STOCK APP</span>
          )}
        </span>
      </div>

      {/* Action Controller Buttons Group */}
      <div className="flex items-center gap-2">
        
        {/* 1. THE LAYOUT RESET BUTTON (Only mounts inside active customization mode) */}
        {selectedTicker && isEditing && (
          <button
            onClick={onResetLayout}
            className="w-8 h-8 flex items-center justify-center border cursor-pointer rounded-lg transition-all duration-150 text-neutral-400 hover:text-red-400 hover:border-red-500/40 bg-transparent border-neutral-700/40 active:scale-95"
            title="Reset Grid Layout Parameters to Default"
          >
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

        {/* 2. MINIMALIST THEME TOGGLE BUTTON (Moved inwards for clean balance) */}
        <button
          onClick={onToggleTheme}
          className={`w-8 h-8 flex items-center justify-center border cursor-pointer rounded-lg transition-all duration-200 ${colors.border} ${colors.hover} active:scale-95`}
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
        
        {/* 3. THE CONFIGURATION WRENCH ICON (🎯 SWAPPED: Placed on the anchor edge position) */}
        {selectedTicker && (
          <button
            onClick={onToggleEdit}
            className={`w-8 h-8 flex items-center justify-center border cursor-pointer rounded-lg transition-all duration-200 active:scale-95 ${colors.border} ${
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
      </div>
    </header>
  );
}