import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/shared/ui/tooltip";

import type { WorkHealth } from "../../work/model";

type HealthCardProps = {
  health: WorkHealth;
};

const BAND_LEFT: Record<string, string> = {
  struggling: "12%",
  wobbly: "50%",
  healthy: "88%",
};

export function HealthCard({ health }: HealthCardProps) {
  const left = BAND_LEFT[health.band] ?? `${Math.round(health.pct * 100)}%`;

  return (
    <section
      aria-labelledby="org-health-heading"
      className="rounded-xl border border-border bg-card p-4"
      data-band={health.band}
      data-testid="org-health-card"
    >
      <div className="flex items-center justify-between gap-3">
        <h2
          className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
          id="org-health-heading"
        >
          Project health — the agent's read
        </h2>
        <p
          className="text-sm font-semibold capitalize"
          data-testid="org-health-band"
        >
          {health.band}
        </p>
      </div>

      <div
        aria-hidden="true"
        className="relative mt-4 mb-1 h-2.5 rounded-full bg-gradient-to-r from-red-400 via-amber-300 to-emerald-500"
      >
        <span
          className="absolute top-1/2 h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-foreground bg-background"
          style={{ left }}
        />
      </div>
      <div className="flex justify-between text-2xs text-muted-foreground">
        <span>struggling</span>
        <span>wobbly</span>
        <span>healthy</span>
      </div>

      <TooltipProvider delayDuration={200}>
        <ul className="mt-4 space-y-2">
          {health.sentences.map((sentence) => (
            <li key={`${sentence.text}:${sentence.rows.join(",")}`}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    className="w-full rounded-md text-left text-sm leading-relaxed hover:bg-muted/60 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                    data-testid="org-health-sentence"
                    type="button"
                  >
                    {sentence.text}
                  </button>
                </TooltipTrigger>
                <TooltipContent
                  className="max-w-xs"
                  data-testid="org-health-rows"
                  side="bottom"
                >
                  {sentence.rows.length === 0 ? (
                    <p>No rows</p>
                  ) : (
                    <ul>
                      {sentence.rows.map((row) => (
                        <li className="break-all font-mono text-2xs" key={row}>
                          {row}
                        </li>
                      ))}
                    </ul>
                  )}
                </TooltipContent>
              </Tooltip>
            </li>
          ))}
        </ul>
      </TooltipProvider>
    </section>
  );
}
