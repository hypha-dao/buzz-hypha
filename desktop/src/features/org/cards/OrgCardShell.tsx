import { useId, useState, type ReactNode } from "react";
import { Link } from "@tanstack/react-router";

import { proposalVoteMarks } from "@/features/org/cards/parse";
import {
  proposalCardFace,
  proposalClaimLines,
} from "@/features/org/proposalDetail";
import { proposalDestination } from "@/features/org/proposalChat";
import { motionDelay } from "@/features/org/ui/OrgMotionFrame";
import {
  overviewCardDelayMs,
  overviewLineDelayMs,
  overviewRuleDelayMs,
  overviewStampDelayMs,
} from "@/features/org/ui/overview/overviewMotion";
import { formatWorkDate } from "@/features/org/work/model";

import { PlanSteps } from "./PlanSteps";
import { KIND_IO_PROPOSAL } from "@/shared/constants/kinds";
import { cn } from "@/shared/lib/cn";

import { useOrgCardMotionIndex } from "./orgCardMotion";
import type { OrgCardModel, OrgFact, OrgReceipt } from "./types";

type OrgCardShellProps = {
  model: OrgCardModel;
  actions?: ReactNode;
  extra?: ReactNode;
  /** Extra copy under the face. A proposal face opens its page instead. */
  detail?: ReactNode;
};

export function OrgCardShell({
  model,
  actions,
  extra,
  detail,
}: OrgCardShellProps) {
  const [open, setOpen] = useState(false);
  const enterIndex = useOrgCardMotionIndex();
  const lines = linePlan(model);
  const headingId = `org-card-claim-${model.event.id}`;
  const panelId = useId();
  const destination =
    proposalDestination(model.event) ??
    (model.event.kind === KIND_IO_PROPOSAL && model.proposalId
      ? {
          to: "/org/proposal/$proposalId" as const,
          params: { proposalId: model.proposalId },
        }
      : null);
  const votes = proposalVoteMarks(model.event);
  const faceCopy = proposalCardFace(model.event, model.kicker);
  const kindId = faceCopy.label ? `org-card-kind-${model.event.id}` : undefined;
  const askerId = faceCopy.asker
    ? `org-card-asker-${model.event.id}`
    : undefined;
  const labelledBy = [kindId, askerId, headingId].filter(Boolean).join(" ");
  const face = (
    <CardFace
      askerId={askerId}
      clamp={Boolean(detail) && !destination && !open}
      enterIndex={enterIndex}
      face={faceCopy}
      headingId={headingId}
      kindId={kindId}
      lines={lines}
      model={model}
      votes={votes}
    />
  );
  const faceClass = cn(
    "block rounded-lg -mx-1 -mt-1 px-1 pt-1 text-left text-inherit no-underline",
    "hover:bg-muted/40",
    "focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring",
  );
  const article = (
    <article
      aria-labelledby={model.itemId && !detail ? undefined : headingId}
      className={cn(
        "rounded-xl border border-border/60 bg-card p-5 shadow-xs",
        enterIndex !== null && "org-overview-settle",
      )}
      data-card-type={model.cardType}
      data-kicker={model.kickerKind}
      data-testid={`org-card-${model.event.id}`}
      style={
        enterIndex === null
          ? undefined
          : motionDelay(overviewCardDelayMs(enterIndex))
      }
    >
      {destination ? (
        <Link
          aria-labelledby={labelledBy}
          className={cn(faceClass, "w-full")}
          data-testid="org-card-open"
          params={destination.params}
          to={destination.to}
        >
          {face}
        </Link>
      ) : detail ? (
        <button
          aria-controls={panelId}
          aria-expanded={open}
          className={cn(faceClass, "w-full")}
          data-testid="org-card-expand"
          onClick={() => setOpen((current) => !current)}
          type="button"
        >
          {face}
          <span className="mt-3 block text-xs font-medium text-muted-foreground">
            {open ? "Hide the proposal" : "Show the proposal"}
          </span>
        </button>
      ) : model.itemId ? (
        <Link
          aria-labelledby={labelledBy}
          className={faceClass}
          data-testid="org-card-open"
          params={{ itemId: model.itemId }}
          to="/org/work/$itemId"
        >
          {face}
        </Link>
      ) : (
        face
      )}
      {open && detail ? (
        <div
          className="mt-3 border-t border-border/60 pt-3"
          data-testid="org-card-detail"
          id={panelId}
        >
          {detail}
        </div>
      ) : null}
      {extra}
      {actions ? (
        <div
          className={cn(
            "mt-4 flex flex-wrap gap-2",
            enterIndex !== null && "org-dir-stamp",
          )}
          style={
            enterIndex === null
              ? undefined
              : motionDelay(overviewStampDelayMs(enterIndex, lines.count))
          }
        >
          {actions}
        </div>
      ) : null}
    </article>
  );
  if (enterIndex === null) return article;
  return <div className="org-overview-lift h-full min-w-0">{article}</div>;
}

function linePlan(model: OrgCardModel): {
  claim: number;
  parent: number | null;
  due: number | null;
  facts: number | null;
  receipts: number | null;
  count: number;
} {
  let next = 1;
  const parent = model.parentTitle ? next++ : null;
  const due = model.dueAt !== null ? next++ : null;
  const facts = model.facts.length > 0 ? next++ : null;
  const receipts = model.receipts.length > 0 ? next++ : null;
  return { claim: 0, parent, due, facts, receipts, count: next };
}

