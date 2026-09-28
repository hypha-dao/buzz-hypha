/**
 * Local Personal Assistant — a member-owned buzz-acp agent that guides
 * intelligent-org onboarding in a DM.
 *
 * Design § Personal Assistant names the eventual surface as the org agent's
 * DM. Phase 0 has no HEAR yet, so this fork ships a temporary PA on the
 * existing buzz-acp harness: it is the founder's own agent (not `39103.agent`,
 * never a sample persona, and not listed as the org agent). When the org
 * agent can listen, this guide folds into that DM (progress follow-up).
 */

import { getDefaultPersonaRuntime } from "@/features/agents/lib/resolvePersonaRuntime";
import { createManagedAgent, listManagedAgents } from "@/shared/api/tauri";
import { startManagedAgent } from "@/shared/api/tauriManagedAgents";
import { discoverAcpRuntimes } from "@/shared/api/tauriAcpDiscovery";
import { getGlobalAgentConfig } from "@/shared/api/tauriGlobalAgentConfig";
import type {
  AcpRuntime,
  Channel,
  CreateManagedAgentInput,
  ManagedAgent,
} from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export const PERSONAL_ASSISTANT_NAME = "Personal Assistant";
/** Stable team marker so we can recognise the PA without a persona id. */
export const PERSONAL_ASSISTANT_TEAM_ID = "hypha:personal-assistant";

export const PERSONAL_ASSISTANT_SYSTEM_PROMPT = `You are the Personal Assistant for a new Buzz community that is becoming an intelligent organization.

Your job in this chat:
1. Congratulate the founder on starting the community.
2. Invite them to make the organization intelligent — AI drafts, people decide; work is offered, never assigned.
3. Ask whether they are the sole Shaper or will invite others. Shapers are like a board: they set direction (mission, vision, objectives, strategy) and approve big projects.
4. If alone: help them define direction in this chat (mission, then vision, then objectives, then strategy). Keep drafts short. They confirm in Overview or by agreeing here.
5. If others: help them mint an invite, then make invitees Shapers. The private #shapers room is created by the relay when they bootstrap (kind 50001) — never invent a fake channel.

You are NOT the org agent. The org agent is a separate, unlisted community member the relay hosts. Do not claim to be it. Do not appear in the Agents door as a sample persona — you are this member's own assistant.

Be warm, concrete, and brief. Prefer questions over lectures. When they are ready to bootstrap as the first Shaper, tell them to tap the choice in the guide panel above the composer (or say "bootstrap me as the only Shaper").`;

function normalizeRelayUrl(relayUrl: string | null | undefined) {
  return relayUrl?.trim().replace(/\/+$/, "") ?? null;
}

function isAgentScopedToRelay(agent: ManagedAgent, relayUrl?: string | null) {
  const target = normalizeRelayUrl(relayUrl);
  if (!target) return true;
  return normalizeRelayUrl(agent.relayUrl) === target;
}

export function isPersonalAssistantAgent(
  agent: Pick<ManagedAgent, "name" | "teamId">,
): boolean {
  if (agent.teamId === PERSONAL_ASSISTANT_TEAM_ID) return true;
  return (
    agent.name.trim().toLowerCase() === PERSONAL_ASSISTANT_NAME.toLowerCase()
  );
}

export function pickPersonalAssistantForRelay(
  agents: readonly ManagedAgent[],
  relayUrl?: string | null,
): ManagedAgent | null {
  const matches = agents.filter(
    (agent) =>
      isPersonalAssistantAgent(agent) && isAgentScopedToRelay(agent, relayUrl),
  );
  return (
    matches.find((agent) => agent.status === "running") ??
    matches.find((agent) => agent.status === "deployed") ??
    matches[0] ??
    null
  );
}

/**
 * True for the member's 1:1 DM with their Personal Assistant managed agent.
 */
export function isPersonalAssistantDm(
  channel: Pick<Channel, "channelType" | "participantPubkeys">,
  paPubkey: string | null,
  currentPubkey: string | null,
): boolean {
  if (channel.channelType !== "dm" || !paPubkey || !currentPubkey) return false;
  if (channel.participantPubkeys.length === 0) return false;
  const pa = normalizePubkey(paPubkey);
  const me = normalizePubkey(currentPubkey);
  const others = new Set(channel.participantPubkeys.map(normalizePubkey));
  others.delete(me);
  return others.size === 1 && others.has(pa);
}

/** Pin the PA DM first in an already-sorted DM list. */
export function pinPersonalAssistantDmFirst<
  T extends Pick<Channel, "channelType" | "participantPubkeys">,
>(
  channels: readonly T[],
  paPubkey: string | null,
  currentPubkey: string | null,
): T[] {
  if (!paPubkey) return channels as T[];
  const index = channels.findIndex((channel) =>
    isPersonalAssistantDm(channel, paPubkey, currentPubkey),
  );
  if (index <= 0) return channels as T[];
  const pinned = channels[index] as T;
  return [pinned, ...channels.slice(0, index), ...channels.slice(index + 1)];
}

async function buildPersonalAssistantCreateInput(
  runtime: AcpRuntime,
  relayUrl?: string | null,
): Promise<CreateManagedAgentInput> {
  return {
    name: PERSONAL_ASSISTANT_NAME,
    teamId: PERSONAL_ASSISTANT_TEAM_ID,
    relayUrl: relayUrl ?? undefined,
    acpCommand: "buzz-acp",
    agentCommand: runtime.command,
    agentArgs: [],
    mcpCommand: runtime.mcpCommand ?? "",
    systemPrompt: PERSONAL_ASSISTANT_SYSTEM_PROMPT,
    spawnAfterCreate: false,
    startOnAppLaunch: true,
    respondTo: "owner-only",
  };
}

/**
 * Ensure a Personal Assistant managed agent exists for this community/relay,
 * start it when possible, and return it. Throws when no ACP runtime is
 * available — callers should surface that as a configure-AI hint.
 */
export async function ensurePersonalAssistant(
  relayUrl?: string | null,
): Promise<ManagedAgent> {
  const existing = pickPersonalAssistantForRelay(
    await listManagedAgents(),
    relayUrl,
  );
  if (existing) {
    if (existing.status !== "running" && existing.status !== "deployed") {
      try {
        return await startManagedAgent(existing.pubkey, {
          expectedRelayUrl: relayUrl ?? undefined,
        });
      } catch (error) {
        console.warn(
          "Personal Assistant start failed; reusing idle agent.",
          error,
        );
        return existing;
      }
    }
    return existing;
  }

  const [catalog, globalConfig] = await Promise.all([
    discoverAcpRuntimes(),
    getGlobalAgentConfig(),
  ]);
  const runtimes = catalog.filter(
    (runtime): runtime is AcpRuntime => runtime.availability === "available",
  );
  const runtime = getDefaultPersonaRuntime(
    runtimes,
    globalConfig.preferred_runtime,
  );
  if (!runtime) {
    throw new Error(
      "No agent runtime is available. Finish AI setup in onboarding or Settings → Agents, then retry.",
    );
  }

  const created = await createManagedAgent(
    await buildPersonalAssistantCreateInput(runtime, relayUrl),
  );
  try {
    return await startManagedAgent(created.agent.pubkey, {
      expectedRelayUrl: relayUrl ?? undefined,
    });
  } catch (error) {
    console.warn(
      "Personal Assistant created but start failed; DM still opens.",
      error,
    );
    return created.agent;
  }
}
