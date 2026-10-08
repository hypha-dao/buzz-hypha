import assert from "node:assert/strict";
import test from "node:test";

import { KIND_IO_PROPOSAL } from "../../shared/constants/kinds.ts";
import {
  chatProposalPreview,
  currentDirectionProposals,
  proposalCardFace,
  proposalClaimLines,
  proposalDetail,
  proposalKindLabel,
  proposalMessageId,
  proposalPage,
} from "./proposalDetail.ts";

const PROPOSAL = "33333333-3333-4333-8333-333333333333";

function proposal(content, tags = [["d", PROPOSAL]]) {
  return {
    kind: KIND_IO_PROPOSAL,
    content: JSON.stringify(content),
    tags,
  };
}

test("a project proposal expands to its brief", () => {
  const detail = proposalDetail(
    proposal({
      kind: "project",
      payload: {
        title: "Weekday hall",
        brief: "Book the hall on weekdays.",
        due_at: 1_700_000_000,
      },
    }),
  );
  assert.equal(detail.title, "Weekday hall");
  assert.equal(detail.body, "Book the hall on weekdays.");
  assert.equal(detail.dueAt, 1_700_000_000);
});

test("a direction proposal expands to its sentence", () => {
  const detail = proposalDetail(
    proposal({
      kind: "direction",
      payload: { slug: "mission", body: "We host the hall for the town." },
    }),
  );
  assert.equal(detail.slug, "mission");
  assert.equal(detail.body, "We host the hall for the town.");
});

test("a strategy proposal card shows each line on its own", () => {
  const stored = proposal({
    kind: "direction",
    payload: {
      slug: "strategy",
      body: "We refuse new orgs. Hypha comes first. Hypha Buzz wins when the two compete.",
      lines: [
        { text: "We refuse new orgs. Hypha comes first." },
        { text: "Hypha Buzz wins when the two compete." },
      ],
    },
  });
  assert.deepEqual(proposalClaimLines(stored), [
    "We refuse new orgs. Hypha comes first.",
    "Hypha Buzz wins when the two compete.",
  ]);
  const paragraph = proposal({
    kind: "direction",
    payload: {
      slug: "objectives",
      body: "Book the hall by March. Pay growers the week they sell.",
    },
  });
  assert.deepEqual(proposalClaimLines(paragraph), [
    "Book the hall by March.",
    "Pay growers the week they sell.",
  ]);
  const mission = proposal({
    kind: "direction",
    payload: { slug: "mission", body: "We host the hall. For the town." },
  });
  assert.deepEqual(proposalClaimLines(mission), []);
});

