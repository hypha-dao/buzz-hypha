import type { UserProfileSummary } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export function personLabel(
  pubkey: string,
  profiles: Record<string, UserProfileSummary>,
): string {
  const key = normalizePubkey(pubkey);
  const profile = profiles[key];
  return profile?.displayName || profile?.name || pubkey.slice(0, 8);
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