function CardFace({
  model,
  headingId,
  kindId,
  askerId,
  face,
  clamp,
  enterIndex,
  lines,
  votes,
}: {
  model: OrgCardModel;
  headingId: string;
  kindId?: string;
  askerId?: string;
  face: { label: string; asker: string | null };
  clamp?: boolean;
  enterIndex: number | null;
  lines: ReturnType<typeof linePlan>;
  votes: { cast: number; seats: number } | null;
}) {
  const motion = enterIndex !== null;
  const claimLines =
    model.event.kind === KIND_IO_PROPOSAL
      ? proposalClaimLines(model.event)
      : [];
  const lineStyle = (lineIndex: number | null) => {
    if (enterIndex === null || lineIndex === null) return undefined;
    return motionDelay(overviewLineDelayMs(enterIndex, lineIndex));
  };
  return (
    <div className="flex items-start gap-3">
      <div className="min-w-0 flex-1">
        {face.label ? (
          <p
            className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
            data-testid="org-card-kicker"
            id={kindId}
          >
            {face.label}
          </p>
        ) : null}
        {face.asker ? (
          <p
            className="mt-1 text-xs text-muted-foreground"
            data-testid="org-card-asker"
            id={askerId}
          >
            {face.asker}
          </p>
        ) : null}
        {claimLines.length > 0 ? (
          <h3
            className={cn(
              "space-y-1.5 text-sm font-normal leading-relaxed",
              face.label || face.asker ? "mt-3" : undefined,
              motion && "org-dir-line",
            )}
            data-testid="org-card-claim-lines"
            id={headingId}
            style={lineStyle(lines.claim)}
          >
            {claimLines.map((line) => (
              <span
                className="block rounded-md border border-border/60 bg-background/40 px-3 py-1.5"
                key={line}
              >
                {line}
              </span>
            ))}
          </h3>
        ) : (
          <h3
            className={cn(
              "text-message font-semibold leading-snug",
              face.label || face.asker ? "mt-3" : undefined,
              clamp ? "line-clamp-3" : undefined,
              motion && "org-dir-line",
            )}
            id={headingId}
            style={lineStyle(lines.claim)}
          >
            {model.claim}
          </h3>
        )}
        {enterIndex !== null ? (
          <span
            aria-hidden="true"
            className="org-dir-rule mt-3 block h-px w-10 bg-foreground/45"
            style={motionDelay(overviewRuleDelayMs(enterIndex))}
          />
        ) : null}
        {model.parentTitle ? (
          <p
            className={cn(
              "mt-2 text-xs text-muted-foreground",
              motion && "org-dir-line",
            )}
            data-testid="org-card-project"
            style={lineStyle(lines.parent)}
          >
            Under {model.parentTitle}
          </p>
        ) : null}
        {model.dueAt !== null ? (
          <p
            className={cn(
              "mt-3 text-xs text-muted-foreground",
              motion && "org-dir-line",
            )}
            style={lineStyle(lines.due)}
          >
            <time
              data-testid="org-card-due"
              dateTime={new Date(model.dueAt * 1000).toISOString()}
            >
              {model.itemKind === "project" || model.draftKind === "project"
                ? "Review"
                : "Due"}{" "}
              {formatWorkDate(model.dueAt)}
            </time>
          </p>
        ) : null}
        {model.draftKind === "project" || model.itemKind === "project" ? (
          <PlanSteps content={model.event.content} />
        ) : null}
        {model.facts.length > 0 ? (
          <div
            className={motion ? "org-dir-line" : undefined}
            style={lineStyle(lines.facts)}
          >
            <FactRow facts={model.facts} />
          </div>
        ) : null}
        {model.receipts.length > 0 ? (
          <div
            className={motion ? "org-dir-line" : undefined}
            style={lineStyle(lines.receipts)}
          >
            <ReceiptRow receipts={model.receipts} />
          </div>
        ) : null}
      </div>
      {votes ? <VoteBubbles cast={votes.cast} seats={votes.seats} /> : null}
    </div>
  );
}

function VoteBubbles({ cast, seats }: { cast: number; seats: number }) {
  const dots = Array.from({ length: seats }, (_, seat) => ({
    id: `seat-${seat}`,
    voted: seat < cast,
  }));
  return (
    <div
      aria-label={`${cast} of ${seats} Shapers have voted`}
      className="flex shrink-0 gap-1 pt-1"
      data-testid="org-card-votes"
      role="img"
    >
      {dots.map((dot) => (
        <span
          aria-hidden="true"
          className={cn(
            "h-2 w-2 rounded-full",
            dot.voted ? "bg-emerald-400" : "bg-amber-300",
          )}
          data-testid="org-card-vote"
          data-voted={dot.voted ? "true" : "false"}
          key={dot.id}
        />
      ))}
    </div>
  );
}

function FactRow({ facts }: { facts: OrgFact[] }) {
  return (
    <dl className="mt-3 flex flex-wrap gap-x-4 gap-y-2 text-xs text-muted-foreground">
      {facts.map((fact) => (
        <div className="flex gap-1" key={`${fact.label}:${fact.value}`}>
          <dt className="text-2xs uppercase tracking-wider">{fact.label}</dt>
          <dd>{fact.value}</dd>
        </div>
      ))}
    </dl>
  );
}

function ReceiptRow({ receipts }: { receipts: OrgReceipt[] }) {
  return (
    <ul
      aria-label="Receipts"
      className="mt-2 flex flex-wrap gap-1.5"
      data-testid="org-card-receipts"
    >
      {receipts.map((receipt) => (
        <li key={`${receipt.kind}:${receipt.id}`}>
          <span
            className={cn(
              "inline-flex items-center rounded-full border border-border/70",
              "bg-muted/40 px-2 py-0.5 text-2xs text-muted-foreground",
            )}
          >
            {receipt.label}
          </span>
        </li>
      ))}
    </ul>
  );
}