test("each direction proposal names itself and opens as a proposal", () => {
  for (const [slug, label] of [
    ["mission", "Mission proposal"],
    ["vision", "Vision proposal"],
    ["situation", "Situation proposal"],
    ["objectives", "Objectives proposal"],
    ["strategy", "Strategy proposal"],
  ]) {
    const opened = proposal(
      {
        kind: "direction",
        status: "open",
        payload: {
          slug,
          body: `The ${slug} text.`,
          lines:
            slug === "objectives" || slug === "strategy"
              ? [{ text: "First line" }, { text: "Second line" }]
              : undefined,
        },
      },
      [
        ["d", PROPOSAL],
        ["s", "open"],
      ],
    );
    assert.equal(proposalKindLabel(opened), label);
    assert.deepEqual(proposalCardFace(opened, "Needs your answer"), {
      label,
      asker: null,
    });
    assert.deepEqual(proposalCardFace(opened, "Maya is asking you"), {
      label,
      asker: "Maya is asking you",
    });
    const page = proposalPage(opened);
    assert.equal(page.eyebrow, label);
    assert.equal(page.parentTo, "/org/my-work");
    assert.equal(page.directionSlug, slug);
    if (slug === "objectives" || slug === "strategy") {
      assert.deepEqual(
        page.lines.map((line) => line.text),
        ["First line", "Second line"],
      );
      assert.equal(page.brief, `The ${slug} text.`);
    } else {
      assert.equal(page.title, `The ${slug} text.`);
      assert.equal(page.brief, "");
      assert.deepEqual(page.lines, []);
    }
  }
  const repeated = proposalPage(
    proposal({
      kind: "direction",
      payload: {
        slug: "objectives",
        body: "Book the hall by March.\nPay the growers the week they sell.",
        lines: [
          { text: "Book the hall by March." },
          { text: "Pay the growers the week they sell." },
        ],
      },
    }),
  );
  assert.equal(repeated.brief, "");
  assert.equal(repeated.lines.length, 2);

  const stated = [
    "Vlad runs Hypha Buzz's daily work in the desktop app. Done when: Vlad takes his next task from the AI each workday. By: 2026-10-15.",
    "A separate Hypha org exists in the app. Done when: the org's direction is set in the app. By: 2026-10-31.",
  ].join("\n");
  const withChecks = proposalPage(
    proposal({
      kind: "direction",
      payload: {
        slug: "objectives",
        body: stated,
        lines: [
          { text: "Vlad runs Hypha Buzz's daily work in the desktop app" },
          { text: "A separate Hypha org exists in the app" },
        ],
      },
    }),
  );
  assert.equal(withChecks.brief, "");
  const storedChecks = proposalPage(
    proposal({
      kind: "direction",
      payload: {
        slug: "objectives",
        body: stated,
        lines: [
          {
            text: "Vlad runs Hypha Buzz's daily work in the desktop app",
            done_when: "Vlad takes his next task from the AI each workday",
            date: Date.parse("2026-10-15T00:00:00Z") / 1000,
          },
          {
            text: "A separate Hypha org exists in the app",
            done_when: "the org's direction is set in the app",
            date: Date.parse("2026-10-31T00:00:00Z") / 1000,
          },
        ],
      },
    }),
  );
  assert.equal(storedChecks.brief, "");
  assert.equal(
    storedChecks.lines[0]?.doneWhen,
    "Vlad takes his next task from the AI each workday",
  );
  assert.deepEqual(
    withChecks.lines.map((line) => ({
      text: line.text,
      doneWhen: line.doneWhen,
      date: line.date,
    })),
    [
      {
        text: "Vlad runs Hypha Buzz's daily work in the desktop app",
        doneWhen: "Vlad takes his next task from the AI each workday",
        date: Date.parse("2026-10-15T00:00:00Z") / 1000,
      },
      {
        text: "A separate Hypha org exists in the app",
        doneWhen: "the org's direction is set in the app",
        date: Date.parse("2026-10-31T00:00:00Z") / 1000,
      },
    ],
  );
  const aside = proposalPage(
    proposal({
      kind: "direction",
      payload: {
        slug: "objectives",
        body: `Vote on these before Friday.\n${stated}`,
        lines: [
          {
            text: "Vlad runs Hypha Buzz's daily work in the desktop app",
            done_when: "Vlad takes his next task from the AI each workday",
            date: Date.parse("2026-10-15T00:00:00Z") / 1000,
          },
          {
            text: "A separate Hypha org exists in the app",
            done_when: "the org's direction is set in the app",
            date: Date.parse("2026-10-31T00:00:00Z") / 1000,
          },
        ],
      },
    }),
  );
  assert.match(aside.brief, /Vote on these before Friday/);

  const project = proposal({
    kind: "project",
    payload: { title: "Weekday hall", brief: "Book it." },
  });
  assert.equal(proposalKindLabel(project), "Project proposal");
});

test("the card hangs on the agent message that asked for it", () => {
  const detail = proposalDetail(
    proposal({
      kind: "project",
      payload: { title: "Weekday hall", brief: "Book it.", due_at: 10 },
    }),
  );
  const messageId = proposalMessageId(
    [
      {
        id: "older",
        createdAt: 1,
        tags: [["project", "Something else", "no"]],
      },
      {
        id: "match",
        createdAt: 2,
        tags: [
          ["project", "Weekday hall", "Book it."],
          ["due", "10"],
        ],
      },
    ],
    detail,
  );
  assert.equal(messageId, "match");
  const preview = chatProposalPreview({
    id: "match",
    createdAt: 2,
    tags: [
      ["project", "Weekday hall", "Book it."],
      ["due", "10"],
    ],
  });
  assert.equal(preview.title, "Weekday hall");
  assert.equal(preview.body, "Book it.");
  assert.equal(preview.dueAt, 10);
});

test("without a matching tag the card sits on the nearest chat line", () => {
  const detail = proposalDetail(
    proposal({
      kind: "project",
      payload: { title: "Weekday hall", brief: "Book it.", due_at: 10 },
    }),
  );
  const messageId = proposalMessageId(
    [
      { id: "ask", createdAt: 100, tags: [] },
      { id: "later", createdAt: 500, tags: [] },
    ],
    detail,
    120,
  );
  assert.equal(messageId, "ask");
});

test("a revise sits on the message that names that proposal", () => {
  const detail = proposalDetail(
    proposal(
      {
        kind: "direction",
        payload: { slug: "mission", body: "We host the river." },
      },
      [["d", PROPOSAL]],
    ),
  );
  const messageId = proposalMessageId(
    [
      {
        id: "first",
        createdAt: 1,
        tags: [["direction", "mission", "We host the hall."]],
      },
      {
        id: "revised",
        createdAt: 2,
        tags: [
          ["revise", PROPOSAL, "direction", "mission", "We host the river."],
        ],
      },
    ],
    detail,
  );
  assert.equal(messageId, "revised");
});

