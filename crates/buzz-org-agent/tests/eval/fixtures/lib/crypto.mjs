// Pure-JS secp256k1 + BIP-340 Schnorr, enough to sign Nostr events
// deterministically. No dependency so the generator runs on a bare
// `node` (the CI policy job installs nothing). Signatures use an all-zero
// `aux_rand`, which BIP-340 permits and which makes the output a pure
// function of (key, message) — the property the idempotency check needs.
// Verified end to end by `crates/buzz-org-agent/tests/eval_fixtures.rs`,
// which runs `buzz_core::verify_event` (rust-secp256k1) over every event.

import { createHash } from "node:crypto";

const P = 0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2fn;
const N = 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141n;
const GX = 0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798n;
const GY = 0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8n;

export function sha256(...parts) {
  const h = createHash("sha256");
  for (const part of parts) {
    h.update(typeof part === "string" ? Buffer.from(part, "utf8") : part);
  }
  return h.digest();
}

export function hex(bytes) {
  return Buffer.from(bytes).toString("hex");
}

export function bytesToBig(bytes) {
  return BigInt(`0x${hex(bytes)}`);
}

export function bigToBytes(value, length = 32) {
  return Buffer.from(value.toString(16).padStart(length * 2, "0"), "hex");
}

function mod(a, m = P) {
  const r = a % m;
  return r >= 0n ? r : r + m;
}

function modPow(base, exp, m) {
  let result = 1n;
  let b = mod(base, m);
  let e = exp;
  while (e > 0n) {
    if (e & 1n) result = (result * b) % m;
    b = (b * b) % m;
    e >>= 1n;
  }
  return result;
}

function inv(a, m = P) {
  return modPow(a, m - 2n, m);
}

// Jacobian coordinates: one field inversion per scalar multiplication
// instead of one per point addition.
const INF = null;

function jDouble(pt) {
  if (pt === INF) return INF;
  const [x, y, z] = pt;
  if (y === 0n) return INF;
  const ysq = mod(y * y);
  const s = mod(4n * x * ysq);
  const m = mod(3n * x * x);
  const nx = mod(m * m - 2n * s);
  const ny = mod(m * (s - nx) - 8n * ysq * ysq);
  const nz = mod(2n * y * z);
  return [nx, ny, nz];
}

function jAdd(a, b) {
  if (a === INF) return b;
  if (b === INF) return a;
  const [x1, y1, z1] = a;
  const [x2, y2, z2] = b;
  const z1z1 = mod(z1 * z1);
  const z2z2 = mod(z2 * z2);
  const u1 = mod(x1 * z2z2);
  const u2 = mod(x2 * z1z1);
  const s1 = mod(y1 * z2 * z2z2);
  const s2 = mod(y2 * z1 * z1z1);
  if (u1 === u2) {
    return s1 === s2 ? jDouble(a) : INF;
  }
  const h = mod(u2 - u1);
  const i = mod(4n * h * h);
  const j = mod(h * i);
  const r = mod(2n * (s2 - s1));
  const v = mod(u1 * i);
  const nx = mod(r * r - j - 2n * v);
  const ny = mod(r * (v - nx) - 2n * s1 * j);
  const nz = mod(((z1 + z2) * (z1 + z2) - z1z1 - z2z2) * h);
  return [nx, ny, nz];
}

function jMul(pt, k) {
  let result = INF;
  let addend = pt;
  let e = mod(k, N);
  while (e > 0n) {
    if (e & 1n) result = jAdd(result, addend);
    addend = jDouble(addend);
    e >>= 1n;
  }
  return result;
}

function toAffine(pt) {
  if (pt === INF) throw new Error("point at infinity");
  const [x, y, z] = pt;
  const zi = inv(z);
  const zi2 = mod(zi * zi);
  return [mod(x * zi2), mod(y * zi2 * zi)];
}

const G = [GX, GY, 1n];

function mulG(k) {
  return toAffine(jMul(G, k));
}

function taggedHash(tag, ...parts) {
  const tagHash = sha256(tag);
  return sha256(tagHash, tagHash, ...parts);
}

// A private key from a name: `sha256("io-fixture:" + name)` (Prototype
// map § Determinism). Reduced into `1..n` for safety; the hash is never
// that large in practice.
export function privateKeyFromName(name) {
  const d = mod(bytesToBig(sha256(`io-fixture:${name}`)), N - 1n) + 1n;
  return bigToBytes(d);
}

export function publicKey(privateKey) {
  const [x] = mulG(bytesToBig(privateKey));
  return bigToBytes(x);
}

// BIP-340 sign with `aux_rand = 0^32`.
export function schnorrSign(message, privateKey) {
  const aux = Buffer.alloc(32);
  let d = bytesToBig(privateKey);
  const [px, py] = mulG(d);
  if (py & 1n) d = N - d;
  const pxBytes = bigToBytes(px);
  const t = bigToBytes(d ^ bytesToBig(taggedHash("BIP0340/aux", aux)));
  const rand = taggedHash("BIP0340/nonce", t, pxBytes, message);
  let k = mod(bytesToBig(rand), N);
  if (k === 0n) throw new Error("zero nonce");
  const [rx, ry] = mulG(k);
  if (ry & 1n) k = N - k;
  const rxBytes = bigToBytes(rx);
  const e = mod(bytesToBig(taggedHash("BIP0340/challenge", rxBytes, pxBytes, message)), N);
  const s = mod(k + e * d, N);
  return Buffer.concat([rxBytes, bigToBytes(s)]);
}
