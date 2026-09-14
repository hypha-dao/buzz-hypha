import { useId, useLayoutEffect, useRef } from "react";
import type { CSSProperties } from "react";
import "./hypha-logo-animation.css";
import { HYPHA_MARK_SEGMENTS, HYPHA_MARK_VIEWBOX } from "./hyphaMarkGeometry";

const LOOP = "indefinite";
const EASE = ".16 1 .3 1";

/** Native length of one assemble pass, in seconds. */
const MORPH_SECONDS = 1.6;
/** Fraction of the pass during which a single segment fades in. */
const SEGMENT_FADE = 0.3;

type TextureConfig = {
  blur: string;
  displacement: string;
  frequency: string;
  frequencyValues: string;
  grainAlpha: string;
  seedValues: string;
};

export type HyphaLogoAnimationProps = {
  ariaLabel?: string;
  className?: string;
  fullScreen?: boolean;
  loop?: boolean;
  /**
   * When looping, hide the mark for this many seconds between plays. The
   * assemble runs at its native speed, then the mark disappears for the rest
   * window before the cycle repeats. Only applies when `loop` is true.
   */
  loopRestSeconds?: number;
  /** Assemble the ring counter-clockwise instead of clockwise. */
  reverse?: boolean;
  showBackground?: boolean;
  style?: CSSProperties;
  /** When false, skips the looping feTurbulence texture filter (CPU-heavy). */
  textured?: boolean;
};

// The mark lives in a 35-unit box, so the texture parameters are scaled down
// from the old 466-unit canvas to keep the same visual weight.
const TEXTURE: TextureConfig = {
  blur: "0.5",
  displacement: "0.7",
  frequency: "1.06",
  frequencyValues: "1;1.18;0.96;1.24;1",
  grainAlpha: ".5",
  seedValues: "6;21;11;27;6",
};

function idPart(value: string) {
  return value.replace(/[^a-zA-Z0-9_-]/g, "");
}

function splines(keyTimes: string) {
  return Array.from(
    { length: keyTimes.split(";").length - 1 },
    () => EASE,
  ).join(";");
}

/**
 * Opacity keyframes that reveal segment `index` of `count` in sequence. The
 * whole sequence occupies the first `morphFraction` of the cycle; the rest of
 * the cycle holds the finished mark (the rest-window fade hides it).
 */
function segmentTiming(index: number, count: number, morphFraction: number) {
  const stagger = count > 1 ? (1 - SEGMENT_FADE) / (count - 1) : 0;
  const start = index * stagger * morphFraction;
  const end = Math.min((index * stagger + SEGMENT_FADE) * morphFraction, 1);
  const keyTimes = [
    "0",
    start.toFixed(4),
    end.toFixed(4),
    ...(end < 1 ? ["1"] : []),
  ].join(";");
  const values = end < 1 ? "0;0;1;1" : "0;0;1";
  return { keyTimes, values };
}

function TextureFilter({ id }: { id: string }) {
  return (
    <filter
      id={id}
      x="-8"
      y="-8"
      width="51"
      height="51"
      filterUnits="userSpaceOnUse"
      colorInterpolationFilters="sRGB"
    >
      <feGaussianBlur
        in="SourceGraphic"
        stdDeviation={TEXTURE.blur}
        result="softLogo"
      />
      <feTurbulence
        type="fractalNoise"
        baseFrequency={TEXTURE.frequency}
        numOctaves="5"
        seed="7"
        result="textureNoise"
      >
        <animate
          attributeName="baseFrequency"
          begin="indefinite"
          dur="0.34s"
          repeatCount={LOOP}
          values={TEXTURE.frequencyValues}
        />
        <animate
          attributeName="seed"
          begin="indefinite"
          dur="0.34s"
          repeatCount={LOOP}
          values={TEXTURE.seedValues}
        />
      </feTurbulence>
      <feDisplacementMap
        in="softLogo"
        in2="textureNoise"
        scale={TEXTURE.displacement}
        xChannelSelector="R"
        yChannelSelector="G"
        result="fuzzedLogo"
      />
      <feColorMatrix
        in="textureNoise"
        type="matrix"
        values={`0 0 0 0 0
                0 0 0 0 0
                0 0 0 0 0
                ${TEXTURE.grainAlpha} ${TEXTURE.grainAlpha} ${TEXTURE.grainAlpha} 0 0`}
        result="grainAlpha"
      />
      <feComposite
        in="fuzzedLogo"
        in2="grainAlpha"
        operator="in"
        result="grainLogo"
      />
      <feMerge>
        <feMergeNode in="fuzzedLogo" />
        <feMergeNode in="grainLogo" />
      </feMerge>
    </filter>
  );
}

