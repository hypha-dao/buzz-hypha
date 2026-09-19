import * as React from "react";
import { Link } from "@tanstack/react-router";

import { useAppNavigation } from "@/app/navigation/useAppNavigation";
import { useIdentityQuery } from "@/shared/api/hooks";
import { Button } from "@/shared/ui/button";

import { useOrgCommands } from "../../useOrgCommands";
import {
  canMarkDone,
  canRelease,
  formatChildrenCounts,
  formatWorkDate,
  homeChannel,
  type WorkHealth,
  type WorkItem,
  type WorkTrailEntry,
} from "../../work/model";
import { HealthCard } from "./HealthCard";
import { HolderName } from "./HolderName";
import { StateChip } from "./StateChip";

type WorkItemViewProps = {
  item: WorkItem;
  parent: WorkItem | null;
  childItems: WorkItem[];
  trail: WorkTrailEntry[];
  health: WorkHealth | null;
};

export function WorkItemView({
  item,
  parent,
  childItems,
  trail,
  health,
}: WorkItemViewProps) {
  const viewer = useIdentityQuery().data?.pubkey ?? null;
  const commands = useOrgCommands();
  const { goChannel } = useAppNavigation();
  const [error, setError] = React.useState<string | null>(null);
  const [dueDraft, setDueDraft] = React.useState("");
  const [busy, setBusy] = React.useState<string | null>(null);
  const room = homeChannel(item);
  const holder = item.state === "offered" ? item.offeredTo : item.dri;
  const dueLabel = item.state === "in_review" ? "Review" : "Due";

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
    <article className="mx-auto flex w-full max-w-3xl flex-col gap-6 p-6">
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

      <header className="space-y-2">
        <p className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
          {item.type === "project" ? "Project" : "Ticket"}
        </p>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <h1 className="text-xl font-semibold tracking-tight" data-testid="org-item-title">
            {item.title}
          </h1>
          <StateChip
            item={item}
            who={undefined}
          />
        </div>
      </header>

      {item.brief ? (
        <section aria-labelledby="org-item-brief-heading">
          <h2
            className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
            id="org-item-brief-heading"
          >
            Brief
          </h2>
          <p
            className="mt-2 whitespace-pre-wrap text-sm leading-relaxed"
            data-testid="org-item-brief"
          >
            {item.brief}
          </p>
        </section>
      ) : null}

      <dl className="grid gap-3 sm:grid-cols-2" data-testid="org-item-facts">
        <Fact label={item.state === "offered" ? "Offered to" : "Holds it"}>
          <HolderName pubkey={holder} />
        </Fact>
        {item.dueAt !== null ? (
          <Fact label={dueLabel}>
            <time data-testid="org-item-due" data-ts={item.dueAt} dateTime={iso(item.dueAt)}>
              {formatWorkDate(item.dueAt)}
            </time>
          </Fact>
        ) : null}
        {item.approvedAt !== null ? (
          <Fact label="Approved">
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

      <div className="flex flex-wrap items-center gap-2" data-testid="org-item-actions">
        {room ? (
          <Button
            data-testid="org-open-room"
            onClick={() => {
              void goChannel(room);
            }}
            type="button"
            variant="outline"
          >
            Open room
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
        {canRelease(item, viewer) ? (
          <Button
            data-testid="org-release"
            disabled={busy !== null}
            onClick={() => {
              void run("release", () =>
                commands.publish(commands.buildIoRelease(item.id)),
              );
            }}
            type="button"
            variant="outline"
          >
            Release
          </Button>
        ) : null}
        <div className="flex flex-wrap items-center gap-2">
          <label className="flex items-center gap-2 text-sm">
            <span className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
              Set due
            </span>
            <input
              className="h-9 rounded-lg border border-input bg-background px-2 text-sm"
              data-testid="org-set-due-date"
              onChange={(event) => setDueDraft(event.target.value)}
              type="date"
              value={dueDraft}
            />
          </label>
          <Button
            data-testid="org-set-due"
            disabled={busy !== null || dueDraft.length === 0}
            onClick={() => {
              const dueAt = dateInputToUnix(dueDraft);
              if (dueAt === null) return;
              void run("due", () =>
                commands.publish(commands.buildIoSetDue(item.id, dueAt)),
              );
            }}
            type="button"
            variant="outline"
          >
            Set due
          </Button>
        </div>
      </div>
      {error ? (
        <p className="text-sm text-destructive" data-testid="org-item-error" role="alert">
          {error}
        </p>
      ) : null}

      {item.type === "project" && health ? <HealthCard health={health} /> : null}

      <section aria-labelledby="org-item-children-heading">
        <div className="mb-2 flex items-center justify-between gap-3">
          <h2
            className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
            id="org-item-children-heading"
          >
            Under this {item.type === "project" ? "project" : "ticket"}
          </h2>
          <p className="text-2xs text-muted-foreground" data-testid="org-item-children-counts">
            {formatChildrenCounts(item.children)}
          </p>
        </div>
        {childItems.length === 0 ? (
          <p className="text-sm text-muted-foreground">Nothing here.</p>
        ) : (
          <ul className="divide-y divide-border rounded-xl border border-border" data-testid="org-item-children">
            {childItems.map((child) => (
              <li key={child.id}>
                <Link
                  className="flex items-center justify-between gap-3 px-3 py-2.5 hover:bg-muted/60 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
                  data-testid={`org-item-child-${child.id}`}
                  params={{ itemId: child.id }}
                  to="/org/work/$itemId"
                >
                  <span className="min-w-0 truncate text-sm">{child.title}</span>
                  <StateChip
                    item={child}
                    who={undefined}
                  />
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section aria-labelledby="org-item-trail-heading">
        <h2
          className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
          id="org-item-trail-heading"
        >
          Trail
        </h2>
        {trail.length === 0 ? (
          <p className="mt-2 text-sm text-muted-foreground">Nothing here.</p>
        ) : (
          <ol className="mt-2 space-y-2" data-testid="org-item-trail">
            {trail.map((entry) => (
              <li
                className="flex items-baseline justify-between gap-3 text-sm"
                data-kind={entry.kind}
                data-testid="org-trail-row"
                key={entry.id}
              >
                <span>{entry.label}</span>
                <time
                  className="text-2xs text-muted-foreground"
                  dateTime={iso(entry.createdAt)}
                >
                  {formatWorkDate(entry.createdAt)}
                </time>
              </li>
            ))}
          </ol>
        )}
      </section>
    </article>
  );
}

function Fact({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-xl border border-border px-3 py-2">
      <dt className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </dt>
      <dd className="mt-1 text-sm font-medium">{children}</dd>
    </div>
  );
}

function iso(unix: number): string {
  return new Date(unix * 1000).toISOString();
}

function dateInputToUnix(value: string): number | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const parsed = Date.parse(`${value}T00:00:00Z`);
  if (Number.isNaN(parsed)) return null;
  return Math.floor(parsed / 1000);
}
