import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import {
  createOverviewSeedEvents,
  HALL_PROJECT_ID,
  HARVEST_PROJECT_ID,
  OVERVIEW_ADD_SHAPER_PUBKEY,
  TALLY_WEEK,
} from "../../src/testing/orgOverviewFixture";
import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";

/**
 * Plan slice D-1 — Overview door (Phase 0 § Overview; Prototype map
 * § Overview / § Direction; Protocol §4.1 / §4.5 / §6.5):
 *
 *   - four direction cards from seeded 39100 (version + confirmer) and empty slots;
 *   - the direction form emits 50002 with base = current version;
 *   - Shapers card from 39103 with Add a Shaper / Step down / Change the rules;
 *   - Add a Shaper emits 50001 op=add;
 *   - who-holds-what from root 39101s;
 *   - tally card (Shapers only) from `{kinds:[50103], "#t":["tally"]}`.
 */

const SHOTS = "test-results/org-overview";

async function openOverview(page: Page) {
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
  await page.getByTestId("sidebar-org-overview").click();
  await expect(page.getByTestId("org-overview")).toBeVisible();
}

async function signedOfKind(page: Page, kind: number) {
  return page.evaluate((target) => {
    return (window.__BUZZ_E2E_SIGNED_EVENTS__ ?? []).filter(
      (event) => event.kind === target,
    );
  }, kind);
}

test.describe("Org Overview door (D-1)", () => {
  test("01 — empty slots say Not set yet.", async ({ page }) => {
    await installMockBridge(page);
    await openOverview(page);

    for (const slug of ["mission", "vision", "objectives", "strategy"]) {
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

    await expect(page.getByTestId("org-direction-meta-mission")).toContainText(
      "v3",
    );
    await expect(page.getByTestId("org-direction-meta-mission")).toContainText(
      "alice",
    );
    await expect(page.getByTestId("org-direction-meta-vision")).toContainText(
      "v1",
    );
    await expect(
      page.getByTestId("org-direction-meta-objectives"),
    ).toContainText("v2");
    await expect(page.getByTestId("org-direction-empty-strategy")).toHaveText(
      "Not set yet.",
    );

    await expect(page.getByTestId("org-shapers-members")).toBeVisible();
    await expect(page.getByTestId("org-shapers-rules")).toContainText(
      "direction: majority",
    );
    await expect(page.getByTestId("org-shapers-agent")).toContainText(
      "Hosted by the relay operator",
    );
    await expect(page.getByTestId("org-shapers-add")).toBeVisible();
    await expect(page.getByTestId("org-shapers-step-down")).toBeVisible();
    await expect(page.getByTestId("org-shapers-rules-change")).toBeVisible();

    await expect(page.getByTestId(`org-hold-${HALL_PROJECT_ID}`)).toContainText(
      "alice",
    );
    await expect(page.getByTestId(`org-hold-${HALL_PROJECT_ID}`)).toContainText(
      "Weekday hall",
    );
    await expect(
      page.getByTestId(`org-hold-${HARVEST_PROJECT_ID}`),
    ).toContainText("needs a DRI");

    await expect(page.getByTestId("org-tally-card")).toBeVisible();
    await expect(page.getByTestId("org-tally-week")).toContainText(TALLY_WEEK);
    await expect(page.getByTestId("org-tally-move-1")).toContainText(
      "opened 6",
    );

    await waitForAnimations(page);
    await page.getByTestId("org-overview").screenshot({
      path: `${SHOTS}/02-seeded-overview.png`,
    });
  });

  test("03 — direction form emits 50002 with base = current version", async ({
    page,
  }) => {
    await installMockBridge(page, {
      org: { events: createOverviewSeedEvents() },
    });
    await openOverview(page);

    await page.getByTestId("org-direction-propose-mission").click();
    await expect(page.getByRole("dialog", { name: /mission/i })).toBeVisible();
    await page
      .getByTestId("org-direction-body-mission")
      .fill("Keep the stall open every Saturday.");
    await page.getByTestId("org-direction-submit-mission").click();

    const signed = await signedOfKind(page, 50002);
    expect(signed.length).toBeGreaterThan(0);
    expect(signed[0]?.kind).toBe(50002);
    expect(signed[0]?.tags).toEqual(
      expect.arrayContaining([
        ["d", "mission"],
        ["base", "3"],
        ["vote", "agree"],
      ]),
    );
    expect(JSON.parse(signed[0]?.content ?? "{}")).toMatchObject({
      body: "Keep the stall open every Saturday.",
    });

    await waitForAnimations(page);
    await page.getByTestId("org-overview").screenshot({
      path: `${SHOTS}/03-direction-form.png`,
    });
  });

  test("04 — Add a Shaper emits 50001 op=add", async ({ page }) => {
    await installMockBridge(page, {
      org: { events: createOverviewSeedEvents() },
    });
    await openOverview(page);

    await page.getByTestId("org-shapers-add").press("Enter");
    await expect(
      page.getByRole("dialog", { name: "Add a Shaper" }),
    ).toBeVisible();
    await page
      .getByTestId("org-shaper-pubkey")
      .fill(OVERVIEW_ADD_SHAPER_PUBKEY);
    await page.getByTestId("org-shaper-add-submit").click();

    const signed = await signedOfKind(page, 50001);
    expect(signed.length).toBeGreaterThan(0);
    expect(signed[0]?.kind).toBe(50001);
    expect(signed[0]?.tags).toEqual(
      expect.arrayContaining([
        ["op", "add"],
        ["p", OVERVIEW_ADD_SHAPER_PUBKEY],
        ["vote", "agree"],
      ]),
    );

    await waitForAnimations(page);
    await page.getByTestId("org-shapers-card").screenshot({
      path: `${SHOTS}/04-add-shaper.png`,
    });
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

    await page.getByTestId("org-direction-propose-strategy").focus();
    await expect(
      page.getByTestId("org-direction-propose-strategy"),
    ).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog", { name: /strategy/i })).toBeVisible();
    await page.keyboard.press("Escape");

    await page.getByTestId("org-shapers-step-down").focus();
    await expect(page.getByTestId("org-shapers-step-down")).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog", { name: "Step down" })).toBeVisible();
    await page.keyboard.press("Escape");

    await page.getByTestId("org-shapers-rules-change").focus();
    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("dialog", { name: "Change the rules" }),
    ).toBeVisible();
    await page.keyboard.press("Escape");

    await page.getByTestId(`org-hold-${HALL_PROJECT_ID}`).focus();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("org-work-item")).toBeVisible();
  });
});
