import { normalizePubkey } from "@/shared/lib/pubkey";

export type OrgSetup =
  | { kind: "alone" }
  | { kind: "invite"; maxUses: number; quorum: number };

const STORAGE_PREFIX = "buzz-org-setup.v1";

export function orgSetupStorageKey(messageId: string) {
  return `${STORAGE_PREFIX}:${messageId}`;
}

export function readSetupResult(
  messageId: string,
): { url: string | null } | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(orgSetupStorageKey(messageId));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { url?: unknown };
    if (parsed.url === null) return { url: null };
    if (typeof parsed.url === "string" && parsed.url.length > 0) {
      return { url: parsed.url };
    }
    return null;
  } catch {
    return null;
  }
}

export function writeSetupResult(messageId: string, url: string | null) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(
      orgSetupStorageKey(messageId),
      JSON.stringify({ url }),
    );
  } catch {
    // The chat still shows the link for this view.
  }
}

function positiveInt(value: string | undefined): number | null {
  if (!value || !/^[1-9][0-9]*$/.test(value)) return null;
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 1) return null;
  return parsed;
}

export function parseOrgSetupMessage(
  message: { pubkey?: string; signerPubkey?: string; tags?: string[][] },
  orgAgentPubkey: string | null,
): OrgSetup | null {
  if (!orgAgentPubkey) return null;
  const agent = normalizePubkey(orgAgentPubkey);
  const author = [message.pubkey, message.signerPubkey].find(
    (pubkey) => pubkey !== undefined && normalizePubkey(pubkey) === agent,
  );
  if (!author) return null;
  const tag = message.tags?.find((row) => row[0] === "io-setup");
  if (!tag) return null;
  if (tag[1] === "alone") return { kind: "alone" };
  if (tag[1] !== "invite") return null;
  const maxUses = positiveInt(tag[2]);
  const quorum = positiveInt(tag[3]);
  if (maxUses === null || quorum === null || quorum > maxUses + 1) {
    return null;
  }
  return { kind: "invite", maxUses, quorum };
}

export async function runOrgSetup({
  bootstrapped,
  mint,
  publishBootstrap,
  publishRules,
  setup,
}: {
  bootstrapped: boolean;
  mint: (maxUses: number) => Promise<{ url: string }>;
  publishBootstrap: () => Promise<unknown>;
  publishRules: (need: number, of: number) => Promise<unknown>;
  setup: OrgSetup;
}): Promise<{ url: string | null }> {
  if (!bootstrapped) {
    await publishBootstrap();
  }
  if (setup.kind === "alone") {
    return { url: null };
  }
  await publishRules(setup.quorum, setup.maxUses + 1);
  const invite = await mint(setup.maxUses);
  return { url: invite.url };
}
