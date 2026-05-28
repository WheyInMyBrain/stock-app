import { useState } from "react";

export function useTheme() {
  const [isDarkMode, setIsDarkMode] = useState(true);

  const colors = {
    bgMain: isDarkMode ? "bg-[#0A0A0A] text-[#F5F5F5]" : "bg-[#FAFAFA] text-[#1A1A1A]",
    bgSidebar: isDarkMode ? "bg-[#020202] border-[#121212]" : "bg-[#F4F4F5] border-[#E4E4E7]",
    border: isDarkMode ? "border-[#121212]" : "border-[#E4E4E7]",
    textMuted: isDarkMode ? "text-[#525252]" : "text-[#A1A1AA]",
    hover: isDarkMode ? "hover:bg-[#121212] hover:text-white" : "hover:bg-[#E4E4E7] hover:text-black",
    activeItem: isDarkMode ? "bg-[#FFFFFF] text-[#000000]" : "bg-[#000000] text-[#FFFFFF]",
    input: isDarkMode ? "bg-[#0A0A0A] border-[#1A1A1A] text-white placeholder-[#404040]" : "bg-white border-[#D4D4D8] text-black placeholder-[#A1A1AA]"
  };

  return { isDarkMode, toggleTheme: () => setIsDarkMode(!isDarkMode), colors };
}