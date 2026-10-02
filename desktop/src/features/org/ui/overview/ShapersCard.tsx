import { ProfileAvatar } from "@/features/profile/ui/ProfileAvatar";
import type { UserProfileSummary } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

import { personLabel } from "./OrgPersonName";
import { OverviewCardShell } from "./OverviewCardShell";
import type { ShapersState } from "./parseOverview";

type ShapersCardProps = {
  enterIndex: number;
  onOpenProfile: (pubkey: string) => void;
  profiles: Record<string, UserProfileSummary>;
  shapers: ShapersState | null;
};

export function ShapersCard({
  enterIndex,
  onOpenProfile,
  profiles,
  shapers,
}: ShapersCardProps) {
  const members = shapers?.shapers ?? [];

  return (
    <OverviewCardShell enterIndex={enterIndex} testId="org-shapers-card">
      <h2 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        Shapers
      </h2>
      {members.length === 0 ? (
        <p className="mt-3 text-base text-muted-foreground">Not set yet.</p>
      ) : (
        <ul className="mt-4 space-y-1" data-testid="org-shapers-members">
          {members.map((member) => {
            const key = normalizePubkey(member);
            const profile = profiles[key];
            const name = personLabel(member, profiles);
            return (
              <li key={key}>
                <button
                  className="flex w-full items-center gap-3 rounded-lg px-2 py-2.5 text-left hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  data-testid={`org-shaper-${key}`}
                  onClick={() => onOpenProfile(member)}
                  type="button"
                >
                  <span aria-hidden="true">
                    <ProfileAvatar
                      avatarUrl={profile?.avatarUrl ?? null}
                      className="h-9 w-9"
                      label={name}
                    />
                  </span>
                  <span className="min-w-0 truncate text-base">{name}</span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
      <p
        className="mt-4 text-base leading-relaxed text-muted-foreground"
        data-testid="org-shapers-majority"
      >
        A majority of Shapers is needed to decide anything.
      </p>
    </OverviewCardShell>
  );
}