/**
 * Hides the parent group during the rest window of a stretched loop cycle:
 * fully visible while the assemble plays, then a quick fade to invisible for
 * the remainder of the cycle. SMIL `<animate>` targets its parent element.
 */
function RestWindowFade({
  cycleSeconds,
  morphSeconds,
  repeatCount,
}: {
  cycleSeconds: number;
  morphSeconds: number;
  repeatCount: string;
}) {
  const fadeSeconds = 0.15;
  const visibleEnd = morphSeconds / cycleSeconds;
  const fadeEnd = Math.min((morphSeconds + fadeSeconds) / cycleSeconds, 1);
  const keyTimes = ["0", visibleEnd.toFixed(4), fadeEnd.toFixed(4), "1"].join(
    ";",
  );

  return (
    <animate
      attributeName="opacity"
      begin="indefinite"
      calcMode="linear"
      dur={`${cycleSeconds}s`}
      fill={repeatCount === LOOP ? "remove" : "freeze"}
      keyTimes={keyTimes}
      repeatCount={repeatCount}
      values="1;1;0;0"
    />
  );
}

/**
 * The Hypha mark assembling segment by segment — the animated counterpart of
 * the static {@link HyphaMark}. Drives SMIL `<animate>` elements so it needs
 * no JavaScript per frame; the optional texture filter adds a fuzzy grain.
 */
export default function HyphaLogoAnimation({
  ariaLabel = "Hypha logo animation",
  className = "",
  fullScreen = true,
  loop = false,
  loopRestSeconds = 0,
  reverse = false,
  showBackground = true,
  style,
  textured = true,
}: HyphaLogoAnimationProps) {
  const markRef = useRef<SVGSVGElement>(null);
  const idSuffix = idPart(useId());
  const restSeconds = loop ? Math.max(loopRestSeconds, 0) : 0;
  const cycleSeconds = MORPH_SECONDS + restSeconds;
  // Stretch the loop period to assemble + rest, packing the reveal keyframes
  // into the start of the cycle at native speed. The rest-window opacity
  // animation below hides the finished mark for the remainder.
  const morphFraction = MORPH_SECONDS / cycleSeconds;
  const duration = `${cycleSeconds}s`;
  const repeatCount = loop ? LOOP : "1";
  const textureId = `hypha-logo-texture-${idSuffix}`;
  const segments = reverse
    ? [...HYPHA_MARK_SEGMENTS].reverse()
    : HYPHA_MARK_SEGMENTS;
  const classes = [
    "hypha-logo",
    fullScreen && "hypha-logo--screen",
    !fullScreen && "hypha-logo--compact",
    showBackground && "hypha-logo--background",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  // biome-ignore lint/correctness/useExhaustiveDependencies: restart SMIL when visual props change
  useLayoutEffect(() => {
    const svg = markRef.current;

    if (!svg || typeof svg.setCurrentTime !== "function") {
      return;
    }

    svg.pauseAnimations?.();
    svg.setCurrentTime?.(0);
    svg.querySelectorAll("animate").forEach((animation) => {
      animation.beginElement?.();
    });
    svg.unpauseAnimations?.();
  }, [loop, reverse, restSeconds, textured]);

  return (
    <div className={classes} style={style} role="img" aria-label={ariaLabel}>
      <svg
        ref={markRef}
        className="hypha-logo__mark"
        viewBox={HYPHA_MARK_VIEWBOX}
        width="320"
        height="320"
        aria-hidden="true"
      >
        <defs>{textured && <TextureFilter id={textureId} />}</defs>
        <g filter={textured ? `url(#${textureId})` : undefined}>
          {restSeconds > 0 ? (
            <RestWindowFade
              cycleSeconds={cycleSeconds}
              morphSeconds={MORPH_SECONDS}
              repeatCount={repeatCount}
            />
          ) : null}
          {segments.map((segment, index) => {
            const { keyTimes, values } = segmentTiming(
              index,
              segments.length,
              morphFraction,
            );
            return (
              <path
                key={segment.d.slice(0, 24)}
                className="hypha-logo__ink"
                d={segment.d}
                opacity="0"
              >
                <animate
                  attributeName="opacity"
                  begin="indefinite"
                  calcMode="spline"
                  dur={duration}
                  fill={repeatCount === LOOP ? "remove" : "freeze"}
                  keySplines={splines(keyTimes)}
                  keyTimes={keyTimes}
                  repeatCount={repeatCount}
                  values={values}
                />
              </path>
            );
          })}
        </g>
      </svg>
    </div>
  );
}
