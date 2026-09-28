import type { UserProfileSummary } from "@/shared/api/types";
import { normalizePubkey, truncatePubkey } from "@/shared/lib/pubkey";

export function personLabel(
  pubkey: string,
  profiles: Record<string, UserProfileSummary>,
): string {
  const key = normalizePubkey(pubkey);
  const profile = profiles[key];
  return profile?.displayName || profile?.name || truncatePubkey(pubkey);
}

export function OrgPersonName({
  pubkey,
  profiles,
}: {
  pubkey: string;
  profiles: Record<string, UserProfileSummary>;
}) {
  const label = personLabel(pubkey, profiles);
  return <span>{label}</span>;
}
