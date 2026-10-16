import React, { useState } from "react";
import { DeviceProvider } from "./context/DeviceContext";
import { createBridge } from "./services/bridge";
import { isDemoMode, createDemoTransport } from "./services/demoAdapter";
import { Header } from "./components/Header";
import { Nav, type TabId } from "./components/Nav";
import { DemoBanner } from "./components/DemoBanner";
import { DevicePage } from "./pages/DevicePage";
import { LcdPage } from "./pages/LcdPage";
import { LightingPage } from "./pages/LightingPage";
import { DiagnosticsPage } from "./pages/DiagnosticsPage";

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabId>("device");
  const demoActive = isDemoMode();

  // If in demo mode, inject simulated demo transport; otherwise use real Tauri invoke bridge
  const [bridge] = useState(() =>
    createBridge(demoActive ? createDemoTransport() : undefined)
  );

  const renderActiveTab = () => {
    switch (activeTab) {
      case "device":
        return <DevicePage />;
      case "lcd":
        return <LcdPage />;
      case "lighting":
        return <LightingPage />;
      case "diagnostics":
        return <DiagnosticsPage />;
      default:
        return <DevicePage />;
    }
  };

  return (
    <DeviceProvider bridge={bridge}>
      <div className="app-shell">
        <a href="#main-content" className="skip-link">
          Skip to main content
        </a>
        <DemoBanner isDemo={demoActive} />
        <Header />
        <div className="main-body">
          <Nav activeTab={activeTab} onTabChange={setActiveTab} />
          <main id="main-content" className="content-area" tabIndex={-1}>
            {renderActiveTab()}
          </main>
        </div>
      </div>
    </DeviceProvider>
  );
};

export default App;
