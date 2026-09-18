// Nostr event construction for the fixtures: deterministic keys, ids, and
// signatures (Prototype map § Determinism).

import { bigToBytes, bytesToBig, hex, privateKeyFromName, publicKey, schnorrSign, sha256 } from "./crypto.mjs";

const keyCache = new Map();

// One key pair per name, shared across every org: `sha256("io-fixture:" + name)`.
export function keyFor(name) {
  let entry = keyCache.get(name);
  if (!entry) {
    const privateKey = privateKeyFromName(name);
    entry = { name, privateKey, pubkey: hex(publicKey(privateKey)) };
    keyCache.set(name, entry);
  }
  return entry;
}

// A lowercase RFC 4122 UUID (version 4 layout, variant 10) derived from a
// stable key, so object ids never move between generator runs.
export function uuidFor(...keyParts) {
  const digest = sha256(`io-fixture:uuid:${keyParts.join(":")}`);
  const bytes = Buffer.from(digest.subarray(0, 16));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const h = hex(bytes);
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}

// `l_<8 hex>` line id from the English text, stable across versions and
// locales (Protocol § 4.1).
export function lineIdFor(org, slug, text) {
  return `l_${hex(sha256(`io-fixture:line:${org}:${slug}:${text}`)).slice(0, 8)}`;
}

// Serialize exactly as serde_json does for `[0, pubkey, created_at, kind,
// tags, content]`: no whitespace, `"`/`\`/controls escaped, UTF-8 kept.
// JSON.stringify matches that byte for byte for the strings we emit.
export function eventId(pubkey, createdAt, kind, tags, content) {
  return hex(sha256(JSON.stringify([0, pubkey, createdAt, kind, tags, content])));
}

// Build and sign one event. `content` may be a string or a JSON value;
// values are serialized with `JSON.stringify` (key order as inserted).
export function signEvent({ signer, kind, createdAt, tags, content }) {
  if (!Number.isInteger(createdAt)) throw new Error(`created_at must be an integer, got ${createdAt}`);
  if (!Number.isInteger(kind)) throw new Error(`kind must be an integer, got ${kind}`);
  for (const tag of tags) {
    if (!Array.isArray(tag) || tag.length === 0 || tag.some((t) => typeof t !== "string")) {
      throw new Error(`malformed tag ${JSON.stringify(tag)} on kind ${kind}`);
    }
  }
  const body = typeof content === "string" ? content : JSON.stringify(content);
  const key = keyFor(signer);
  const id = eventId(key.pubkey, createdAt, kind, tags, body);
  const sig = hex(schnorrSign(Buffer.from(id, "hex"), key.privateKey));
  return { id, pubkey: key.pubkey, created_at: createdAt, kind, tags, content: body, sig };
}

export { bigToBytes, bytesToBig, hex, sha256 };
