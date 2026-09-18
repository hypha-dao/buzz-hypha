import type { Channel, RelayEvent } from "@/shared/api/types";
import { KIND_IO_SHAPERS } from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

/** The `d` tag of the one `kind:39103` Shapers-and-rules event per community. */
export const SHAPERS_D_TAG = "shapers";

/**
 * Who the community's org agent is, as the relay-signed `kind:39103` says
 * (Protocol §4.5). `pubkey` is null until the community is bootstrapped or
 * while `39103.agent` is null (a relay with no hosted-agent row — see the
 * `io_hosted_agents` follow-up in the progress log). `hosted` mirrors
 * `agent_hosted`: true while the relay operator runs the agent.
 */
export type OrgAgentIdentity = {
  pubkey: string | null;
  hosted: boolean;
};

export const NO_ORG_AGENT: OrgAgentIdentity = { pubkey: null, hosted: false };

const HEX_PUBKEY = /^[0-9a-f]{64}$/;

function asPubkey(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const normalized = normalizePubkey(value);
  return HEX_PUBKEY.test(normalized) ? normalized : null;
}

function isShapersState(event: RelayEvent): boolean {
  return (
    event.kind === KIND_IO_SHAPERS &&
    event.tags.some((tag) => tag[0] === "d" && tag[1] === SHAPERS_D_TAG)
  );
}

/**
 * Read the org agent out of the newest `kind:39103` `d=shapers` event among
 * `events`. Anything that is not a well-formed `39103` (other kinds, other `d`
 * tags, unparsable content, a malformed `agent`) is ignored rather than
 * treated as "no agent", so a stray event can never demote a known agent.
 */
export function parseOrgAgentFromShapers(
  events: readonly RelayEvent[],
): OrgAgentIdentity {
  const newest = events
    .filter(isShapersState)
    .sort((left, right) => right.created_at - left.created_at)[0];
  if (!newest) return NO_ORG_AGENT;

  let content: unknown;
  try {
    content = JSON.parse(newest.content);
  } catch {
    return NO_ORG_AGENT;
  }
  if (typeof content !== "object" || content === null) return NO_ORG_AGENT;
  const record = content as Record<string, unknown>;
  const pubkey = asPubkey(record.agent);
  return {
    pubkey,
    hosted: pubkey !== null && record.agent_hosted === true,
  };
}

/**
 * True for the member's own DM with the org agent: the DM whose participant
 * set minus `39103.agent` is exactly the member (Protocol §6.8 "The agent
 * DM"). Two shapes qualify — `{member, agent}`, as a relay that still lists
 * the agent among the `p` tags writes it, and `{member}` alone, the DM
 * identity §6.8 specifies once the relay excludes the agent from it (a
 * self-only DM exists for no other reason). Group DMs the agent also sits in
 * and every other DM are not it. With no `currentPubkey` yet (identity still
 * loading) only the first shape can be recognised: a 1:1 whose other party
 * is the agent, which is the same channel once identity resolves.
 */
export function isOrgAgentDm(
  channel: Pick<Channel, "channelType" | "participantPubkeys">,
  orgAgentPubkey: string | null,
  currentPubkey: string | null,
): boolean {
  if (channel.channelType !== "dm" || !orgAgentPubkey) return false;
  // A DM with no participant list is unknown, not the agent's.
  if (channel.participantPubkeys.length === 0) return false;
  const agent = normalizePubkey(orgAgentPubkey);
  const others = new Set(channel.participantPubkeys.map(normalizePubkey));
  const listsAgent = others.delete(agent);
  if (currentPubkey) {
    others.delete(normalizePubkey(currentPubkey));
    return others.size === 0;
  }
  return listsAgent && others.size <= 1;
}

/**
 * Stable partition that moves the org agent's DM to the front of an already
 * sorted DM list (AGENTS.md § Hypha fork: "DMs list the org agent first").
 * Returns the input array itself when nothing needs to move, so memoised
 * consumers keep their reference.
 */
export function pinOrgAgentDmFirst<
  T extends Pick<Channel, "channelType" | "participantPubkeys">,
>(
  channels: readonly T[],
  orgAgentPubkey: string | null,
  currentPubkey: string | null,
): T[] {
  if (!orgAgentPubkey) return channels as T[];
  const index = channels.findIndex((channel) =>
    isOrgAgentDm(channel, orgAgentPubkey, currentPubkey),
  );
  if (index <= 0) return channels as T[];
  const pinned = channels[index] as T;
  return [pinned, ...channels.slice(0, index), ...channels.slice(index + 1)];
}

/**
 * Drop the org agent from a list of a member's agents. The Agents door never
 * lists it (Design § Where it runs — Members' own agents stay), even when a
 * Shapers decision has pointed `39103.agent` at a key this member happens to
 * run locally. Returns the input array itself when nothing is removed.
 */
export function excludeOrgAgent<T extends { pubkey: string }>(
  agents: readonly T[],
  orgAgentPubkey: string | null,
): T[] {
  if (!orgAgentPubkey) return agents as T[];
  const agent = normalizePubkey(orgAgentPubkey);
  if (!agents.some((entry) => normalizePubkey(entry.pubkey) === agent)) {
    return agents as T[];
  }
  return agents.filter((entry) => normalizePubkey(entry.pubkey) !== agent);
}
