import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { installMockBridge } from "../helpers/bridge";
import { MOCK_VIEWER, orgEvent } from "./helpers/orgWork";

/**
 * Plan slice P-3 — the project draft card and the project page show the
 * change plan. Held steps are text. Agree emits 50004 whose content
 * includes the plan.
 */

const PLAN_DRAFT_ID = "c3".repeat(32);
const RECEIPT_ID = "e3".repeat(32);
const PROJECT_ID = "77777777-7777-4777-8777-777777777777";

const CHANGE = {
  from: "no weekday night",
  to: "a trial has answered whether buyers come",
  done_when: ["sessions held"],
};

const STEPS = [
  {
    piece: "Evening licence application",
    kind: "writing",
    gate: true,
    produces: ["sessions held"],
  },
  {
    piece: "Book the nights",
    kind: "ops",
    gate: false,
    held: "after the licence",
    after: ["Evening licence application"],
  },
];

function planEvents() {
  return [
    orgEvent({
      id: PLAN_DRAFT_ID,
      kind: 50100,
      pubkey: "a".repeat(64),
      createdAt: 1_700_000_200,
      tags: [
        ["n", MOCK_VIEWER],
        ["t", "project"],
        ["move", "1"],
        ["origin", "gap"],
        ["p", MOCK_VIEWER, "", "needs"],
        ["e", RECEIPT_ID, "", "receipt"],
        ["ref", "objectives@1#line-a"],
      ],
      content: {
        title: "Weekday hall trial",
        brief: "A short trial of a weekday night.",
        why: "learn whether weekday buyers come",
        due_at: 1_785_488_400,
        objective_ref: "objectives@1#line-a",
        change: CHANGE,
        plan: STEPS,
      },
    }),
    orgEvent({
      id: PROJECT_ID.replace(/-/g, "").padEnd(64, "0"),
      kind: 39101,
      createdAt: 1_700_000_210,
      tags: [
        ["d", PROJECT_ID],
        ["s", "accepted"],
        ["root", PROJECT_ID],
        ["t", "project"],
        ["p", MOCK_VIEWER],
      ],
      content: {
        id: PROJECT_ID,
        parent: null,
        root: PROJECT_ID,
        depth: 0,
        path: [],
        title: "Weekday hall trial",
        brief: "A short trial of a weekday night.",
        state: "accepted",
        dri: MOCK_VIEWER,
        due_at: 1_785_488_400,
        children: { open: 0, offered: 0, accepted: 0, done: 0 },
        change: CHANGE,
        plan: STEPS,
      },
    }),
  ];
}

async function openMyWork(page: Page) {
  await installMockBridge(page, { org: { events: planEvents() } });
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
  await page.getByTestId("sidebar-org-my-work").click();
  await expect(page.getByTestId("org-my-work-board")).toBeVisible();
}

async function signedOfKind(page: Page, kind: number) {
  return page.evaluate((wanted) => {
    return (window.__BUZZ_E2E_SIGNED_EVENTS__ ?? []).filter(
      (event) => event.kind === wanted,
    );
  }, kind);
}

test.describe("Org change plan on the card and the project page (P-3)", () => {
  test("01 — a seeded project draft shows the steps and Agree emits the plan", async ({
    page,
  }) => {
    await openMyWork(page);
    const card = page.getByTestId(`org-card-${PLAN_DRAFT_ID}`);
    await expect(card.getByTestId("org-plan")).toBeVisible();
    await expect(card.getByText("no weekday night")).toBeVisible();
    await expect(
      card.getByText("a trial has answered whether buyers come"),
    ).toBeVisible();
    await expect(card.getByTestId("org-plan-done-when")).toHaveText(
      "sessions held",
    );
    await expect(
      card.getByTestId("org-plan-step-Evening licence application"),
    ).toBeVisible();
    const held = card.getByTestId("org-plan-step-Book the nights");
    await expect(held).toContainText("held");
    await expect(held.getByRole("button")).toHaveCount(0);

    await card.getByTestId("org-card-agree").click();
    await expect
      .poll(async () => (await signedOfKind(page, 50004)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50004);
    const content = JSON.parse(signed[0]?.content ?? "{}") as {
      plan?: Array<{ piece?: string }>;
    };
    expect(content.plan?.map((step) => step.piece)).toEqual([
      "Evening licence application",
      "Book the nights",
    ]);
  });

  test("02 — the project page renders the same plan", async ({ page }) => {
    await installMockBridge(page, { org: { events: planEvents() } });
    await page.goto("/");
    await page.getByTestId("channel-general").click();
    await page.goto(`/#/org/work/${PROJECT_ID}`);
    await expect(page.getByTestId("org-item-title")).toHaveText(
      "Weekday hall trial",
    );
    await expect(page.getByTestId("org-plan")).toBeVisible();
    await expect(page.getByTestId("org-plan-done-when")).toHaveText(
      "sessions held",
    );
    const held = page.getByTestId("org-plan-step-Book the nights");
    await expect(held).toContainText("held");
    await expect(held.getByRole("button")).toHaveCount(0);
  });
});
