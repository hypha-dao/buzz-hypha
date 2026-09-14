import { HYPHA_MARK_SEGMENTS, HYPHA_MARK_VIEWBOX } from "./hyphaMarkGeometry";

export type HyphaMarkProps = {
  className?: string;
  /**
   * `mono` (default) paints every segment in `currentColor` so the mark tints
   * with the surrounding theme. `brand` paints the official Hypha palette.
   */
  variant?: "mono" | "brand";
};

/**
 * The Hypha mark as a plain static SVG — no SMIL, no scripting, no animation
 * machinery — so it paints complete on the very first frame regardless of
 * animation support. Geometry comes from the official Hypha wordmark.
 */
export function HyphaMark({ className, variant = "mono" }: HyphaMarkProps) {
  return (
    <svg
      aria-hidden="true"
      className={["hypha-mark", className].filter(Boolean).join(" ")}
      viewBox={HYPHA_MARK_VIEWBOX}
      fill={variant === "mono" ? "currentColor" : undefined}
    >
      {HYPHA_MARK_SEGMENTS.map((segment) => (
        <path
          key={segment.d.slice(0, 24)}
          d={segment.d}
          fill={variant === "brand" ? segment.fill : undefined}
        />
      ))}
    </svg>
  );
}
