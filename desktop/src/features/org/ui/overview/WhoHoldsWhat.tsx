import { ProfileAvatar } from "@/features/profile/ui/ProfileAvatar";
import type { UserProfileSummary } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

import { personLabel } from "./OrgPersonName";
import { OverviewCardShell } from "./OverviewCardShell";
import type { ProjectHold } from "./parseOverview";

type WhoHoldsWhatProps = {
  enterIndex: number;
  holds: readonly ProjectHold[];
  profiles: Record<string, UserProfileSummary>;
  onOpenItem: (itemId: string) => void;
};

export function WhoHoldsWhat({
  enterIndex,
  holds,
  profiles,
  onOpenItem,
}: WhoHoldsWhatProps) {
  return (
    <OverviewCardShell enterIndex={enterIndex} testId="org-who-holds">
      <h2 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        Who holds a project
      </h2>
      {holds.length === 0 ? (
        <p className="mt-3 text-base text-muted-foreground">Not set yet.</p>
      ) : (
        <ul className="mt-4 space-y-1">
          {holds.map((hold) => (
            <li key={hold.id}>
              {hold.dri ? (
                <HolderRow
                  hold={hold}
                  name={personLabel(hold.dri, profiles)}
                  onOpenItem={onOpenItem}
                  profile={profiles[normalizePubkey(hold.dri)]}
                />
              ) : (
                <OpenRow hold={hold} onOpenItem={onOpenItem} />
              )}
            </li>
          ))}
        </ul>
      )}
    </OverviewCardShell>
  );
}

function HolderRow({
  hold,
  name,
  onOpenItem,
  profile,
}: {
  hold: ProjectHold;
  name: string;
  onOpenItem: (itemId: string) => void;
  profile: UserProfileSummary | undefined;
}) {
  return (
    <button
      className="flex w-full items-center gap-3 rounded-lg px-2 py-2.5 text-left hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      data-testid={`org-hold-${hold.id}`}
      onClick={() => onOpenItem(hold.id)}
      type="button"
    >
      <span aria-hidden="true">
        <ProfileAvatar
          avatarUrl={profile?.avatarUrl ?? null}
          className="h-9 w-9"
          label={name}
        />
      </span>
      <span className="min-w-0 truncate text-base">
        <span className="font-medium">{name}</span>
        <span className="text-muted-foreground"> · {hold.title} →</span>
      </span>
    </button>
  );
}

function OpenRow({
  hold,
  onOpenItem,
}: {
  hold: ProjectHold;
  onOpenItem: (itemId: string) => void;
}) {
  return (
    <button
      className="flex w-full items-center gap-3 rounded-lg px-2 py-2.5 text-left hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      data-testid={`org-hold-${hold.id}`}
      onClick={() => onOpenItem(hold.id)}
      type="button"
    >
      <span
        aria-hidden="true"
        className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-dashed border-muted-foreground text-base text-muted-foreground"
      >
        ?
      </span>
      <span className="min-w-0">
        <span className="block truncate text-base text-muted-foreground">
          {hold.title} →
        </span>
        <span className="block text-xs text-muted-foreground">needs a DRI</span>
      </span>
    </button>
  );
}
