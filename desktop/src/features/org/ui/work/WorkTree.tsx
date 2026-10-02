import { Link } from "@tanstack/react-router";

import { useIdentityQuery } from "@/shared/api/hooks";
import type { RelayEvent } from "@/shared/api/types";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { useUserProfileQuery } from "@/features/profile/hooks";
import { cn } from "@/shared/lib/cn";
import { normalizePubkey } from "@/shared/lib/pubkey";
import { UserAvatar } from "@/shared/ui/UserAvatar";

import {
  formatReviewDate,
  formatWorkDate,
  latestHealth,
  workBoardColumn,
  type WorkHealth,
  type WorkItem,
  type WorkTreeNode,
} from "../../work/model";
import { OrgMotionFrame, motionDelay } from "../OrgMotionFrame";
import {
  overviewLineDelayMs,
  overviewRuleDelayMs,
  overviewStampDelayMs,
} from "../overview/overviewMotion";
import { StateChip } from "./StateChip";

const CARD_CLASS =
  "flex h-36 overflow-hidden rounded-xl border border-border bg-card";

type WorkTreeProps = {
  events: readonly RelayEvent[];
  nodes: WorkTreeNode[];
};

export function WorkTree({ events, nodes }: WorkTreeProps) {
  const waiting = nodes.filter(
    (node) => workBoardColumn(node.item) === "waiting",
  );
  const ongoing = nodes.filter(
    (node) => workBoardColumn(node.item) === "ongoing",
  );

  return (
    <div data-testid="org-work-tree">
      <p
        className="org-dir-line text-2xs font-medium uppercase tracking-wider text-muted-foreground"
        style={motionDelay(overviewLineDelayMs(0, 0))}
      >
        Who is working on what
      </p>
      <span
        aria-hidden="true"
        className="org-dir-rule mt-3 block h-px w-10 bg-foreground/45"
        style={motionDelay(overviewRuleDelayMs(0))}
      />
      <div className="mt-5 grid items-start gap-8 lg:grid-cols-2">
        <WorkColumn
          events={events}
          nodes={waiting}
          testId="org-work-column-waiting"
          title="Not accepted yet"
        />
        <WorkColumn
          events={events}
          nodes={ongoing}
          showOpen
          testId="org-work-column-ongoing"
          title="Ongoing"
        />
      </div>
    </div>
  );
}

