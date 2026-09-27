import { useIdentityQuery } from "@/shared/api/hooks";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { useUserProfileQuery } from "@/features/profile/hooks";
import { normalizePubkey } from "@/shared/lib/pubkey";

type HolderNameProps = {
  pubkey: string | null;
  empty?: string;
  testId?: string;
};

export function HolderName({
  pubkey,
  empty = "nobody yet",
  testId = "org-holder-name",
}: HolderNameProps) {
  const identity = useIdentityQuery().data?.pubkey ?? null;
  const profile = useUserProfileQuery(pubkey ?? undefined).data;
  if (!pubkey) {
    return (
      <span data-empty="true" data-testid={testId}>
        {empty}
      </span>
    );
  }
  const label = resolveUserLabel({
    pubkey,
    currentPubkey: identity ?? undefined,
    profiles: profile
      ? {
          [normalizePubkey(pubkey)]: {
            displayName: profile.displayName,
            avatarUrl: profile.avatarUrl,
            nip05Handle: profile.nip05Handle,
            ownerPubkey: profile.ownerPubkey,
          },
        }
      : undefined,
  });
  return (
    <span data-pubkey={pubkey} data-testid={testId}>
      {label}
    </span>
  );
}
