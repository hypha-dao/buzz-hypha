import * as React from "react";
import { Link } from "@tanstack/react-router";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { useIdentityQuery } from "@/shared/api/hooks";
import { Button } from "@/shared/ui/button";

import { useOrgCommands } from "../../useOrgCommands";
import { OrgMotionFrame, motionDelay } from "../OrgMotionFrame";
import {
  overviewCardDelayMs,
  overviewLineDelayMs,
  overviewRuleDelayMs,
} from "../overview/overviewMotion";
import {
  canChangeTicketDue,
  canMarkDone,
  formatChildrenCounts,
  formatWorkDate,
  homeChannel,
  type WorkHealth,
  type WorkItem,
} from "../../work/model";
import { PlanSteps } from "../../cards/PlanSteps";
import { HealthCard } from "./HealthCard";
import { HolderName } from "./HolderName";
import { StateChip } from "./StateChip";

type WorkItemViewProps = {
  item: WorkItem;
  parent: WorkItem | null;
  childItems: WorkItem[];
  health: WorkHealth | null;
};

export function WorkItemView({
  item,
  parent,
  childItems,
  health,
}: WorkItemViewProps) {
  const viewer = useIdentityQuery().data?.pubkey ?? null;
  const commands = useOrgCommands();
  const { goChannel } = useAppNavigation();
  const [error, setError] = React.useState<string | null>(null);
  const [dueDraft, setDueDraft] = React.useState("");
  const [editingDue, setEditingDue] = React.useState(false);
  const [busy, setBusy] = React.useState<string | null>(null);
  const room = homeChannel(item);
  const holder = item.state === "offered" ? item.offeredTo : item.dri;
  const dateLabel = item.type === "project" ? "Review" : "Due";
  const canEditDue = canChangeTicketDue(item, viewer);

  const run = React.useCallback(
    async (name: string, action: () => Promise<unknown>) => {
      setError(null);
      setBusy(name);
      try {
        await action();
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
      } finally {
        setBusy(null);
      }
    },
    [],
  );

  return (
    <article className="mx-auto flex w-full max-w-3xl flex-col gap-10 px-8 py-10">
      <nav aria-label="Breadcrumb" data-testid="org-item-breadcrumb">
        <ol className="flex flex-wrap items-center gap-1 text-2xs text-muted-foreground">
          <li>
            <Link
              className="rounded-sm hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
              to="/org/work"
            >
              Work
            </Link>
          </li>
          {parent ? (
            <li className="flex items-center gap-1">
              <span aria-hidden="true">/</span>
              <Link
                className="rounded-sm hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                data-testid="org-item-breadcrumb-parent"
                params={{ itemId: parent.id }}
                to="/org/work/$itemId"
              >
                {parent.title}
              </Link>
            </li>
          ) : null}
          <li className="flex items-center gap-1">
            <span aria-hidden="true">/</span>
            <span aria-current="page" className="text-foreground">
              {item.title}
            </span>
          </li>
        </ol>
      </nav>

      <header
        className="org-overview-settle space-y-3"
        style={motionDelay(overviewCardDelayMs(0))}
      >
        <p className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
          {item.type === "project" ? "Project" : "Ticket"}
        </p>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <h1
            className="org-dir-line text-xl font-semibold tracking-tight"
            data-testid="org-item-title"
            style={motionDelay(overviewLineDelayMs(0, 0))}
          >
            {item.title}
          </h1>
          <StateChip item={item} who={undefined} />
        </div>
      </header>

      {item.brief ? (
        <section
          aria-labelledby="org-item-brief-heading"
          className="org-overview-settle space-y-3"
          style={motionDelay(overviewCardDelayMs(1))}
        >
          <h2
            className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
            id="org-item-brief-heading"
          >
            Brief
          </h2>
          <span
            aria-hidden="true"
            className="org-dir-rule block h-px w-10 bg-foreground/45"
            style={motionDelay(overviewRuleDelayMs(1))}
          />
          <p
            className="org-dir-line whitespace-pre-wrap text-sm leading-relaxed"
            data-testid="org-item-brief"
            style={motionDelay(overviewLineDelayMs(1, 0))}
          >
            {item.brief}
          </p>
        </section>
      ) : null}

      {item.planContent ? <PlanSteps content={item.planContent} /> : null}

      <dl className="grid gap-4 sm:grid-cols-2" data-testid="org-item-facts">
        <Fact
          enterIndex={2}
          label={item.state === "offered" ? "Offered to" : "Holds it"}
        >
          <HolderName pubkey={holder} />
        </Fact>
        <Fact
          action={
            canEditDue && !editingDue ? (
              <Button
                aria-label={`Modify ${dateLabel.toLowerCase()} date`}
                data-testid="org-modify-due"
                onClick={() => {
                  setDueDraft(
                    item.dueAt !== null ? unixToDateInput(item.dueAt) : "",
                  );
                  setEditingDue(true);
                }}
                size="xs"
                type="button"
                variant="ghost"
              >
                Modify
              </Button>
            ) : null
          }
          enterIndex={3}
          label={dateLabel}
        >
          {editingDue ? (
            <form
              className="flex flex-wrap items-center gap-2"
              onSubmit={(event) => {
                event.preventDefault();
                const dueAt = dateInputToUnix(dueDraft);
                if (dueAt === null) return;
                void run("due", async () => {
                  await commands.publish(
                    commands.buildIoSetDue(item.id, dueAt),
                  );
                  setEditingDue(false);
                });
              }}
            >
              <label className="sr-only" htmlFor="org-set-due-date">
                {dateLabel} date
              </label>
              <input
                className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
                data-testid="org-set-due-date"
                id="org-set-due-date"
                onChange={(event) => setDueDraft(event.target.value)}
                type="date"
                value={dueDraft}
              />
              <Button
                aria-label={`Save ${dateLabel.toLowerCase()} date`}
                data-testid="org-set-due"
                disabled={busy !== null || dueDraft.length === 0}
                size="xs"
                type="submit"
              >
                Save
              </Button>
              <Button
                disabled={busy !== null}
                onClick={() => setEditingDue(false)}
                size="xs"
                type="button"
                variant="ghost"
              >
                Cancel
              </Button>
            </form>
          ) : item.dueAt !== null ? (
            <time
              data-testid="org-item-due"
              data-ts={item.dueAt}
              dateTime={iso(item.dueAt)}
            >
              {formatWorkDate(item.dueAt)}
            </time>
          ) : (
            <span className="text-muted-foreground" data-testid="org-item-due">
              Not set
            </span>
          )}
        </Fact>
        {item.approvedAt !== null ? (
          <Fact enterIndex={4} label="Approved">
            <time
              data-testid="org-item-approved"
              data-ts={item.approvedAt}
              dateTime={iso(item.approvedAt)}
            >
              {formatWorkDate(item.approvedAt)}
            </time>
          </Fact>
        ) : null}
      </dl>

      <div
        className="org-dir-stamp flex flex-wrap items-center gap-3"
        data-testid="org-item-actions"
        style={motionDelay(overviewCardDelayMs(3))}
      >
        {room ? (
          <Button
            data-testid="org-open-room"
            onClick={() => {
              void goChannel(room);
            }}
            type="button"
            variant="outline"
          >
            Open channel
          </Button>
        ) : null}
        {canMarkDone(item, viewer) ? (
          <Button
            data-testid="org-mark-done"
            disabled={busy !== null}
            onClick={() => {
              void run("done", () =>
                commands.publish(commands.buildIoDone({ item: item.id })),
              );
            }}
            type="button"
          >
            Mark done
          </Button>
        ) : null}
      </div>
      {error ? (
        <p
          className="text-sm text-destructive"
          data-testid="org-item-error"
          role="alert"
        >
          {error}
        </p>
      ) : null}

      {item.type === "project" && health ? (
        <div className="org-overview-lift">
          <HealthCard
            className="org-overview-settle"
            health={health}
            style={motionDelay(overviewCardDelayMs(4))}
          />
        </div>
      ) : null}

      <section
        aria-labelledby="org-item-children-heading"
        className="org-overview-settle space-y-4"
        style={motionDelay(overviewCardDelayMs(5))}
      >
        <div className="flex items-center justify-between gap-3">
          <h2
            className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
            id="org-item-children-heading"
          >
            Under this {item.type === "project" ? "project" : "ticket"}
          </h2>
          <p
            className="text-2xs text-muted-foreground"
            data-testid="org-item-children-counts"
          >
            {formatChildrenCounts(item.children)}
          </p>
        </div>
        {childItems.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No tickets created yet
          </p>
        ) : (
          <ul
            className="divide-y divide-border rounded-xl border border-border"
            data-testid="org-item-children"
          >
            {childItems.map((child, index) => (
              <li key={child.id}>
                <Link
                  className="org-dir-line flex items-center justify-between gap-3 px-4 py-3.5 hover:bg-muted/60 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                  data-testid={`org-item-child-${child.id}`}
                  params={{ itemId: child.id }}
                  style={motionDelay(overviewLineDelayMs(5, index))}
                  to="/org/work/$itemId"
                >
                  <span className="min-w-0 truncate text-sm">
                    {child.title}
                  </span>
                  <StateChip item={child} who={undefined} />
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>
    </article>
  );
}

function Fact({
  label,
  action,
  children,
  enterIndex,
}: {
  label: string;
  action?: React.ReactNode;
  children: React.ReactNode;
  enterIndex: number;
}) {
  return (
    <OrgMotionFrame
      className="rounded-xl border border-border px-4 py-4"
      enterIndex={enterIndex}
    >
      <div className="flex items-center justify-between gap-3">
        <dt className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
          {label}
        </dt>
        {action}
      </div>
      <dd className="mt-3 text-sm font-medium">{children}</dd>
    </OrgMotionFrame>
  );
}

function iso(unix: number): string {
  return new Date(unix * 1000).toISOString();
}

function unixToDateInput(unix: number): string {
  return new Date(unix * 1000).toISOString().slice(0, 10);
}

function dateInputToUnix(value: string): number | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const parsed = Date.parse(`${value}T00:00:00Z`);
  if (Number.isNaN(parsed)) return null;
  return Math.floor(parsed / 1000);
}