function WorkColumn({
  events,
  nodes,
  showOpen = false,
  testId,
  title,
}: {
  events: readonly RelayEvent[];
  nodes: WorkTreeNode[];
  showOpen?: boolean;
  testId: string;
  title: string;
}) {
  return (
    <section aria-labelledby={testId} data-testid={testId}>
      <h2
        className="org-dir-line text-2xs font-medium uppercase tracking-wider text-muted-foreground"
        id={testId}
        style={motionDelay(overviewLineDelayMs(0, 0))}
      >
        {title}
      </h2>
      {nodes.length === 0 ? (
        <p
          className="org-dir-line mt-3 text-sm text-muted-foreground"
          style={motionDelay(overviewLineDelayMs(0, 1))}
        >
          None yet.
        </p>
      ) : (
        <ul className="mt-3 grid grid-cols-1 gap-3">
          {nodes.map((node, index) => (
            <li key={node.item.id}>
              <OrgMotionFrame className={CARD_CLASS} enterIndex={index}>
                <WorkTreeRow
                  depth={0}
                  enterIndex={index}
                  events={events}
                  item={node.item}
                  showOpen={showOpen}
                />
              </OrgMotionFrame>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function WorkTreeRow({
  item,
  depth,
  enterIndex,
  events,
  showOpen = false,
}: {
  item: WorkItem;
  depth: number;
  enterIndex: number;
  events: readonly RelayEvent[];
  showOpen?: boolean;
}) {
  const holder = item.state === "offered" ? item.offeredTo : item.dri;
  const health =
    depth === 0 && workBoardColumn(item) === "ongoing"
      ? latestHealth(events, item.id)
      : null;

  return (
    <Link
      className="flex h-full w-full flex-col px-5 py-4 hover:bg-muted/40 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
      data-depth={depth}
      data-testid={`org-work-row-${item.id}`}
      params={{ itemId: item.id }}
      to="/org/work/$itemId"
    >
      <div className="flex items-start justify-between gap-4">
        <p
          className="org-dir-line min-w-0 truncate text-base font-semibold leading-snug"
          style={motionDelay(overviewLineDelayMs(enterIndex, 0))}
        >
          {item.title}
        </p>
        <div className="flex shrink-0 items-center gap-3 pt-0.5">
          {health ? <HealthMark health={health} /> : null}
          <StateChip item={item} />
          {showOpen ? (
            <span className="text-sm font-medium">Open →</span>
          ) : null}
        </div>
      </div>
      <span
        aria-hidden="true"
        className="org-dir-rule mt-2 block h-px w-10 bg-foreground/45"
        style={motionDelay(overviewRuleDelayMs(enterIndex))}
      />
      <div
        className="org-dir-line mt-2 flex h-9 items-center"
        style={motionDelay(overviewLineDelayMs(enterIndex, 1))}
      >
        {holder ? (
          <PersonLine
            pubkey={holder}
            testId={`org-work-row-holder-${item.id}`}
          />
        ) : null}
      </div>
      <div
        className="org-dir-stamp mt-auto pt-2"
        style={motionDelay(overviewStampDelayMs(enterIndex, 2))}
      >
        <ReviewLine depth={depth} item={item} />
      </div>
    </Link>
  );
}

function ReviewLine({ item, depth }: { item: WorkItem; depth: number }) {
  if (item.type === "project" || depth === 0) {
    if (item.dueAt === null) {
      return (
        <p
          className="text-sm text-muted-foreground"
          data-testid={`org-work-row-review-${item.id}`}
        >
          review not set
        </p>
      );
    }
    return (
      <p className="text-sm text-muted-foreground">
        <time
          data-testid={`org-work-row-due-${item.id}`}
          dateTime={iso(item.dueAt)}
        >
          review {formatReviewDate(item.dueAt)}
        </time>
      </p>
    );
  }

  if (item.dueAt === null) return null;
  return (
    <p className="text-sm text-muted-foreground">
      <time
        data-testid={`org-work-row-due-${item.id}`}
        dateTime={iso(item.dueAt)}
      >
        due {formatWorkDate(item.dueAt)}
      </time>
    </p>
  );
}

function PersonLine({ pubkey, testId }: { pubkey: string; testId: string }) {
  const identity = useIdentityQuery().data?.pubkey ?? null;
  const profile = useUserProfileQuery(pubkey).data;
  const name = resolveUserLabel({
    pubkey,
    currentPubkey: identity ?? undefined,
    preferResolvedSelfLabel: true,
    profiles: profile
      ? {
          [normalizePubkey(pubkey)]: {
            displayName: profile.displayName,
            avatarUrl: profile.avatarUrl,
            nip05Handle: profile.nip05Handle,
            ownerPubkey: profile.ownerPubkey,
          },
        }
      : undefined,
  });

  return (
    <p className="flex items-center gap-2 text-sm">
      <span aria-hidden="true" className="inline-flex shrink-0">
        <UserAvatar
          avatarUrl={profile?.avatarUrl ?? null}
          displayName={name}
          size="md"
        />
      </span>
      <span data-pubkey={pubkey} data-testid={testId}>
        {name}
      </span>
    </p>
  );
}

function HealthMark({ health }: { health: WorkHealth }) {
  const band = health.band.trim();
  if (!band) return null;
  const label = band.charAt(0).toUpperCase() + band.slice(1);
  return (
    <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
      <span
        aria-hidden="true"
        className={cn("h-2 w-2 rounded-full", healthDotClass(band))}
      />
      {label}
    </span>
  );
}

function healthDotClass(band: string): string {
  if (band === "healthy") return "bg-foreground";
  if (band === "wobbly") return "bg-foreground/45";
  if (band === "struggling") return "bg-foreground/20";
  return "bg-muted-foreground";
}

function iso(unix: number): string {
  return new Date(unix * 1000).toISOString();
}