test("a proposal does not hang on a chat that started after it", () => {
  const detail = proposalDetail(
    proposal({
      kind: "direction",
      payload: { slug: "mission", body: "We host the river." },
    }),
  );
  assert.equal(
    proposalMessageId(
      [{ id: "greeting", createdAt: 500, body: "Hey", tags: [] }],
      detail,
      100,
    ),
    null,
  );
});

test("an edited publish stays on the draft it came from", () => {
  const detail = proposalDetail(
    proposal({
      kind: "direction",
      payload: {
        slug: "mission",
        body: "We build an operating system for volunteer-run teams of five to twenty.",
      },
    }),
  );
  const messageId = proposalMessageId(
    [
      {
        id: "draft",
        createdAt: 10,
        tags: [
          [
            "direction",
            "mission",
            "We build an operating system for any group of people working together.",
          ],
        ],
      },
      { id: "later", createdAt: 40, tags: [] },
    ],
    detail,
    30,
  );
  assert.equal(messageId, "draft");
});

test("a new objectives draft does not collect earlier versions", () => {
  const firstId = "11111111-1111-4111-8111-111111111111";
  const secondId = "22222222-2222-4222-8222-222222222222";
  const first = proposalDetail(
    proposal(
      {
        kind: "direction",
        payload: {
          slug: "objectives",
          body: "Start using the tool internally in Hypha.",
        },
      },
      [["d", firstId]],
    ),
  );
  const second = proposalDetail(
    proposal(
      {
        kind: "direction",
        payload: {
          slug: "objectives",
          body: "Start using the tool internally in Hypha. Make the UI impressive.",
        },
      },
      [["d", secondId]],
    ),
  );
  const messages = [
    {
      id: "v1",
      createdAt: 1,
      tags: [
        [
          "direction",
          "objectives",
          "Start using the tool internally in Hypha.",
        ],
      ],
    },
    {
      id: "draft",
      createdAt: 9,
      tags: [
        [
          "direction",
          "objectives",
          "Start using the tool internally in Hypha. Make the UI impressive.",
        ],
      ],
    },
  ];
  assert.equal(proposalMessageId(messages, first, 2), "v1");
  assert.equal(proposalMessageId(messages, second, 8), "draft");
  assert.equal(
    proposalMessageId(
      [{ id: "draft", createdAt: 9, tags: messages[1].tags }],
      first,
      2,
    ),
    null,
  );
});

test("chat keeps the current direction and every project", () => {
  const older = "11111111-1111-4111-8111-111111111111";
  const newer = "22222222-2222-4222-8222-222222222222";
  const project = "33333333-3333-4333-8333-333333333333";
  const shown = currentDirectionProposals([
    {
      ...proposal(
        {
          kind: "direction",
          payload: { slug: "objectives", body: "Ship the first cut." },
        },
        [["d", older]],
      ),
      created_at: 1,
    },
    {
      ...proposal(
        {
          kind: "direction",
          payload: {
            slug: "objectives",
            body: "Ship the first cut. Then the UI.",
          },
        },
        [["d", newer]],
      ),
      created_at: 2,
    },
    {
      ...proposal(
        {
          kind: "project",
          payload: { title: "Weekday hall", brief: "Book it.", due_at: 10 },
        },
        [["d", project]],
      ),
      created_at: 3,
    },
  ]);
  assert.deepEqual(
    shown.map((event) => event.tags.find((tag) => tag[0] === "d")?.[1]),
    [project, newer],
  );
});

test("a publish announcement in #shapers carries the proposal", () => {
  const detail = proposalDetail(
    proposal({
      kind: "project",
      payload: { title: "Fix the hall", brief: "The roof leaks.", due_at: 1 },
    }),
  );
  const messageId = proposalMessageId(
    [
      {
        id: "older",
        createdAt: 1,
        body: "something else",
        tags: [],
      },
      {
        id: "announced",
        createdAt: 5,
        body: "Opened a project proposal: Fix the hall.\n\n[Open it in My work](/org/my-work)",
        tags: [],
      },
    ],
    detail,
    4,
  );
  assert.equal(messageId, "announced");
});

test("a codebases proposal hangs on its publish line", () => {
  const detail = proposalDetail(
    proposal(
      {
        kind: "codebases",
        payload: {
          title: "buzz-hypha",
          brief: "buzz-hypha — https://github.com/hypha-dao/buzz-hypha",
        },
      },
      [
        ["d", PROPOSAL],
        ["t", "codebases"],
        ["s", "open"],
      ],
    ),
  );
  assert.equal(detail.kind, "codebases");
  assert.equal(detail.title, "buzz-hypha");
  assert.equal(
    detail.body,
    "buzz-hypha — https://github.com/hypha-dao/buzz-hypha",
  );
  const messageId = proposalMessageId(
    [
      {
        id: "announced",
        createdAt: 5,
        body: "Opened a codebases proposal: buzz-hypha.",
        tags: [],
      },
    ],
    detail,
    6,
  );
  assert.equal(messageId, "announced");
});
