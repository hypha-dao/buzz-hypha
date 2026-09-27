import assert from "node:assert/strict";
import test from "node:test";

import {
  KIND_IO_PROFILE,
  KIND_IO_PROFILE_SET,
} from "../../shared/constants/kinds.ts";
import { buildIoProfileSet } from "./commands.ts";
import {
  addSkillLabel,
  buildValidatedProfileSet,
  newestOrgProfile,
  parseOpenLimitInput,
  parseOrgProfileContent,
  PROFILE_ABOUT_MAX_CHARS,
  PROFILE_MAX_SKILLS,
  PROFILE_SKILL_LABEL_MAX_CHARS,
  validateProfileSet,
} from "./profile.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const OTHER =
  "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";

function profileEvent(content, createdAt, pubkey = ME) {
  return {
    id: String(createdAt).padStart(64, "0"),
    pubkey: "f".repeat(64),
    created_at: createdAt,
    kind: KIND_IO_PROFILE,
    tags: [
      ["d", pubkey],
      ["p", pubkey],
    ],
    content,
    sig: "",
  };
}

test("parseOrgProfileContent reads 39105 about, skill labels, open_limit", () => {
  const parsed = parseOrgProfileContent(
    JSON.stringify({
      pubkey: ME,
      version: 2,
      about: "I run the Tuesday kitchen.",
      skills: [{ slug: "grant-writing", label: "grant writing" }],
      open_limit: 3,
      updated_at: 0,
      receipt: "ab".repeat(32),
    }),
  );
  assert.equal(parsed.about, "I run the Tuesday kitchen.");
  assert.deepEqual(parsed.skills, [
    { slug: "grant-writing", label: "grant writing" },
  ]);
  assert.equal(parsed.openLimit, 3);
});

test("parseOrgProfileContent treats missing or invalid content as empty", () => {
  assert.deepEqual(parseOrgProfileContent("{"), {
    about: "",
    openLimit: null,
    skills: [],
  });
  assert.deepEqual(parseOrgProfileContent("[]"), {
    about: "",
    openLimit: null,
    skills: [],
  });
  assert.equal(
    parseOrgProfileContent(JSON.stringify({ open_limit: 0 })).openLimit,
    null,
  );
  assert.equal(
    parseOrgProfileContent(JSON.stringify({ open_limit: 51 })).openLimit,
    null,
  );
});

test("newestOrgProfile is the latest 39105 for that d tag", () => {
  const events = [
    profileEvent(JSON.stringify({ about: "old", skills: [] }), 1),
    profileEvent(JSON.stringify({ about: "new", skills: ["spanish"] }), 9),
    profileEvent(JSON.stringify({ about: "other", skills: [] }), 20, OTHER),
  ];
  const newest = newestOrgProfile(events, ME);
  assert.equal(newest.about, "new");
  assert.deepEqual(newest.skills, [{ label: "spanish", slug: "spanish" }]);
  assert.deepEqual(newestOrgProfile([], ME), {
    about: "",
    openLimit: null,
    skills: [],
  });
});

test("addSkillLabel enforces §4.7a chip limits", () => {
  assert.equal(addSkillLabel([], "  ").ok, false);
  assert.equal(
    addSkillLabel([], "x".repeat(PROFILE_SKILL_LABEL_MAX_CHARS + 1)).ok,
    false,
  );
  const added = addSkillLabel(["grant writing"], "Grant writing");
  assert.equal(added.ok, false);
  const next = addSkillLabel(["grant writing"], "hosting events");
  assert.deepEqual(next, {
    ok: true,
    skills: ["grant writing", "hosting events"],
  });
  const full = Array.from(
    { length: PROFILE_MAX_SKILLS },
    (_, index) => `s${index}`,
  );
  assert.equal(addSkillLabel(full, "one more").ok, false);
});

test("validateProfileSet + buildIoProfileSet is the full 50021 body", () => {
  const validated = validateProfileSet({
    about: "I wire halls.",
    skills: ["electrics", "hosting events"],
    openLimit: "3",
  });
  assert.equal(validated.ok, true);
  if (!validated.ok) return;
  const command = buildIoProfileSet(validated.value);
  assert.equal(command.kind, KIND_IO_PROFILE_SET);
  assert.deepEqual(command.tags, []);
  assert.equal(
    command.content,
    JSON.stringify({
      about: "I wire halls.",
      skills: ["electrics", "hosting events"],
      open_limit: 3,
    }),
  );
});

test("an empty profile is allowed and omits open_limit", () => {
  const built = buildValidatedProfileSet({
    about: "",
    skills: [],
    openLimit: "",
  });
  assert.equal(built.ok, true);
  if (!built.ok) return;
  assert.equal(built.command.kind, KIND_IO_PROFILE_SET);
  assert.equal(
    built.command.content,
    JSON.stringify({ about: "", skills: [] }),
  );
  assert.ok(!built.command.tags.some((tag) => tag[0] === "p"));
});

test("validateProfileSet refuses over-long about and a bad limit", () => {
  assert.equal(
    validateProfileSet({
      about: "x".repeat(PROFILE_ABOUT_MAX_CHARS + 1),
      skills: [],
      openLimit: "",
    }).ok,
    false,
  );
  assert.equal(parseOpenLimitInput("0").ok, false);
  assert.equal(parseOpenLimitInput("51").ok, false);
  assert.deepEqual(parseOpenLimitInput(""), { ok: true });
  assert.deepEqual(parseOpenLimitInput("3"), { ok: true, openLimit: 3 });
});
