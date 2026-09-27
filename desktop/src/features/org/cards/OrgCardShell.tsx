import type { ReactNode } from "react";

import { cn } from "@/shared/lib/cn";

import type { OrgCardModel, OrgFact, OrgReceipt } from "./types";

type OrgCardShellProps = {
  model: OrgCardModel;
  actions: ReactNode;
  extra?: ReactNode;
};

export function OrgCardShell({ model, actions, extra }: OrgCardShellProps) {
  const headingId = `org-card-claim-${model.event.id}`;
  return (
    <article
      aria-labelledby={headingId}
      className="rounded-xl border border-border/60 bg-card p-4 shadow-xs"
      data-card-type={model.cardType}
      data-kicker={model.kickerKind}
      data-testid={`org-card-${model.event.id}`}
    >
      <p
        className="text-2xs font-medium uppercase tracking-wider text-muted-foreground"
        data-testid="org-card-kicker"
      >
        {model.kicker}
      </p>
      <h3
        className="mt-2 text-message font-semibold leading-snug"
        id={headingId}
      >
        {model.claim}
      </h3>
      {model.facts.length > 0 ? <FactRow facts={model.facts} /> : null}
      {model.needed ? (
        <p
          className="mt-2 text-2xs text-muted-foreground"
          data-testid="org-card-needed"
        >
          {model.needed.agrees} of {model.needed.needed}
        </p>
      ) : null}
      {model.receipts.length > 0 ? (
        <ReceiptRow receipts={model.receipts} />
      ) : null}
      {extra}
      <div className="mt-3 flex flex-wrap gap-2">{actions}</div>
    </article>
  );
}

function FactRow({ facts }: { facts: OrgFact[] }) {
  return (
    <dl className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
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
