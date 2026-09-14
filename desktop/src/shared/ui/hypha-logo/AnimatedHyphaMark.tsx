import { HyphaMark } from "./HyphaMark";

/**
 * The Hypha mark, alive: the segmented ring turns slowly while a loading gate
 * is on screen. The same silhouette as the static {@link HyphaMark}, rendered
 * in `currentColor` so it tints per theme.
 *
 * The rotation animates an HTML-level wrapper, not the SVG itself. This is
 * deliberate: WebKit paints SVG *children* on the main thread, so a transform
 * animation inside the `<svg>` freezes for as long as boot work (bundle eval,
 * first React render of the app tree) hogs the thread — exactly the window in
 * which the loading gate is visible. Transforms on HTML-level elements run on
 * the compositor (Core Animation in WKWebView) and keep turning regardless.
 *
 * Everything is plain SVG + CSS (no JS/SMIL), so it paints on the very first
 * frame and the motion starts as soon as styles load. Reduced motion falls
 * back to the static mark via the CSS media query (see animations.css).
 */
export function AnimatedHyphaMark({ className }: { className?: string }) {
  return (
    <div
      aria-hidden="true"
      className={["hypha-mark-live", "relative", "aspect-square", className]
        .filter(Boolean)
        .join(" ")}
    >
      <HyphaMark className="block h-full w-full" />
    </div>
  );
}
