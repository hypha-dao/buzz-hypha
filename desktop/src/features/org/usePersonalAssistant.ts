import { useQuery } from "@tanstack/react-query";

import { useCommunities } from "@/features/communities/useCommunities";
import { pickPersonalAssistantForRelay } from "@/features/org/personalAssistant";
import { listManagedAgents } from "@/shared/api/tauri";

export const personalAssistantQueryKey = ["org", "personal-assistant"] as const;

export function usePersonalAssistantQuery() {
  const { activeCommunity } = useCommunities();
  const relayUrl = activeCommunity?.relayUrl ?? null;
  return useQuery({
    queryKey: [...personalAssistantQueryKey, relayUrl],
    queryFn: async () => {
      const agents = await listManagedAgents();
      return pickPersonalAssistantForRelay(agents, relayUrl);
    },
    staleTime: 30_000,
  });
}

export function usePersonalAssistantPubkey(): string | null {
  return usePersonalAssistantQuery().data?.pubkey ?? null;
}
