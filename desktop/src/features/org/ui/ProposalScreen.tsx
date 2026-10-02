import * as React from "react";
import { Link, useNavigate } from "@tanstack/react-router";

import { useLiveDoorEvents } from "@/features/org/hooks";
import { proposalChatLink } from "@/features/org/proposalChat";
import { proposalDetail } from "@/features/org/proposalDetail";
import { formatWorkDate } from "@/features/org/work/model";
import { KIND_IO_PROPOSAL } from "@/shared/constants/kinds";
import type { RelayEvent } from "@/shared/api/types";
import { Button } from "@/shared/ui/button";

import { useCardCommand } from "../cards/useCardCommand";
import { ORG_HISTORY_LIMIT } from "../hooks/filters";
import { OrgMotionFrame, motionDelay } from "./OrgMotionFrame";
import {
  overviewLineDelayMs,
  overviewRuleDelayMs,
  overviewStampDelayMs,
} from "./overview/overviewMotion";
import { HolderName } from "./work/HolderName";
import { ORG_EMPTY_NOT_SET_YET, OrgDoorScreen } from "./OrgDoorScreen";

/** A proposal, full page. A passed project continues on the work item page. */
export function ProposalScreen({ proposalId }: { proposalId: string }) {
  const navigate = useNavigate();
  const filters = React.useMemo(
    () => [
      {
        kinds: [KIND_IO_PROPOSAL],
        "#d": [proposalId],
        limit: ORG_HISTORY_LIMIT,
      },
    ],
    [proposalId],
  );
  const { events, isLoading } = useLiveDoorEvents(filters);
  const event = newest(events);
  const detail = event ? proposalDetail(event) : null;
  const opened = event ? readProposal(event) : null;
  const link = event ? proposalChatLink(event) : null;
  const passedItemId = link?.kind === "project" ? link.itemId : null;
  const { commands, error, pending, publish } = useCardCommand();
  const busy = pending !== null;

  React.useEffect(() => {
    if (!passedItemId) return;
    void navigate({
      to: "/org/work/$itemId",
      params: { itemId: passedItemId },
      replace: true,
    });
  }, [navigate, passedItemId]);

  if (!isLoading && !event) {
    return (
      <OrgDoorScreen
        empty={ORG_EMPTY_NOT_SET_YET}
        testId="org-proposal"
        title="Work"
      />
    );
  }

  const title = detail?.title || opened?.title || "Proposal";
  const brief = detail?.body || opened?.brief || "";
  const dueAt = detail?.dueAt ?? opened?.dueAt ?? null;
  const status = opened?.status ?? "open";
  const canVote = status === "open" && Boolean(event);

  return (
    <OrgDoorScreen testId="org-proposal" title="Work">
      <div className="min-h-0 flex-1 overflow-y-auto">
        <article className="mx-auto flex w-full max-w-3xl flex-col gap-10 px-8 py-10">
          <nav aria-label="Breadcrumb" data-testid="org-proposal-breadcrumb">
            <ol className="flex flex-wrap items-center gap-1 text-2xs text-muted-foreground">
              <li>
                <Link
                  className="rounded-sm hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                  to="/org/work"
                >
                  Work
                </Link>
              </li>
              <li className="flex items-center gap-1">
                <span aria-hidden="true">/</span>
                <span aria-current="page" className="text-foreground">
                  {title}
                </span>
              </li>
            </ol>
          </nav>

          <header className="org-overview-settle space-y-3">
            <p className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
              {opened?.kind === "project" || detail?.kind === "project"
                ? "Project"
                : "Proposal"}
            </p>
            <div className="flex flex-wrap items-start justify-between gap-3">
              <h1
                className="org-dir-line text-xl font-semibold tracking-tight"
                data-testid="org-proposal-title"
                style={motionDelay(overviewLineDelayMs(0, 0))}
              >
                {title}
              </h1>
              <span className="rounded-full border border-border px-2 py-0.5 text-2xs text-muted-foreground">
                {statusLabel(status)}
              </span>
            </div>
          </header>

          {brief ? (
            <section className="org-overview-settle space-y-3">
              <h2 className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                Brief
              </h2>
              <span
                aria-hidden="true"
                className="org-dir-rule block h-px w-10 bg-foreground/45"
                style={motionDelay(overviewRuleDelayMs(1))}
              />
              <p
                className="org-dir-line whitespace-pre-wrap text-sm leading-relaxed"
                data-testid="org-proposal-brief"
                style={motionDelay(overviewLineDelayMs(1, 0))}
              >
                {brief}
              </p>
            </section>
          ) : null}

          <dl className="grid gap-4 sm:grid-cols-2">
            <Fact enterIndex={2} label="Holds it">
              {opened?.suggestedDri ? (
                <HolderName pubkey={opened.suggestedDri} />
              ) : (
                "Not yet"
              )}
            </Fact>
            <Fact enterIndex={3} label="Review">
              {dueAt !== null ? (
                <time dateTime={new Date(dueAt * 1000).toISOString()}>
                  {formatWorkDate(dueAt)}
                </time>
              ) : (
                "Not set"
              )}
            </Fact>
          </dl>

          {canVote ? (
            <div
              className="org-dir-stamp flex flex-wrap gap-2"
              style={motionDelay(overviewStampDelayMs(0, 2))}
            >
              <Button
                data-testid="org-proposal-agree"
                disabled={busy}
                onClick={() =>
                  void publish(
                    "Agree",
                    commands.buildIoVote({
                      proposal: proposalId,
                      vote: "agree",
                    }),
                  )
                }
                type="button"
              >
                Agree
              </Button>
              <Button
                data-testid="org-proposal-decline"
                disabled={busy}
                onClick={() =>
                  void publish(
                    "Decline",
                    commands.buildIoVote({
                      proposal: proposalId,
                      vote: "decline",
                    }),
                  )
                }
                type="button"
                variant="secondary"
              >
                Decline
              </Button>
            </div>
          ) : null}
          {error ? (
            <p className="text-sm text-destructive" role="alert">
              {error}
            </p>
          ) : null}
        </article>
      </div>
    </OrgDoorScreen>
  );
}

