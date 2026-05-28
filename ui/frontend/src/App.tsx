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

  // Invoke the native backend directory crawler
  useEffect(() => {
    invoke<string[]>("get_history_tickers")
      .then(setTickers)
      .catch((err) => console.error("History engine failure:", err));
  }, []);

  return (
    <div className={`flex w-screen h-screen overflow-hidden transition-colors duration-300 font-sans ${colors.bgMain}`}>
      {/* 1. Modular Sidebar Component */}
      <Sidebar
        isOpen={sidebarOpen}
        tickers={tickers}
        selected={selectedTicker}
        onSelect={setSelectedTicker}
        colors={colors}
      />

      {/* 2. Primary Viewing Container */}
      <div className="flex-1 h-full flex flex-col overflow-hidden">
        {/* 3. Modular Navigation Header */}
        <Header
          sidebarOpen={sidebarOpen}
          setSidebarOpen={setSidebarOpen}
          isDark={isDarkMode}
          onToggleTheme={toggleTheme}
          colors={colors}
        />

        {/* 4. Modular Interactive Workspace */}
        <Workspace selectedTicker={selectedTicker} colors={colors} />
      </div>
    </div>
  );
}