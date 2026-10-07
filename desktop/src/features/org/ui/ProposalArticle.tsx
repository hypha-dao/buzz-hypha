import type * as React from "react";
import { Link } from "@tanstack/react-router";

import { OrgMotionFrame, motionDelay } from "./OrgMotionFrame";
import {
  overviewLineDelayMs,
  overviewRuleDelayMs,
} from "./overview/overviewMotion";

function parentLabel(
  parentTo: "/org/my-drafts" | "/org/my-work" | "/org/work",
): string {
  if (parentTo === "/org/my-drafts") return "My drafts";
  if (parentTo === "/org/my-work") return "My Work";
  return "Work";
}

type ProposalArticleProps = {
  /** Prefix for every `data-testid` on the page. */
  testId: string;
  parentTo: "/org/my-drafts" | "/org/my-work" | "/org/work";
  crumb: string;
  eyebrow: string;
  title: string;
  status: string;
  briefHeading: string;
  brief: string;
  lines: readonly {
    id: string;
    text: string;
    doneWhen?: string;
    date?: number;
    lineType?: "bet" | "rule" | "refusal";
  }[];
  /** `ProposalFact` cells; omitted when the page has none. */
  facts?: React.ReactNode;
  children?: React.ReactNode;
};

/** The proposal page body: one layout for a proposal and for its draft. */
export function ProposalArticle({
  testId,
  parentTo,
  crumb,
  eyebrow,
  title,
  status,
  briefHeading,
  brief,
  lines,
  facts,
  children,
}: ProposalArticleProps) {
  const titleId = `${testId}-title`;
  return (
    <article className="mx-auto flex w-full max-w-3xl flex-col gap-10 px-8 py-10">
      <nav aria-label="Breadcrumb" data-testid={`${testId}-breadcrumb`}>
        <ol className="flex flex-wrap items-center gap-1 text-2xs text-muted-foreground">
          <li>
            <Link
              className="rounded-sm hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
              to={parentTo}
            >
              {parentLabel(parentTo)}
            </Link>
          </li>
          <li className="flex items-center gap-1">
            <span aria-hidden="true">/</span>
            <span aria-current="page" className="text-foreground">
              {crumb}
            </span>
          </li>
        </ol>
      </nav>

      <header className="org-overview-settle space-y-3">
        <p
          className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
          data-testid={`${testId}-kind`}
        >
          {eyebrow}
        </p>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <h1
            className="org-dir-line text-xl font-semibold tracking-tight"
            data-testid={titleId}
            id={titleId}
            style={motionDelay(overviewLineDelayMs(0, 0))}
          >
            {title}
          </h1>
          <span
            className="rounded-full border border-border px-2 py-0.5 text-2xs text-muted-foreground"
            data-testid={`${testId}-status`}
          >
            {status}
          </span>
        </div>
      </header>

      {brief ? (
        <section className="org-overview-settle space-y-3">
          <h2 className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
            {briefHeading}
          </h2>
          <span
            aria-hidden="true"
            className="org-dir-rule block h-px w-10 bg-foreground/45"
            style={motionDelay(overviewRuleDelayMs(1))}
          />
          <p
            className="org-dir-line whitespace-pre-wrap text-sm leading-relaxed"
            data-testid={`${testId}-brief`}
            style={motionDelay(overviewLineDelayMs(1, 0))}
          >
            {brief}
          </p>
        </section>
      ) : null}

      {lines.length > 0 ? (
        <section className="org-overview-settle">
          <ol
            aria-labelledby={titleId}
            className="list-decimal space-y-3 pl-5"
            data-testid={`${testId}-lines`}
          >
            {lines.map((line) => {
              const note = lineNote(line);
              return (
                <li className="text-sm leading-relaxed" key={line.id}>
                  {line.text}
                  {note ? (
                    <p className="mt-1 text-sm text-muted-foreground">{note}</p>
                  ) : null}
                  {line.lineType ? (
                    <p className="mt-1 text-sm capitalize text-muted-foreground">
                      {line.lineType}
                    </p>
                  ) : null}
                </li>
              );
            })}
          </ol>
        </section>
      ) : null}

      {facts ? <dl className="grid gap-4 sm:grid-cols-2">{facts}</dl> : null}

      {children}
    </article>
  );
}

function lineNote(line: { doneWhen?: string; date?: number }): string | null {
  const parts: string[] = [];
  if (line.doneWhen) parts.push(`Done when: ${line.doneWhen}`);
  if (line.date !== undefined) {
    parts.push(`By: ${new Date(line.date * 1000).toISOString().slice(0, 10)}`);
  }
  return parts.length > 0 ? parts.join(". ") : null;
}

export function ProposalFact({
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
