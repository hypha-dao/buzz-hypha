import * as React from "react";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { ChatHeader } from "@/features/chat/ui/ChatHeader";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { TopChromeInsetHeader } from "@/shared/layout/TopChromeInsetHeader";
import { Button } from "@/shared/ui/button";

import type { DirectionSlug } from "../commands";
import { useLiveDoorEvents } from "../hooks";

import { DIRECTION_LABEL, NOT_SET_YET } from "./overview/overviewCopy";
import { directionPageFilters } from "./overview/overviewExtraFilters";
import { motionDelay } from "./OrgMotionFrame";
import { OrgPersonName } from "./overview/OrgPersonName";
import {
  overviewCardDelayMs,
  overviewLineDelayMs,
  overviewRuleDelayMs,
} from "./overview/overviewMotion";
import {
  DIRECTION_SLUGS,
  directionHistory,
  directionSlots,
} from "./overview/parseOverview";

export function isDirectionSlug(value: string): value is DirectionSlug {
  return (DIRECTION_SLUGS as readonly string[]).includes(value);
}

/** Direction page — Protocol §6.5 read of the head and passed proposals. */
export function DirectionScreen({ slug }: { slug: string }) {
  const navigation = useAppNavigation();
  const valid = isDirectionSlug(slug) ? slug : null;
  const { events } = useLiveDoorEvents(
    valid ? directionPageFilters(valid) : [],
  );
  const head = React.useMemo(() => {
    if (!valid) return null;
    return (
      directionSlots(events).find((slot) => slot.slug === valid)?.head ?? null
    );
  }, [events, valid]);
  const versions = React.useMemo(
    () => (valid ? directionHistory(events, valid) : []),
    [events, valid],
  );
  const pubkeys = React.useMemo(() => {
    const set = new Set<string>();
    if (head?.confirmedBy) set.add(head.confirmedBy);
    for (const version of versions) {
      if (version.confirmedBy) set.add(version.confirmedBy);
    }
    return [...set];
  }, [head, versions]);
  const profiles = useUsersBatchQuery(pubkeys).data?.profiles ?? {};
  const label = valid ? DIRECTION_LABEL[valid] : null;

  return (
    <div
      className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
      data-testid="org-direction"
    >
      <TopChromeInsetHeader data-tauri-drag-region flush>
        <ChatHeader mode="org" title={label ? label.title : "Direction"} />
      </TopChromeInsetHeader>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
        <div className="mx-auto flex max-w-2xl flex-col gap-4">
          <Button
            className="w-fit px-0"
            data-testid="org-direction-back"
            onClick={() => {
              void navigation.goOrg();
            }}
            type="button"
            variant="link"
          >
            Overview
          </Button>
          {label ? (
            <div>
              <h1
                className="org-dir-line text-xl font-semibold"
                style={motionDelay(overviewLineDelayMs(0, 0))}
              >
                {label.title}
                <span className="font-normal text-muted-foreground">
                  {" "}
                  — {label.question}
                </span>
              </h1>
              {head ? (
                <p
                  className="org-dir-line mt-1 text-2xs text-muted-foreground"
                  style={motionDelay(overviewLineDelayMs(0, 1))}
                >
                  v{head.version}
                  {head.confirmedBy ? (
                    <>
                      {" "}
                      · confirmed by{" "}
                      <OrgPersonName
                        profiles={profiles}
                        pubkey={head.confirmedBy}
                      />
                    </>
                  ) : null}
                </p>
              ) : (
                <p
                  className="org-dir-line mt-2 text-sm text-muted-foreground"
                  style={motionDelay(overviewLineDelayMs(0, 1))}
                >
                  {NOT_SET_YET}
                </p>
              )}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">{NOT_SET_YET}</p>
          )}
          {head ? (
            <div className="space-y-3" data-testid="org-direction-body">
              <span
                aria-hidden="true"
                className="org-dir-rule block h-px w-10 bg-foreground/45"
                style={motionDelay(overviewRuleDelayMs(0))}
              />
              {head.lines.length > 0 ? (
                <ol className="list-decimal space-y-2 pl-5">
                  {head.lines.map((line, index) => (
                    <li
                      className="org-dir-line text-message leading-relaxed"
                      key={line.id}
                      style={motionDelay(overviewLineDelayMs(1, index))}
                    >
                      {line.text}
                    </li>
                  ))}
                </ol>
              ) : (
                <p
                  className="org-dir-line text-message leading-relaxed"
                  style={motionDelay(overviewLineDelayMs(1, 0))}
                >
                  {head.body}
                </p>
              )}
            </div>
          ) : null}
          <section aria-labelledby="org-direction-history-heading">
            <h2
              className="org-dir-line text-2xs font-medium uppercase tracking-wider text-muted-foreground"
              id="org-direction-history-heading"
              style={motionDelay(overviewLineDelayMs(2, 0))}
            >
              Versions — every one confirmed by a Shaper
            </h2>
            {versions.length === 0 ? (
              <p className="mt-2 text-sm text-muted-foreground">
                {NOT_SET_YET}
              </p>
            ) : (
              <ol
                className="mt-2 space-y-2"
                data-testid="org-direction-history"
              >
                {versions.map((version, index) => (
                  <li key={version.proposal}>
                    <div className="org-overview-lift">
                      <div
                        className="org-overview-settle rounded-lg border border-border/70 px-3 py-2"
                        style={motionDelay(overviewCardDelayMs(index + 1))}
                      >
                        <p className="text-2xs text-muted-foreground">
                          v{version.version}
                          {version.confirmedBy ? (
                            <>
                              {" "}
                              · confirmed by{" "}
                              <OrgPersonName
                                profiles={profiles}
                                pubkey={version.confirmedBy}
                              />
                            </>
                          ) : null}
                        </p>
                        <p className="mt-1 text-sm">{version.body}</p>
                      </div>
                    </div>
                  </li>
                ))}
              </ol>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}
