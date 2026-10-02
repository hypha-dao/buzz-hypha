import assert from "node:assert/strict";
import test from "node:test";

import {
  isDirectionBody,
  nextDirectionStatement,
} from "./directionFromChat.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const ADA = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const AGENT =
  "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";

function turn(id, pubkey, body, tags = [], createdAt = Number(id)) {
  return { id, createdAt, pubkey, body, tags };
}

test("a tagged reply proposes the member's previous sentence at base 0", () => {
  const statement = nextDirectionStatement({
    messages: [
      turn("1", ME, "We exist so organizations can run themselves."),
      turn("2", AGENT, "Got it. What's the vision?", [
        ["direction", "mission"],
      ]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [],
    handledIds: new Set(),
  });
  assert.deepEqual(statement, {
    kind: "propose",
    agentEventId: "2",
    slug: "mission",
    body: "We exist so organizations can run themselves.",
    base: 0,
  });
});

test("an existing head sets the base, and the same wording is skipped", () => {
  const same = nextDirectionStatement({
    messages: [
      turn("1", ME, "We exist so organizations can run themselves."),
      turn("2", AGENT, "Got it.", [["direction", "mission"]]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [
      {
        slug: "mission",
        version: 3,
        body: "We exist so organizations can run themselves.",
      },
    ],
    handledIds: new Set(),
  });
  assert.deepEqual(same, { kind: "skip", agentEventId: "2" });

  const next = nextDirectionStatement({
    messages: [
      turn("1", ME, "A community that decides its own work."),
      turn("2", AGENT, "That's the vision.", [["direction", "vision"]]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [{ slug: "vision", version: 1, body: "older vision" }],
    handledIds: new Set(),
  });
  assert.equal(next.kind, "propose");
  assert.equal(next.base, 1);
});

test("a short reply and a non-shaper do not become direction", () => {
  assert.equal(isDirectionBody("alone"), false);
  const short = nextDirectionStatement({
    messages: [
      turn("1", ME, "alone"),
      turn("2", AGENT, "What is this org for?", [["direction", "mission"]]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [],
    handledIds: new Set(),
  });
  assert.deepEqual(short, { kind: "skip", agentEventId: "2" });

  const outsider = nextDirectionStatement({
    messages: [
      turn("1", ME, "We exist so organizations can run themselves."),
      turn("2", AGENT, "Got it.", [["direction", "mission"]]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: ["ab".repeat(32)],
    heads: [],
    handledIds: new Set(),
  });
  assert.deepEqual(outsider, { kind: "skip", agentEventId: "2" });
});

test("in #shapers only the person the agent answered signs direction", () => {
  const statement = nextDirectionStatement({
    messages: [
      turn("1", ME, "We exist so organizations can run themselves."),
      turn("2", AGENT, "Got it.", [
        [
          "direction",
          "mission",
          "We exist so organizations can run themselves.",
        ],
        ["from", ADA],
      ]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME, ADA],
    heads: [],
    handledIds: new Set(),
    room: "shapers",
  });
  assert.deepEqual(statement, { kind: "skip", agentEventId: "2" });
});

test("a confirmation records the sentence the agent quoted, not the yes", () => {
  const statement = nextDirectionStatement({
    messages: [
      turn("1", ME, "yes thats good, set it"),
      turn(
        "2",
        AGENT,
        'Mission set: "We exist to build the operating system organizations need to run themselves." Next up — vision.',
        [
          [
            "direction",
            "mission",
            "We exist to build the operating system organizations need to run themselves.",
          ],
        ],
      ),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [],
    handledIds: new Set(),
  });
  assert.equal(statement.kind, "propose");
  assert.equal(
    statement.body,
    "We exist to build the operating system organizations need to run themselves.",
  );
});

test("a stored confirmation is replaced by the quoted mission", () => {
  const statement = nextDirectionStatement({
    messages: [
      turn("1", ME, "yes thats good, set it"),
      turn(
        "2",
        AGENT,
        'Mission set: "We exist to build the operating system organizations need to run themselves."',
        [["direction", "mission"]],
      ),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [{ slug: "mission", version: 2, body: "yes thats good, set it" }],
    handledIds: new Set(["2"]),
  });
  assert.deepEqual(statement, {
    kind: "propose",
    agentEventId: "fix:mission:2",
    slug: "mission",
    body: "We exist to build the operating system organizations need to run themselves.",
    base: 2,
  });
});

test("a second objective is kept with the first", () => {
  const first = "Start using the tool internally in Hypha.";
  const second =
    "Test the desktop app locally, deploy the relay, and release the app for others to download.";
  const statement = nextDirectionStatement({
    messages: [
      turn("2", AGENT, `Objective one set: "${first}"`, [
        ["direction", "objectives", first],
      ]),
      turn(
        "4",
        AGENT,
        `Objective two set: "${second}"`,
        [["direction", "objectives", second]],
        4,
      ),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [{ slug: "objectives", version: 1, body: first }],
    handledIds: new Set(["2"]),
  });
  assert.equal(statement.kind, "propose");
  assert.deepEqual(
    statement.lines.map((line) => line.text),
    [first, second],
  );
  assert.equal(statement.base, 1);
});

test("an overwritten objective list is restored from the chat", () => {
  const first = "Start using the tool internally in Hypha.";
  const second =
    "Test the desktop app locally, deploy the relay, and release the app for others to download.";
  const statement = nextDirectionStatement({
    messages: [
      turn("2", AGENT, `Objective one set: "${first}"`, [
        ["direction", "objectives", first],
      ]),
      turn(
        "4",
        AGENT,
        `Objective two set: "${second}"`,
        [["direction", "objectives", second]],
        4,
      ),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [{ slug: "objectives", version: 2, body: second, lines: [] }],
    handledIds: new Set(["2", "4"]),
  });
  assert.equal(statement.agentEventId, "fix:objectives:2:2");
  assert.deepEqual(
    statement.lines.map((line) => line.text),
    [first, second],
  );
});

test("a repeated objective blob is not written again", () => {
  const first = "Start using the tool internally in Hypha.";
  const second =
    "Test the desktop app locally, deploy the relay, and release the app for others to download.";
  const blob = `${first}\n${second}\n`.repeat(8);
  const statement = nextDirectionStatement({
    messages: [
      turn("2", AGENT, `Objective one set: "${first}"`, [
        ["direction", "objectives", first],
      ]),
      turn(
        "4",
        AGENT,
        `Objective two set: "${second}"`,
        [["direction", "objectives", second]],
        4,
      ),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [{ slug: "objectives", version: 15, body: blob, lines: [] }],
    handledIds: new Set(["2", "4"]),
  });
  assert.equal(statement, null);
});

test("shapers still loading waits, and a handled tag is ignored", () => {
  const waiting = nextDirectionStatement({
    messages: [
      turn("1", ME, "We exist so organizations can run themselves."),
      turn("2", AGENT, "Got it.", [["direction", "mission"]]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: null,
    heads: [],
    handledIds: new Set(),
  });
  assert.equal(waiting, "wait");

  const done = nextDirectionStatement({
    messages: [
      turn("1", ME, "We exist so organizations can run themselves."),
      turn("2", AGENT, "Got it.", [["direction", "mission"]]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    heads: [],
    handledIds: new Set(["2"]),
  });
  assert.equal(done, null);
});