function newest(events: readonly RelayEvent[]): RelayEvent | null {
  let best: RelayEvent | null = null;
  for (const event of events) {
    if (!best || event.created_at >= best.created_at) best = event;
  }
  return best;
}

function readProposal(event: RelayEvent): {
  kind: string;
  status: string;
  title: string;
  brief: string;
  dueAt: number | null;
  suggestedDri: string | null;
} | null {
  let content: Record<string, unknown> | null = null;
  try {
    const value: unknown = JSON.parse(event.content);
    if (value && typeof value === "object" && !Array.isArray(value)) {
      content = value as Record<string, unknown>;
    }
  } catch {
    content = null;
  }
  if (!content) return null;
  const payload =
    content.payload &&
    typeof content.payload === "object" &&
    !Array.isArray(content.payload)
      ? (content.payload as Record<string, unknown>)
      : null;
  const due = payload?.due_at;
  const suggested = payload?.suggested_dri;
  return {
    kind: typeof content.kind === "string" ? content.kind : "proposal",
    status: typeof content.status === "string" ? content.status : "open",
    title: text(payload?.title) || text(payload?.body) || "Proposal",
    brief: text(payload?.brief) || text(payload?.body),
    dueAt: typeof due === "number" && Number.isFinite(due) ? due : null,
    suggestedDri: typeof suggested === "string" ? suggested : null,
  };
}

function text(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function statusLabel(status: string): string {
  if (status === "open") return "needs a vote";
  if (status === "passed") return "passed";
  return status;
}

function Fact({
  children,
  enterIndex,
  label,
}: {
  children: React.ReactNode;
  enterIndex: number;
  label: string;
}) {
  return (
    <OrgMotionFrame
      className="rounded-xl border border-border px-4 py-4"
      enterIndex={enterIndex}
    >
      <dt className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </dt>
      <dd className="mt-3 text-sm font-medium">{children}</dd>
    </OrgMotionFrame>
  );
}
