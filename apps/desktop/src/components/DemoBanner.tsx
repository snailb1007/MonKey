import React from "react";

interface DemoBannerProps {
  isDemo: boolean;
}

export const DemoBanner: React.FC<DemoBannerProps> = ({ isDemo }) => {
  if (!isDemo) return null;

  return (
    <aside
      className="demo-banner"
      role="status"
      aria-label="Demo mode warning"
    >
      [DEMO] SIMULATED ENVIRONMENT — NO HARDWARE WRITES WILL BE PERFORMED
    </aside>
  );
};
