import type { CSSProperties } from "react";

import type { DirectionSlug } from "../../commands";

const STROKE = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.6,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

/** Small line-drawn glyph that strokes in with the direction card. */
export function DirectionMark({
  kind,
  style,
}: {
  kind: DirectionSlug;
  style?: CSSProperties;
}) {
  return (
    <svg
      aria-hidden="true"
      className="org-dir-mark mt-0.5 shrink-0 text-muted-foreground"
      height={18}
      style={style}
      viewBox="0 0 16 16"
      width={18}
    >
      {kind === "mission" && (
        <>
          <circle cx="8" cy="8" r="6.2" {...STROKE} />
          <path d="M8 3.6 L9.6 8 L8 12.4 L6.4 8 Z" {...STROKE} />
        </>
      )}
      {kind === "vision" && (
        <>
          <path d="M2 11.5 H14" {...STROKE} />
          <path d="M4.5 11.5 A3.5 3.5 0 0 1 11.5 11.5" {...STROKE} />
          <path d="M8 4 V5.6 M4 6.2 L5 7.2 M12 6.2 L11 7.2" {...STROKE} />
        </>
      )}
      {kind === "situation" && (
        <>
          <path
            d="M8 14 C8 14, 3.2 9.4, 3.2 6.4 A4.8 4.8 0 0 1 12.8 6.4 C12.8 9.4, 8 14, 8 14 Z"
            {...STROKE}
          />
          <circle cx="8" cy="6.4" r="1.6" {...STROKE} />
        </>
      )}
      {kind === "objectives" && (
        <>
          <path d="M3 4.5 H13" {...STROKE} />
          <path d="M3 8 H13" {...STROKE} />
          <path d="M3 11.5 H9" {...STROKE} />
        </>
      )}
      {kind === "strategy" && (
        <>
          <path d="M3 13 C3 8, 8 9, 8 6 C8 3.5, 12 4, 13 3" {...STROKE} />
          <circle cx="3" cy="13" r="1" {...STROKE} />
          <circle cx="13" cy="3" r="1" {...STROKE} />
        </>
      )}
    </svg>
  );
}
