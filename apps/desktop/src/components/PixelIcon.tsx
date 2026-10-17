import React from "react";

export type PixelIconName =
  | "device"
  | "lcd"
  | "lighting"
  | "diagnostics"
  | "check"
  | "warn"
  | "error"
  | "refresh"
  | "play"
  | "stop"
  | "save"
  | "upload"
  | "download";

interface PixelIconProps {
  name: PixelIconName;
  size?: number;
  className?: string;
  "aria-hidden"?: boolean | "true" | "false";
}

export const PixelIcon: React.FC<PixelIconProps> = ({
  name,
  size = 16,
  className = "",
  "aria-hidden": ariaHidden = true,
}) => {
  const renderPath = () => {
    switch (name) {
      case "device":
        // Keyboard grid glyph
        return (
          <>
            <rect x="1" y="3" width="14" height="10" fill="none" stroke="currentColor" strokeWidth="1" />
            <rect x="3" y="5" width="2" height="2" fill="currentColor" />
            <rect x="7" y="5" width="2" height="2" fill="currentColor" />
            <rect x="11" y="5" width="2" height="2" fill="currentColor" />
            <rect x="4" y="9" width="8" height="2" fill="currentColor" />
          </>
        );
      case "lcd":
        // Monitor / screen glyph
        return (
          <>
            <rect x="2" y="2" width="12" height="9" fill="none" stroke="currentColor" strokeWidth="1" />
            <rect x="4" y="4" width="8" height="5" fill="currentColor" />
            <rect x="7" y="11" width="2" height="2" fill="currentColor" />
            <rect x="5" y="13" width="6" height="1" fill="currentColor" />
          </>
        );
      case "lighting":
        // Sun / light bulb pixel glyph
        return (
          <>
            <rect x="7" y="1" width="2" height="2" fill="currentColor" />
            <rect x="7" y="13" width="2" height="2" fill="currentColor" />
            <rect x="1" y="7" width="2" height="2" fill="currentColor" />
            <rect x="13" y="7" width="2" height="2" fill="currentColor" />
            <rect x="5" y="5" width="6" height="6" fill="currentColor" />
          </>
        );
      case "diagnostics":
        // Doctor cross / gauge pixel glyph
        return (
          <>
            <rect x="2" y="2" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1" />
            <rect x="7" y="4" width="2" height="8" fill="currentColor" />
            <rect x="4" y="7" width="8" height="2" fill="currentColor" />
          </>
        );
      case "check":
        // Checkmark
        return (
          <path
            d="M3 8 L6 11 L13 4"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="square"
          />
        );
      case "warn":
        // Exclamation mark
        return (
          <>
            <rect x="7" y="3" width="2" height="6" fill="currentColor" />
            <rect x="7" y="11" width="2" height="2" fill="currentColor" />
          </>
        );
      case "error":
        // X mark
        return (
          <path
            d="M3 3 L13 13 M13 3 L3 13"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="square"
          />
        );
      case "refresh":
        // Loop arrows
        return (
          <>
            <rect x="3" y="3" width="10" height="2" fill="currentColor" />
            <rect x="11" y="3" width="2" height="6" fill="currentColor" />
            <rect x="3" y="11" width="10" height="2" fill="currentColor" />
            <rect x="3" y="7" width="2" height="6" fill="currentColor" />
          </>
        );
      case "play":
        return <polygon points="4,2 14,8 4,14" fill="currentColor" />;
      case "stop":
        return <rect x="3" y="3" width="10" height="10" fill="currentColor" />;
      case "save":
        return (
          <>
            <rect x="2" y="2" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1" />
            <rect x="4" y="2" width="8" height="4" fill="currentColor" />
            <rect x="5" y="8" width="6" height="5" fill="currentColor" />
          </>
        );
      case "upload":
        return (
          <>
            <polygon points="8,2 3,7 6,7 6,12 10,12 10,7 13,7" fill="currentColor" />
            <rect x="2" y="13" width="12" height="2" fill="currentColor" />
          </>
        );
      case "download":
        return (
          <>
            <polygon points="8,12 3,7 6,7 6,2 10,2 10,7 13,7" fill="currentColor" />
            <rect x="2" y="13" width="12" height="2" fill="currentColor" />
          </>
        );
      default:
        return null;
    }
  };

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      className={`pixel-icon ${className}`}
      shapeRendering="crispEdges"
      aria-hidden={ariaHidden}
      focusable="false"
    >
      {renderPath()}
    </svg>
  );
};
