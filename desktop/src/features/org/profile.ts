/**
 * Org profile (`39105` / `50021`) — Protocol §4.7a / §4.8.
 *
 * Clients render from the relay-signed `39105`. Save sends the whole
 * profile as `50021` (`about`, skill labels, optional `open_limit`).
 * The relay kebab-cases labels; this file does not invent slugs.
 */

import type { RelayEvent } from "@/shared/api/types";
import { KIND_IO_PROFILE } from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

import type { UnsignedOrgCommand } from "./commands";
import { buildIoProfileSet } from "./commands";

/** Protocol §4.7a: `about` ≤ 1 000 chars. */
export const PROFILE_ABOUT_MAX_CHARS = 1_000;
/** Protocol §4.7a: at most 20 skills. */
export const PROFILE_MAX_SKILLS = 20;
/** Protocol §4.7a: each label ≤ 40 chars. */
export const PROFILE_SKILL_LABEL_MAX_CHARS = 40;
/** Protocol §4.7a: `open_limit` 1–50 or absent. */
export const PROFILE_OPEN_LIMIT_MIN = 1;
/** Protocol §4.7a: `open_limit` 1–50 or absent. */
export const PROFILE_OPEN_LIMIT_MAX = 50;
/** At most this many social links. */
export const PROFILE_MAX_SOCIALS = 8;
/** Each social URL is at most this many characters. */
export const PROFILE_SOCIAL_URL_MAX_CHARS = 200;

export const SOCIAL_NETWORKS = [
  "website",
  "github",
  "x",
  "linkedin",
  "nostr",
  "mastodon",
  "telegram",
  "discord",
  "youtube",
  "instagram",
  "bluesky",
] as const;

export type SocialNetwork = (typeof SOCIAL_NETWORKS)[number];

export type OrgSocial = {
  network: SocialNetwork;
  url: string;
};

export type OrgSkill = {
  label: string;
  slug: string;
};

export type OrgProfile = {
  about: string;
  openLimit: number | null;
  skills: OrgSkill[];
  socials: OrgSocial[];
};

export const EMPTY_ORG_PROFILE: OrgProfile = {
  about: "",
  openLimit: null,
  skills: [],
  socials: [],
};

export type ProfileSetFields = {
  about: string;
  openLimit?: number;
  skills: string[];
  socials: OrgSocial[];
};

export type ProfileSetValidation =
  | { ok: true; value: ProfileSetFields }
  | { ok: false; error: string };

function asRecord(value: unknown): Record<string, unknown> | null {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  return value as Record<string, unknown>;
}

function parseSkill(value: unknown): OrgSkill | null {
  if (typeof value === "string") {
    const label = value.trim();
    if (label.length === 0) return null;
    return { label, slug: label };
  }
  const record = asRecord(value);
  if (!record) return null;
  const label =
    typeof record.label === "string"
      ? record.label.trim()
      : typeof record.slug === "string"
        ? record.slug.trim()
        : "";
  if (label.length === 0) return null;
  const slug = typeof record.slug === "string" ? record.slug.trim() : label;
  return { label, slug };
}

const SOCIAL_NETWORK_SET = new Set<string>(SOCIAL_NETWORKS);

