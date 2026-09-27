import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";
import {
  ASKING_ID,
  DECISION_ID,
  DONE_ID,
  DRAFTED_ID,
  ITEM_DONE,
  myWorkSeedEvents,
  OFFER_EVENT_ID,
  REVIEW_ID,
  SUGGEST_ID,
} from "../helpers/orgMyWork";

/**
 * Plan slice D-2 — My Work and the card set (Phase 0 § My Work, § Cards
 * by move; Design § Surfaces cards; Prototype map § Cards):
 *
 *   - three columns; each card type from a seeded 50100 / 39101 / 39102
 *   - kickers: AI is asking you / AI is suggesting for (name) / Drafted by the agent
 *   - receipts; n of needed
 *   - Agree carries the draft e tag (same sign_event capture as D-0)
 *   - Decline emits 50012 with the chosen reason chip
 *   - Inbox needs_action renders the same cards
 *   - keyboard path for every tap (rule 8); a11y (rule 7)
 */

const SHOTS = "test-results/org-my-work";

async function openMyWork(page: Page) {
  await installMockBridge(page, { org: { events: myWorkSeedEvents() } });
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

test.describe("Org My Work — the card set (D-2)", () => {
  test("01 — three columns and each card type from seeded events", async ({
    page,
  }) => {
    await openMyWork(page);

    const needs = page.getByTestId("org-my-work-column-needs-answer");
    const held = page.getByTestId("org-my-work-column-you-hold");
    const offered = page.getByTestId("org-my-work-column-you-offered");
    await expect(needs).toBeVisible();
    await expect(held).toBeVisible();
    await expect(offered).toBeVisible();

    await expect(page.getByTestId(`org-card-${ASKING_ID}`)).toHaveAttribute(
      "data-card-type",
      "draft",
    );
    await expect(
      page.getByTestId(`org-card-${OFFER_EVENT_ID}`),
    ).toHaveAttribute("data-card-type", "offer");
    await expect(page.getByTestId(`org-card-${DECISION_ID}`)).toHaveAttribute(
      "data-card-type",
      "decision",
    );
    await expect(page.getByTestId(`org-card-${DONE_ID}`)).toHaveAttribute(
      "data-card-type",
      "done",
    );
    await expect(page.getByTestId(`org-card-${REVIEW_ID}`)).toHaveAttribute(
      "data-card-type",
      "review",
    );

    await expect(held.getByText("Weekday hall")).toBeVisible();
    await expect(offered.getByText("Flyer run")).toBeVisible();

    await waitForAnimations(page);
    await page.getByTestId("org-my-work-board").screenshot({
      path: `${SHOTS}/01-three-columns.png`,
    });
  });

  test("02 — kickers, receipts, and n of needed", async ({ page }) => {
    await openMyWork(page);

    const asking = page.getByTestId(`org-card-${ASKING_ID}`);
    await expect(asking.getByTestId("org-card-kicker")).toHaveText(
      "AI is asking you",
    );
    await expect(asking).toHaveAttribute("data-kicker", "asking");
    await expect(asking.getByTestId("org-card-receipts")).toBeVisible();
    await expect(
      asking.getByRole("list", { name: "Receipts" }).getByRole("listitem"),
    ).toHaveCount(2);

    await expect(
      page.getByTestId(`org-card-${SUGGEST_ID}`).getByTestId("org-card-kicker"),
    ).toHaveText(/^AI is suggesting for /);
    await expect(page.getByTestId(`org-card-${SUGGEST_ID}`)).toHaveAttribute(
      "data-kicker",
      "suggesting",
    );

    await expect(
      page.getByTestId(`org-card-${DRAFTED_ID}`).getByTestId("org-card-kicker"),
    ).toHaveText("Drafted by the agent");
    await expect(page.getByTestId(`org-card-${DRAFTED_ID}`)).toHaveAttribute(
      "data-kicker",
      "drafted",
    );

    await expect(
      page
        .getByTestId(`org-card-${DECISION_ID}`)
        .getByTestId("org-card-needed"),
    ).toHaveText("1 of 2");
  });

  test("03 — Agree carries the draft e tag on sign_event", async ({ page }) => {
    await openMyWork(page);
    await expect
      .poll(async () =>
        page.evaluate(() => Boolean(window.__BUZZ_E2E_ORG_COMMANDS__)),
      )
      .toBe(true);

    const card = page.getByTestId(`org-card-${ASKING_ID}`);
    await card.getByTestId("org-card-agree").click();

    await expect
      .poll(async () => (await signedOfKind(page, 50004)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50004);
    expect(signed[0]?.tags).toEqual(
      expect.arrayContaining([
        ["e", ASKING_ID, "", "draft"],
        ["vote", "agree"],
      ]),
    );
  });

  test("04 — Decline emits 50012 with the chosen reason chip", async ({
    page,
  }) => {
    await openMyWork(page);
    const card = page.getByTestId(`org-card-${ASKING_ID}`);
    await card.getByTestId("org-card-reason-already_covered").click();
    await card.getByTestId("org-card-decline").click();

    await expect
      .poll(async () => (await signedOfKind(page, 50012)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50012);
    expect(signed[0]?.tags).toEqual([
      ["e", ASKING_ID],
      ["outcome", "decline"],
      ["reason", "already_covered"],
    ]);
  });

  test("05 — Inbox needs_action renders the same cards", async ({ page }) => {
    await installMockBridge(page, { org: { events: myWorkSeedEvents() } });
    await page.goto("/");
    await page.getByTestId("channel-general").click();
    await page.getByRole("button", { name: "Inbox" }).click();
    await expect(page.getByTestId("home-inbox")).toBeVisible();
    await page.getByTestId("inbox-filter-trigger").click();
    await page.getByRole("menuitemradio", { name: "Needs action" }).click();

    await page.getByTestId(`home-inbox-item-${ASKING_ID}`).click();
    await expect(page.getByTestId("org-inbox-card")).toBeVisible();
    const card = page.getByTestId(`org-card-${ASKING_ID}`);
    await expect(card.getByTestId("org-card-kicker")).toHaveText(
      "AI is asking you",
    );
    await expect(
      card.getByRole("button", { name: "Agree", exact: true }),
    ).toBeVisible();

    await waitForAnimations(page);
    await page.getByTestId("org-inbox-card").screenshot({
      path: `${SHOTS}/05-inbox-card.png`,
    });
  });

  test("06 — keyboard path for every tap and a11y audit", async ({ page }) => {
    await openMyWork(page);

    const asking = page.getByTestId(`org-card-${ASKING_ID}`);
    await expect(asking).toHaveAttribute(
      "aria-labelledby",
      `org-card-claim-${ASKING_ID}`,
    );
    await expect(
      asking.getByRole("button", { name: "Agree", exact: true }),
    ).toBeVisible();
    await expect(
      asking.getByRole("button", { name: "Edit then agree" }),
    ).toBeVisible();
    await expect(asking.getByRole("button", { name: "Decline" })).toBeVisible();
    await expect(
      asking.getByRole("group", { name: "Decline reason" }),
    ).toBeVisible();
    await expect(asking.getByRole("list", { name: "Receipts" })).toBeVisible();

    await asking.getByRole("button", { name: "Edit then agree" }).focus();
    await asking
      .getByRole("button", { name: "Edit then agree" })
      .press("Enter");
    await expect(
      asking.getByRole("group", { name: "Edit then agree" }),
    ).toBeVisible();
    await asking
      .getByRole("button", { name: "Agree with edit" })
      .press("Enter");
    await expect
      .poll(async () => (await signedOfKind(page, 50004)).length)
      .toBeGreaterThan(0);

    const offer = page.getByTestId(`org-card-${OFFER_EVENT_ID}`);
    await offer.getByRole("button", { name: "Accept" }).press("Enter");
    await expect
      .poll(async () => (await signedOfKind(page, 50007)).length)
      .toBeGreaterThan(0);

    const decision = page.getByTestId(`org-card-${DECISION_ID}`);
    await decision.getByRole("button", { name: "Agree" }).press("Enter");
    await expect
      .poll(async () => (await signedOfKind(page, 50003)).length)
      .toBeGreaterThan(0);

    const done = page.getByTestId(`org-card-${DONE_ID}`);
    await done.getByRole("button", { name: "Mark done" }).press("Enter");
    await expect
      .poll(async () => (await signedOfKind(page, 50009)).length)
      .toBeGreaterThan(0);
    const doneSigned = await signedOfKind(page, 50009);
    expect(doneSigned[0]?.tags).toEqual(
      expect.arrayContaining([
        ["i", ITEM_DONE],
        ["e", DONE_ID, "", "draft"],
      ]),
    );

    await done.getByRole("radio", { name: "not now" }).press("Space");
    await done.getByRole("button", { name: "Not yet" }).press("Enter");
    await expect
      .poll(async () => (await signedOfKind(page, 50012)).length)
      .toBeGreaterThan(0);

    const review = page.getByTestId(`org-card-${REVIEW_ID}`);
    await review
      .getByRole("button", { name: "Open the follow-up" })
      .press("Enter");
    await expect
      .poll(async () => (await signedOfKind(page, 50004)).length)
      .toBeGreaterThan(1);
    await review.getByRole("button", { name: "Nothing more" }).press("Enter");
    await review
      .getByRole("button", { name: "Keep open until" })
      .press("Enter");
    await expect
      .poll(async () => (await signedOfKind(page, 50011)).length)
      .toBeGreaterThan(0);
  });
});
