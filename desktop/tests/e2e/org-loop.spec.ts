import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";
import {
  CHILD_ID,
  DIRECTION_DECISION_ID,
  DIRECTION_PROPOSAL_ID,
  DONE_DRAFT_ID,
  DRI_DRAFT_ID,
  OFFER_CHILD_EVENT_ID,
  PROJECT_DECISION_ID,
  PROJECT_DRAFT_ID,
  PROJECT_PROPOSAL_ID,
  ROOT_ID,
  stageAccept,
  stageChildAndDone,
  stageDirectionConfirm,
  stageOffer,
  stageProjectCard,
  stageProposal1of2,
  stageRootOnWork,
} from "./helpers/orgLoop";

/**
 * Plan slice D-6 — Playwright coverage of the loop (Phase 0 § Keeping the
 * agent honest — desktop):
 *
 *   seeded direction confirm → project card → agree → proposal card with
 *   _1 of 2_ → pass → root on Work → offer → accept → child card → done card
 *
 * The mock bridge does not settle proposals, so each stage re-seeds the
 * next snapshot and asserts the tap's signed kind/tags.
 */

const SHOTS = "test-results/org-loop";

async function openApp(page: Page) {
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
}

async function openMyWork(page: Page) {
  await page.getByTestId("sidebar-org-my-work").click();
  await expect(page.getByTestId("org-my-work-board")).toBeVisible();
}

async function openWork(page: Page) {
  await page.getByTestId("sidebar-org-work").click();
  await expect(page.getByTestId("org-work-tree")).toBeVisible();
}

async function signedOfKind(page: Page, kind: number) {
  return page.evaluate((wanted) => {
    return (window.__BUZZ_E2E_SIGNED_EVENTS__ ?? []).filter(
      (event) => event.kind === wanted,
    );
  }, kind);
}

async function waitForCommands(page: Page) {
  await expect
    .poll(async () =>
      page.evaluate(() => Boolean(window.__BUZZ_E2E_ORG_COMMANDS__)),
    )
    .toBe(true);
}

