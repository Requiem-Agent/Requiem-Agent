import React from "react";

interface LogoProps {
  size?: number;
  className?: string;
  showText?: boolean;
  variant?: "full" | "icon" | "minimal";
}

export function Logo({ size = 60, className = "", showText = true, variant = "full" }: LogoProps) {
  const iconSize = size * 0.47;

  if (variant === "minimal") {
    return (
      <div className={className} style={{ display: "inline-flex", alignItems: "center", gap: "8px" }}>
        <svg width={size * 0.5} height={size * 0.5} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect x="2" y="6" width="20" height="14" rx="3" stroke="#000" strokeWidth="1.5" fill="none"/>
          <path d="M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2" stroke="#000" strokeWidth="1.5" fill="none"/>
          <circle cx="12" cy="13" r="3" stroke="#000" strokeWidth="1.5" fill="none"/>
          <path d="M12 10v-1M12 17v-1M9 13H8M16 13h-1" stroke="#000" strokeWidth="1" strokeLinecap="round"/>
        </svg>
      </div>
    );
  }

  return (
    <div className={className} style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "12px" }}>
      {/* Logo Icon */}
      <div style={{
        height: `${size}px`,
        width: `${size}px`,
        borderRadius: `${size * 0.27}px`,
        background: "#000000",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        position: "relative",
        overflow: "hidden",
      }}>
        {/* Decorative dots */}
        <div style={{
          position: "absolute",
          top: "8px",
          right: "8px",
          width: "4px",
          height: "4px",
          borderRadius: "50%",
          background: "#FFFFFF",
          opacity: 0.3,
        }} />
        <div style={{
          position: "absolute",
          top: "8px",
          right: "16px",
          width: "4px",
          height: "4px",
          borderRadius: "50%",
          background: "#FFFFFF",
          opacity: 0.2,
        }} />
        <div style={{
          position: "absolute",
          top: "8px",
          right: "24px",
          width: "4px",
          height: "4px",
          borderRadius: "50%",
          background: "#FFFFFF",
          opacity: 0.1,
        }} />

        {/* Main icon - Brain/Chip */}
        <svg width={iconSize} height={iconSize} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          {/* Outer circuit */}
          <rect x="3" y="3" width="18" height="18" rx="4" stroke="#FFFFFF" strokeWidth="1.5" fill="none" opacity="0.3"/>
          {/* Inner brain shape */}
          <path d="M12 6C9.5 6 7.5 8 7.5 10.5C7.5 12 8.5 13.3 10 14V17.5C10 18.05 10.45 18.5 11 18.5H13C13.55 18.5 14 18.05 14 17.5V14C15.5 13.3 16.5 12 16.5 10.5C16.5 8 14.5 6 12 6Z" stroke="#FFFFFF" strokeWidth="1.5" fill="none"/>
          {/* Connection lines */}
          <path d="M12 6V3M12 21V18.5M6 12H3M21 12H18.5M7.5 7.5L5.5 5.5M18.5 7.5L16.5 5.5M7.5 16.5L5.5 18.5M18.5 16.5L16.5 18.5" stroke="#FFFFFF" strokeWidth="1" strokeLinecap="round" opacity="0.5"/>
          {/* Center dot */}
          <circle cx="12" cy="10.5" r="1.5" fill="#FFFFFF"/>
        </svg>
      </div>

      {/* Text */}
      {showText && variant === "full" && (
        <div style={{ textAlign: "center" }}>
          <div style={{
            fontSize: `${Math.max(16, size * 0.3)}px`,
            fontWeight: 700,
            color: "#000000",
            letterSpacing: "-0.5px",
            lineHeight: 1.2,
          }}>
            PopCorn
          </div>
          <div style={{
            fontSize: `${Math.max(10, size * 0.17)}px`,
            fontWeight: 500,
            color: "#737373",
            letterSpacing: "2px",
            textTransform: "uppercase",
          }}>
            AI Studio
          </div>
        </div>
      )}
    </div>
  );
}

// Export as default for backward compatibility
export default Logo;
