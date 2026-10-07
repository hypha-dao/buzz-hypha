import { OverviewCardShell } from "./OverviewCardShell";
import { NOT_SET_YET } from "./overviewCopy";
import type { CodebaseLink } from "./parseOverview";

type CodebasesCardProps = {
  enterIndex: number;
  items: readonly CodebaseLink[] | null;
};

/** Repositories and the landing page, kept off the strategy card. */
export function CodebasesCard({ enterIndex, items }: CodebasesCardProps) {
  return (
    <section
      aria-labelledby="org-codebases-heading"
      className="flex flex-col gap-4 md:col-span-2"
    >
      <h2
        className="text-xs font-medium uppercase tracking-wider text-muted-foreground"
        id="org-codebases-heading"
      >
        Codebases
      </h2>
      <OverviewCardShell enterIndex={enterIndex} testId="org-codebases-card">
        {items === null || items.length === 0 ? (
          <p className="text-base text-muted-foreground">{NOT_SET_YET}</p>
        ) : (
          <ul className="space-y-4" data-testid="org-codebases-list">
            {items.map((item) => (
              <CodebaseRow item={item} key={item.id} />
            ))}
          </ul>
        )}
      </OverviewCardShell>
    </section>
  );
}

function CodebaseRow({ item }: { item: CodebaseLink }) {
  const label = item.kind === "site" ? "Landing page" : "Repository";
  const href = httpUrl(item.url);
  const name = item.name.trim() || label;
  return (
    <li data-testid={`org-codebase-${item.id}`}>
      <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </p>
      {href ? (
        <a
          className="mt-1 block text-base font-medium underline underline-offset-4"
          href={href}
          rel="noreferrer"
          target="_blank"
        >
          {name}
        </a>
      ) : (
        <p className="mt-1 text-base">{item.about.trim() || NOT_SET_YET}</p>
      )}
      {href && item.about.trim() ? (
        <p className="mt-1 text-sm text-muted-foreground">{item.about}</p>
      ) : null}
    </li>
  );
}

function httpUrl(url: string): string | null {
  const trimmed = url.trim();
  if (trimmed.startsWith("https://") || trimmed.startsWith("http://")) {
    return trimmed;
  }
  return null;
}
