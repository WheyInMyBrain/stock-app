import OverviewPanel from "./OverviewPanel";

// 🎯 Type Definition ensuring complete stability across any components you write later
export interface WorkspaceModule {
  name: string;
  description: string;
  render: (ticker: string, colors: any) => React.ReactNode;
}

// 🏢 MASTER COMPONENT DICTIONARY
// This maps string IDs to their actual TSX file layers
export const componentRegistry: Record<string, WorkspaceModule> = {
  overview_metrics: {
    name: "Data Matrix Repository",
    description: "Displays core metrics and financial table indices.",
    render: (ticker, colors) => <OverviewPanel ticker={ticker} colors={colors} />
  },
  
  // 💡 Future expansion examples:
  /*
  financial_charts: {
    name: "Interactive Performance Graph",
    description: "Visualizes deep chart telemetry matrices.",
    render: (ticker, colors) => <FinancialChart ticker={ticker} colors={colors} />
  },
  pipeline_console: {
    name: "Rust / Go Sidecar Logs",
    description: "Monitors execution threads of underlying local scripts.",
    render: (ticker, colors) => <PipelineConsole ticker={ticker} colors={colors} />
  }
  */
};

// Standard fallback order for initialization
export const defaultLayoutOrder: string[] = ["overview_metrics"];