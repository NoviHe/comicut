import React from "react";

interface ComiCutLogoProps {
  className?: string;
  colorMode?: "duotone" | "monochrome";
}

export const ComiCutLogo: React.FC<ComiCutLogoProps> = ({
  className = "h-5 w-5",
  colorMode = "duotone",
}) => {
  if (colorMode === "monochrome") {
    return (
      <svg
        viewBox="0 0 256 256"
        className={className}
        fill="currentColor"
        role="img"
        aria-label="ComiCut Logo"
      >
        <path d="M 124 22 L 196 22 C 212.57 22 226 35.43 226 52 L 226 84 C 226 92.84 218.84 100 210 100 L 158 100 C 146.95 100 138 108.95 138 120 L 138 128 L 80 70 L 96 22 Z" />
        <path d="M 30 104 L 88 162 L 88 170 C 88 181.05 96.95 190 108 190 L 160 190 C 168.84 190 176 197.16 176 206 L 176 214 C 176 226.15 166.15 236 154 236 L 74 236 C 50.8 236 32 217.2 32 194 L 32 128 Z" />
      </svg>
    );
  }

  return (
    <svg
      viewBox="0 0 256 256"
      className={className}
      fill="none"
      role="img"
      aria-label="ComiCut Logo"
    >
      <defs>
        <linearGradient id="comicut-top-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#A78BFA" />
          <stop offset="100%" stopColor="#C084FC" />
        </linearGradient>
        <linearGradient id="comicut-bot-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#F472B6" />
          <stop offset="100%" stopColor="#FB7185" />
        </linearGradient>
      </defs>
      <path
        fill="url(#comicut-top-grad)"
        d="M 124 22 L 196 22 C 212.57 22 226 35.43 226 52 L 226 84 C 226 92.84 218.84 100 210 100 L 158 100 C 146.95 100 138 108.95 138 120 L 138 128 L 80 70 L 96 22 Z"
      />
      <path
        fill="url(#comicut-bot-grad)"
        d="M 30 104 L 88 162 L 88 170 C 88 181.05 96.95 190 108 190 L 160 190 C 168.84 190 176 197.16 176 206 L 176 214 C 176 226.15 166.15 236 154 236 L 74 236 C 50.8 236 32 217.2 32 194 L 32 128 Z"
      />
    </svg>
  );
};
