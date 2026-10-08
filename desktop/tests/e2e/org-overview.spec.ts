import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import {
  createOverviewSeedEvents,
  HALL_PROJECT_ID,
  HARVEST_PROJECT_ID,
} from "../../src/testing/orgOverviewFixture";
import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";

/**
 * Plan slice D-1 — Overview door (Phase 0 § Overview; Prototype map
 * § Overview / § Direction; Protocol §4.1 / §4.5 / §6.5):
 *
 *   - four direction cards from seeded 39100 (version + confirmer) and empty slots;
 *   - the direction form emits 50002 with base = current version;
 *   - Shapers card lists each Shaper and opens their profile;
 *   - Personal holds My work, My drafts, and My profile;
 *   - who-holds-what from root 39101s.
 * The tally card stays off this door for now.
 */

const SHOTS = "test-results/org-overview";

async function openOverview(page: Page) {
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
  await page.getByTestId("sidebar-org-overview").click();
  await expect(page.getByTestId("org-overview")).toBeVisible();
}

const VIEWER_PUBKEY = "deadbeef".repeat(8);

test.describe("Org Overview door (D-1)", () => {
  test("01 — empty slots say Not set yet.", async ({ page }) => {
    await installMockBridge(page);
    await openOverview(page);

    for (const slug of [
      "mission",
      "vision",
      "situation",
      "objectives",
      "strategy",
    ]) {
      await expect(
        page.getByTestId(`org-direction-card-${slug}`),
      ).toBeVisible();
      await expect(page.getByTestId(`org-direction-empty-${slug}`)).toHaveText(
        "Not set yet.",
      );
    }
    await expect(page.getByTestId("org-tally-card")).toHaveCount(0);

    await waitForAnimations(page);
    await page.getByTestId("org-overview").screenshot({
      path: `${SHOTS}/01-empty-slots.png`,
    });
  });

  test("02 — seeded 39100 / 39103 / 39101 / 50103 render", async ({ page }) => {
    await installMockBridge(page, {
      org: { events: createOverviewSeedEvents() },
    });
    await openOverview(page);

    await expect(page.getByTestId("org-direction-card-mission")).toContainText(
      "Feed the Saturday market every week.",
    );
    await expect(page.getByTestId("org-direction-open-mission")).toHaveText(
      "Read more",
    );
    await expect(page.getByTestId("org-direction-propose-mission")).toHaveCount(
      0,
    );
    await expect(page.getByTestId("org-direction-empty-strategy")).toHaveText(
      "Not set yet.",
    );
    const situation = page.getByTestId("org-direction-card-situation");
    await expect(situation).toContainText("where we stand today");
    await expect(situation).toContainText("never run a weekday night");
    const [visionBox, situationBox, objectivesBox] = await Promise.all(
      ["vision", "situation", "objectives"].map((slug) =>
        page.getByTestId(`org-direction-card-${slug}`).boundingBox(),
      ),
    );
    expect(situationBox?.y).toBeGreaterThan(visionBox?.y ?? 0);
    expect(objectivesBox?.y).toBeGreaterThan(situationBox?.y ?? 0);

    await expect(page.getByTestId("org-shapers-members")).toBeVisible();
    await expect(page.getByTestId(`org-shaper-${VIEWER_PUBKEY}`)).toBeVisible();
    await expect(page.getByTestId("org-shapers-majority")).toContainText(
      "A majority of Shapers is needed to decide anything.",
    );
    await expect(page.getByTestId("org-shapers-rules")).toHaveCount(0);
    await expect(page.getByTestId("org-shapers-add")).toHaveCount(0);
    await expect(page.getByTestId("sidebar-personal-group-label")).toHaveText(
      "Personal",
    );
    await expect(page.getByTestId("sidebar-org-my-work")).toBeVisible();
    await expect(page.getByTestId("sidebar-org-my-drafts")).toBeVisible();
    await expect(page.getByTestId("sidebar-org-my-profile")).toBeVisible();

    await expect(page.getByTestId(`org-hold-${HALL_PROJECT_ID}`)).toContainText(
      "alice",
    );
    await expect(page.getByTestId(`org-hold-${HALL_PROJECT_ID}`)).toContainText(
      "Weekday hall",
    );
    await expect(
      page.getByTestId(`org-hold-${HARVEST_PROJECT_ID}`),
    ).toContainText("needs a DRI");
    await expect(page.getByTestId("org-tally-card")).toHaveCount(0);

    await waitForAnimations(page);
    await page.getByTestId("org-overview").screenshot({
      path: `${SHOTS}/02-seeded-overview.png`,
    });
  });

  test("03 — direction cards offer Read more and no propose button", async ({
    page,
  }) => {
    await installMockBridge(page, {
      org: { events: createOverviewSeedEvents() },
    });
    await openOverview(page);

    await expect(
      page.getByRole("button", { name: "Propose a version" }),
    ).toHaveCount(0);
    await expect(page.getByTestId("org-direction-open-mission")).toHaveText(
      "Read more",
    );
    await page.getByTestId("org-direction-open-mission").click();
    await expect(page.getByTestId("org-direction")).toBeVisible();
  });

  test("04 — a Shaper opens their profile", async ({ page }) => {
    await installMockBridge(page, {
      org: { events: createOverviewSeedEvents() },
    });
    await openOverview(page);

    await page.getByTestId(`org-shaper-${VIEWER_PUBKEY}`).click();
    await expect(page.getByTestId("org-member-profile")).toBeVisible();
    await expect(page.getByTestId("chat-title")).toHaveText("My profile");
    await expect(page.getByTestId("org-profile-projects")).toBeVisible();
    await expect(page.getByTestId("org-profile-tickets")).toBeVisible();
    await expect(page.getByTestId("org-profile-earlier")).toBeVisible();
    await expect(page.getByTestId("org-profile-activity")).toBeVisible();

    await waitForAnimations(page);
    await page.getByTestId("org-member-profile").screenshot({
      path: `${SHOTS}/04-my-profile.png`,
    });

    await page.getByTestId("sidebar-org-overview").click();
    await expect(page.getByTestId("org-overview")).toBeVisible();
    await page.getByTestId("sidebar-org-my-profile").click();
    await expect(page.getByTestId("chat-title")).toHaveText("My profile");
  });

  test("05 — direction page uses the §6.5 history REQ", async ({ page }) => {
    await installMockBridge(page, {
      org: { events: createOverviewSeedEvents() },
    });
    await openOverview(page);

    await page.getByTestId("org-direction-open-mission").click();
    await expect(page.getByTestId("org-direction")).toBeVisible();
    await expect(page.getByTestId("org-direction-body")).toContainText(
      "Feed the Saturday market every week.",
    );
    await expect(page.getByTestId("org-direction-history")).toContainText("v3");
    await expect(page.getByTestId("org-direction-history")).toContainText(
      "alice",
    );

    await page.getByTestId("org-direction-back").click();
    await expect(page.getByTestId("org-overview")).toBeVisible();

    await waitForAnimations(page);
    await page.goto("/#/org/direction/mission");
    await expect(page.getByTestId("org-direction-history")).toBeVisible();
    await waitForAnimations(page);
    await page.getByTestId("org-direction").screenshot({
      path: `${SHOTS}/05-direction-page.png`,
    });
  });

  test("06 — keyboard reaches every Overview tap", async ({ page }) => {
    await installMockBridge(page, {
      org: { events: createOverviewSeedEvents() },
    });
    await openOverview(page);

    await page.getByTestId("org-direction-open-strategy").focus();
    await expect(page.getByTestId("org-direction-open-strategy")).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("org-direction")).toBeVisible();
    await page.getByTestId("org-direction-back").click();
    await expect(page.getByTestId("org-overview")).toBeVisible();

    const shaper = page.getByTestId(`org-shaper-${VIEWER_PUBKEY}`);
    await shaper.focus();
    await expect(shaper).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("org-member-profile")).toBeVisible();
    await page.getByTestId("sidebar-org-overview").click();
    await expect(page.getByTestId("org-overview")).toBeVisible();

    await page.getByTestId(`org-hold-${HALL_PROJECT_ID}`).focus();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("org-work-item")).toBeVisible();
  });
});
