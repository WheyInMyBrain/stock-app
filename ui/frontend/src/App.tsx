import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "./hooks/useTheme";
import Sidebar from "./components/Sidebar";
import Header from "./components/Header";
import Workspace from "./components/Workspace";

export default function App() {
  const { isDarkMode, toggleTheme, colors } = useTheme();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [tickers, setTickers] = useState<string[]>([]);
  const [selectedTicker, setSelectedTicker] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>("get_history_tickers")
      .then(setTickers)
      .catch((err) => console.error("History engine failure:", err));
  }, []);

  return (
    <div className={`flex w-screen h-screen overflow-hidden transition-colors duration-300 font-sans ${colors.bgMain}`}>
      <Sidebar
        isOpen={sidebarOpen}
        setIsOpen={setSidebarOpen}
        tickers={tickers}
        selected={selectedTicker}
        onSelect={setSelectedTicker}
        colors={colors}
      />

      <div className="flex-1 h-full flex flex-col overflow-hidden">
        {/* 🎯 Pass the active state hook context down here */}
        <Header
          selectedTicker={selectedTicker}
          isDark={isDarkMode}
          onToggleTheme={toggleTheme}
          colors={colors}
        />
        <Workspace selectedTicker={selectedTicker} colors={colors} />
      </div>
    </div>
  );
}