test.describe("Org loop — direction confirm through done (D-6)", () => {
  test("01 — direction confirm Agree emits 50003", async ({ page }) => {
    await installMockBridge(page, {
      org: { events: stageDirectionConfirm() },
    });
    await openApp(page);
    await openMyWork(page);
    await waitForCommands(page);

    const card = page.getByTestId(`org-card-${DIRECTION_DECISION_ID}`);
    await expect(card).toHaveAttribute("data-card-type", "decision");
    await expect(card.getByTestId("org-card-vote")).toHaveCount(2);
    await expect(card.locator('[data-voted="true"]')).toHaveCount(1);
    await expect(card).toContainText("Confirm objectives");

    await waitForAnimations(page);
    await card.screenshot({ path: `${SHOTS}/01-direction-confirm.png` });

    await card.getByTestId("org-card-agree").click();
    await expect
      .poll(async () => (await signedOfKind(page, 50003)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50003);
    expect(signed[0]?.tags).toEqual(
      expect.arrayContaining([
        ["e", DIRECTION_PROPOSAL_ID],
        ["vote", "agree"],
      ]),
    );
  });

  test("02 — project card Agree emits 50004 with draft e", async ({ page }) => {
    await installMockBridge(page, { org: { events: stageProjectCard() } });
    await openApp(page);
    await openMyWork(page);
    await waitForCommands(page);

    const card = page.getByTestId(`org-card-${PROJECT_DRAFT_ID}`);
    await expect(card).toHaveAttribute("data-card-type", "draft");
    await expect(card.getByTestId("org-card-kicker")).toHaveText(
      "AI is asking you",
    );
    await expect(card).toContainText("Weekday hall");

    await waitForAnimations(page);
    await card.screenshot({ path: `${SHOTS}/02-project-card.png` });

    await card.getByTestId("org-card-agree").click();
    await expect
      .poll(async () => (await signedOfKind(page, 50004)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50004);
    expect(signed[0]?.tags).toEqual(
      expect.arrayContaining([
        ["e", PROJECT_DRAFT_ID, "", "draft"],
        ["vote", "agree"],
      ]),
    );
  });

  test("03 — proposal 1 of 2 Agree is the pass vote (50003)", async ({
    page,
  }) => {
    await installMockBridge(page, { org: { events: stageProposal1of2() } });
    await openApp(page);
    await openMyWork(page);
    await waitForCommands(page);

    const card = page.getByTestId(`org-card-${PROJECT_DECISION_ID}`);
    await expect(card).toHaveAttribute("data-card-type", "decision");
    await expect(card.getByTestId("org-card-vote")).toHaveCount(2);
    await expect(card.locator('[data-voted="true"]')).toHaveCount(1);
    await expect(card).toContainText("Weekday hall");

    await waitForAnimations(page);
    await page.getByTestId("org-my-work-board").screenshot({
      path: `${SHOTS}/03-proposal-1-of-2.png`,
    });

    await card.getByTestId("org-card-agree").click();
    await expect
      .poll(async () => (await signedOfKind(page, 50003)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50003);
    expect(signed[0]?.tags).toEqual(
      expect.arrayContaining([
        ["e", PROJECT_PROPOSAL_ID],
        ["vote", "agree"],
      ]),
    );
  });

  test("04 — passed project appears as a root on Work", async ({ page }) => {
    await installMockBridge(page, { org: { events: stageRootOnWork() } });
    await openApp(page);
    await openWork(page);

    await expect(page.getByTestId(`org-work-row-${ROOT_ID}`)).toContainText(
      "Weekday hall",
    );
    await expect(
      page.getByTestId(`org-work-row-${ROOT_ID}`).getByTestId("org-state-chip"),
    ).toHaveText("needs a DRI");

    await waitForAnimations(page);
    await page.getByTestId("org-work").screenshot({
      path: `${SHOTS}/04-root-on-work.png`,
    });
  });

  test("05 — Offer on the DRI draft emits 50006", async ({ page }) => {
    await installMockBridge(page, { org: { events: stageOffer() } });
    await openApp(page);
    await openMyWork(page);
    await waitForCommands(page);

    const card = page.getByTestId(`org-card-${DRI_DRAFT_ID}`);
    await expect(card).toHaveAttribute("data-card-type", "offer");
    await expect(card.getByTestId("org-card-offer")).toBeVisible();

    await waitForAnimations(page);
    await card.screenshot({ path: `${SHOTS}/05-offer.png` });

    await card.getByTestId("org-card-offer").click();
    await expect
      .poll(async () => (await signedOfKind(page, 50006)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50006);
    expect(signed[0]?.tags).toEqual(
      expect.arrayContaining([
        ["i", CHILD_ID],
        ["e", DRI_DRAFT_ID, "", "draft"],
      ]),
    );
  });

  test("06 — Accept on the offered child emits 50007", async ({ page }) => {
    await installMockBridge(page, { org: { events: stageAccept() } });
    await openApp(page);
    await openMyWork(page);
    await waitForCommands(page);

    const card = page.getByTestId(`org-card-${OFFER_CHILD_EVENT_ID}`);
    await expect(card).toHaveAttribute("data-card-type", "offer");
    await expect(card).toContainText("Electrics");

    await waitForAnimations(page);
    await card.screenshot({ path: `${SHOTS}/06-accept-child.png` });

    await card.getByTestId("org-card-accept").click();
    await expect
      .poll(async () => (await signedOfKind(page, 50007)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50007);
    expect(signed[0]?.tags).toEqual([["i", CHILD_ID]]);
  });

  test("07 — Work lists the project, and done card Mark done emits 50009", async ({
    page,
  }) => {
    await installMockBridge(page, { org: { events: stageChildAndDone() } });
    await openApp(page);
    await openWork(page);

    await expect(page.getByTestId(`org-work-row-${ROOT_ID}`)).toContainText(
      "Weekday hall",
    );
    await expect(page.getByTestId(`org-work-row-${CHILD_ID}`)).toHaveCount(0);

    await waitForAnimations(page);
    await page.getByTestId("org-work").screenshot({
      path: `${SHOTS}/07a-child-on-work.png`,
    });

    await openMyWork(page);
    await waitForCommands(page);

    const card = page.getByTestId(`org-card-${DONE_DRAFT_ID}`);
    await expect(card).toHaveAttribute("data-card-type", "done");
    await expect(card).toContainText("Lighting is wired and tested.");

    await waitForAnimations(page);
    await card.screenshot({ path: `${SHOTS}/07b-done-card.png` });

    await card.getByTestId("org-card-mark-done").click();
    await expect
      .poll(async () => (await signedOfKind(page, 50009)).length)
      .toBeGreaterThan(0);
    const signed = await signedOfKind(page, 50009);
    expect(signed[0]?.tags).toEqual(
      expect.arrayContaining([
        ["i", CHILD_ID],
        ["e", DONE_DRAFT_ID, "", "draft"],
      ]),
    );
  });
});