function hostOf(url: string): string | null {
  const rest = url.slice("https://".length);
  const end = rest.search(/[/?#]/);
  const host = (end === -1 ? rest : rest.slice(0, end)).toLowerCase();
  if (!host.includes(".") || host.includes("..") || host.includes("@"))
    return null;
  if (!/^[a-z0-9.-]+$/.test(host)) return null;
  return host;
}

function inferNetwork(host: string): SocialNetwork {
  const bare = host.replace(/^www\./, "");
  if (bare === "github.com" || bare.endsWith(".github.io")) return "github";
  if (bare === "x.com" || bare === "twitter.com") return "x";
  if (bare === "linkedin.com" || bare === "lnkd.in") return "linkedin";
  if (
    bare === "youtube.com" ||
    bare === "youtu.be" ||
    bare === "youtube-nocookie.com"
  ) {
    return "youtube";
  }
  if (bare === "instagram.com") return "instagram";
  if (bare === "t.me" || bare === "telegram.me" || bare === "telegram.org") {
    return "telegram";
  }
  if (bare === "discord.com" || bare === "discord.gg") return "discord";
  if (bare === "bsky.app") return "bluesky";
  if (
    bare === "njump.me" ||
    bare === "primal.net" ||
    bare === "nostr.com" ||
    bare === "snort.social"
  ) {
    return "nostr";
  }
  if (bare.includes("mastodon")) return "mastodon";
  return "website";
}

function namedNetwork(raw: string, host: string): SocialNetwork | null {
  const named = raw.trim().toLowerCase();
  if (
    named === "" ||
    named === "link" ||
    named === "web" ||
    named === "site" ||
    named === "homepage" ||
    named === "url"
  ) {
    return inferNetwork(host);
  }
  if (named === "twitter") return "x";
  if (!SOCIAL_NETWORK_SET.has(named)) return null;
  return named as SocialNetwork;
}

function hostMatches(network: SocialNetwork, host: string): boolean {
  if (network === "website" || network === "mastodon") return true;
  return inferNetwork(host) === network;
}

/** https links only, one per named network, at most three websites. */
export function normalizeSocials(input: readonly OrgSocial[]): OrgSocial[] {
  const out: OrgSocial[] = [];
  const seen = new Set<string>();
  let websites = 0;
  for (const raw of input) {
    if (out.length >= PROFILE_MAX_SOCIALS) break;
    const trimmed = raw.url.trim();
    if (
      trimmed.length === 0 ||
      trimmed.length > PROFILE_SOCIAL_URL_MAX_CHARS ||
      !trimmed.startsWith("https://") ||
      /\s/.test(trimmed)
    ) {
      continue;
    }
    const host = hostOf(trimmed);
    if (!host) continue;
    const network = namedNetwork(raw.network, host);
    if (!network || !hostMatches(network, host)) continue;
    if (network === "website") {
      websites += 1;
      if (websites > 3) continue;
    } else if (seen.has(network)) {
      continue;
    } else {
      seen.add(network);
    }
    const path = trimmed.slice("https://".length + host.length);
    out.push({ network, url: `https://${host}${path}` });
  }
  return out;
}

function parseSocialList(value: unknown): OrgSocial[] {
  if (!Array.isArray(value)) return [];
  const raw: OrgSocial[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) continue;
    const record = entry as Record<string, unknown>;
    const network = typeof record.network === "string" ? record.network : "";
    const url = typeof record.url === "string" ? record.url : "";
    if (!url) continue;
    const host = hostOf(url.trim().toLowerCase()) ?? "";
    raw.push({
      network: namedNetwork(network, host) ?? "website",
      url,
    });
  }
  return normalizeSocials(raw);
}

/** Social links from the agent's `socials` tag. Invalid links are dropped. */
export function parseSocialsTag(raw: string | undefined): OrgSocial[] {
  if (!raw?.trim()) return [];
  try {
    return parseSocialList(JSON.parse(raw) as unknown);
  } catch {
    return [];
  }
}

export function socialLabel(network: SocialNetwork): string {
  switch (network) {
    case "github":
      return "GitHub";
    case "x":
      return "X";
    case "linkedin":
      return "LinkedIn";
    case "nostr":
      return "Nostr";
    case "mastodon":
      return "Mastodon";
    case "telegram":
      return "Telegram";
    case "discord":
      return "Discord";
    case "youtube":
      return "YouTube";
    case "instagram":
      return "Instagram";
    case "bluesky":
      return "Bluesky";
    case "website":
      return "Website";
  }
}

/**
 * Parse a `39105` content body. Unknown or malformed JSON is empty — the
 * form still works; we do not treat a bad event as authoritative state.
 */
export function parseOrgProfileContent(content: string): OrgProfile {
  try {
    const record = asRecord(JSON.parse(content) as unknown);
    if (!record) return EMPTY_ORG_PROFILE;
    const about = typeof record.about === "string" ? record.about : "";
    const skills = Array.isArray(record.skills)
      ? record.skills
          .map(parseSkill)
          .filter((skill): skill is OrgSkill => skill !== null)
      : [];
    const openLimit =
      typeof record.open_limit === "number" &&
      Number.isInteger(record.open_limit) &&
      record.open_limit >= PROFILE_OPEN_LIMIT_MIN &&
      record.open_limit <= PROFILE_OPEN_LIMIT_MAX
        ? record.open_limit
        : null;
    return {
      about,
      openLimit,
      skills,
      socials: parseSocialList(record.socials),
    };
  } catch {
    return EMPTY_ORG_PROFILE;
  }
}

/** Newest `39105` for `d = pubkey`. Addressable: one head per member. */
export function newestOrgProfile(
  events: readonly Pick<
    RelayEvent,
    "kind" | "tags" | "content" | "created_at"
  >[],
  pubkey: string,
): OrgProfile {
  if (pubkey.length === 0) return EMPTY_ORG_PROFILE;
  const dTag = normalizePubkey(pubkey);
  const newest = events
    .filter(
      (event) =>
        event.kind === KIND_IO_PROFILE &&
        event.tags.some((tag) => tag[0] === "d" && tag[1] === dTag),
    )
    .sort((left, right) => right.created_at - left.created_at)[0];
  if (!newest) return EMPTY_ORG_PROFILE;
  return parseOrgProfileContent(newest.content);
}

/** Trim a typed skill label. Empty after trim is rejected by the caller. */
export function normalizeSkillLabel(value: string): string {
  return value.trim();
}

function labelsEqual(left: string, right: string): boolean {
  return left.localeCompare(right, undefined, { sensitivity: "accent" }) === 0;
}

/**
 * Append a skill chip. Rejects empty, over-long, duplicate, and the 21st
 * skill — the §4.7a limits, enforced before Save so the form matches the
 * command the relay will accept.
 */
export function addSkillLabel(
  skills: readonly string[],
  raw: string,
): { ok: true; skills: string[] } | { ok: false; error: string } {
  const label = normalizeSkillLabel(raw);
  if (label.length === 0) {
    return { ok: false, error: "Skill must not be empty." };
  }
  if (label.length > PROFILE_SKILL_LABEL_MAX_CHARS) {
    return {
      ok: false,
      error: `Each skill must be ${PROFILE_SKILL_LABEL_MAX_CHARS} characters or fewer.`,
    };
  }
  if (skills.some((existing) => labelsEqual(existing, label))) {
    return { ok: false, error: "That skill is already listed." };
  }
  if (skills.length >= PROFILE_MAX_SKILLS) {
    return {
      ok: false,
      error: `At most ${PROFILE_MAX_SKILLS} skills.`,
    };
  }
  return { ok: true, skills: [...skills, label] };
}

export function parseOpenLimitInput(
  value: string,
): { ok: true; openLimit?: number } | { ok: false; error: string } {
  const trimmed = value.trim();
  if (trimmed.length === 0) return { ok: true };
  const parsed = Number(trimmed);
  if (
    !Number.isInteger(parsed) ||
    parsed < PROFILE_OPEN_LIMIT_MIN ||
    parsed > PROFILE_OPEN_LIMIT_MAX
  ) {
    return {
      ok: false,
      error: `Open limit must be ${PROFILE_OPEN_LIMIT_MIN}–${PROFILE_OPEN_LIMIT_MAX}, or empty for no limit.`,
    };
  }
  return { ok: true, openLimit: parsed };
}

/** Validate the whole form into `buildIoProfileSet` fields. */
export function validateProfileSet(input: {
  about: string;
  openLimit: string;
  skills: readonly string[];
  socials?: readonly OrgSocial[];
}): ProfileSetValidation {
  if (input.about.length > PROFILE_ABOUT_MAX_CHARS) {
    return {
      ok: false,
      error: `About must be ${PROFILE_ABOUT_MAX_CHARS} characters or fewer.`,
    };
  }
  if (input.skills.length > PROFILE_MAX_SKILLS) {
    return { ok: false, error: `At most ${PROFILE_MAX_SKILLS} skills.` };
  }
  for (const label of input.skills) {
    if (label.length === 0 || label.length > PROFILE_SKILL_LABEL_MAX_CHARS) {
      return {
        ok: false,
        error: `Each skill must be ${PROFILE_SKILL_LABEL_MAX_CHARS} characters or fewer.`,
      };
    }
  }
  const limit = parseOpenLimitInput(input.openLimit);
  if (!limit.ok) return limit;
  const value: ProfileSetFields = {
    about: input.about,
    skills: [...input.skills],
    socials: normalizeSocials(input.socials ?? []),
  };
  if (limit.openLimit !== undefined) value.openLimit = limit.openLimit;
  return { ok: true, value };
}

/** Production seam: the unsigned `50021` Save will publish. */
export function buildValidatedProfileSet(input: {
  about: string;
  openLimit: string;
  skills: readonly string[];
  socials?: readonly OrgSocial[];
}): { ok: true; command: UnsignedOrgCommand } | { ok: false; error: string } {
  const validated = validateProfileSet(input);
  if (!validated.ok) return validated;
  return { ok: true, command: buildIoProfileSet(validated.value) };
}
