import React from "react";
import { PixelIcon, type PixelIconName } from "./PixelIcon";

export type TabId = "device" | "lcd" | "lighting" | "diagnostics";

interface NavProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

interface NavItem {
  id: TabId;
  label: string;
  icon: PixelIconName;
}

const NAV_ITEMS: NavItem[] = [
  { id: "device", label: "Device", icon: "device" },
  { id: "lcd", label: "LCD Screen", icon: "lcd" },
  { id: "lighting", label: "RGB Lighting", icon: "lighting" },
  { id: "diagnostics", label: "Diagnostics", icon: "diagnostics" },
];

export const Nav: React.FC<NavProps> = ({ activeTab, onTabChange }) => {
  return (
    <nav className="sidebar-nav" aria-label="Main Navigation">
      <ul className="nav-list" role="list">
        {NAV_ITEMS.map((item) => {
          const isActive = activeTab === item.id;
          return (
            <li key={item.id} className="nav-item">
              <button
                type="button"
                className={`nav-item-btn ${isActive ? "active" : ""}`}
                onClick={() => onTabChange(item.id)}
                aria-current={isActive ? "page" : undefined}
              >
                <PixelIcon name={item.icon} size={18} />
                <span>{item.label}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </nav>
  );
};
