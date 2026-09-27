import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";

/**
 * Plan slice D-0 — feature gate and skeleton (Design § Surfaces; Phase 0
 * § The app; Prototype map § Routes; Protocol §6.5 / §4.8):
 *
 *   - the `org` gate hides the sidebar group when off;
 *   - `/org`, `/org/work`, `/org/work/$itemId`, `/org/my-work` render their
 *     empty states;
 *   - a command hook produces the C-1 kind and tags (captured `sign_event`).
 */

const SHOTS = "test-results/org-skeleton";
const ITEM_ID = "11111111-1111-4111-8111-111111111111";
const DRAFT_ID = "ab".repeat(32);

async function openApp(page: Page) {
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
}

test.describe("Org doors — feature gate and skeleton (D-0)", () => {
  test("01 — gate off hides the sidebar group", async ({ page }) => {
    await installMockBridge(page, undefined, { seedPreviewFeatures: false });
    await openApp(page);

    await expect(page.getByTestId("sidebar-primary-menu")).toBeVisible();
    await expect(page.getByTestId("sidebar-org-group")).toHaveCount(0);
    await expect(page.getByTestId("sidebar-org-overview")).toHaveCount(0);
    await expect(page.getByTestId("sidebar-org-work")).toHaveCount(0);
    await expect(page.getByTestId("sidebar-org-my-work")).toHaveCount(0);

    await waitForAnimations(page);
    await page.getByTestId("sidebar-primary-menu").screenshot({
      path: `${SHOTS}/01-gate-off-hides-group.png`,
    });
  });

  test("02 — Overview renders Not set yet.", async ({ page }) => {
    await installMockBridge(page);
    await openApp(page);

    await expect(page.getByTestId("sidebar-org-group")).toBeVisible();
    await page.getByTestId("sidebar-org-overview").click();
    await expect(page.getByTestId("org-overview")).toBeVisible();
    await expect(page.getByTestId("org-direction-empty-mission")).toHaveText(
      "Not set yet.",
    );
    await expect(page.getByTestId("chat-title")).toHaveText("Overview");

    await waitForAnimations(page);
    await page.getByTestId("sidebar-primary-menu").screenshot({
      path: `${SHOTS}/02-gate-on-shows-group.png`,
    });
    await page.getByTestId("org-overview").screenshot({
      path: `${SHOTS}/02-overview-empty.png`,
    });
  });

  test("03 — Work renders Nothing needs you.", async ({ page }) => {
    await installMockBridge(page);
    await openApp(page);

    await page.getByTestId("sidebar-org-work").click();
    await expect(page.getByTestId("org-work-empty")).toHaveText(
      "Nothing needs you.",
    );
    await expect(page.getByTestId("chat-title")).toHaveText("Work");

    await waitForAnimations(page);
    await page.getByTestId("org-work").screenshot({
      path: `${SHOTS}/03-work-empty.png`,
    });
  });

  test("04 — item page renders Not set yet.", async ({ page }) => {
    await installMockBridge(page);
    await openApp(page);

    await page.goto(`/#/org/work/${ITEM_ID}`);
    await expect(page.getByTestId("org-work-item-empty")).toHaveText(
      "Not set yet.",
    );

    await waitForAnimations(page);
    await page.getByTestId("org-work-item").screenshot({
      path: `${SHOTS}/04-work-item-empty.png`,
    });
  });

  test("05 — My Work renders Nothing needs you.", async ({ page }) => {
    await installMockBridge(page);
    await openApp(page);

    await page.getByTestId("sidebar-org-my-work").click();
    await expect(page.getByTestId("org-my-work-empty")).toHaveText(
      "Nothing needs you.",
    );
    await expect(page.getByTestId("chat-title")).toHaveText("My Work");

    await waitForAnimations(page);
    await page.getByTestId("org-my-work").screenshot({
      path: `${SHOTS}/05-my-work-empty.png`,
    });
  });

  test("06 — a command hook publishes the C-1 kind and draft e tag", async ({
    page,
  }) => {
    await installMockBridge(page);
    await openApp(page);
    await page.getByTestId("sidebar-org-overview").click();
    await expect(page.getByTestId("org-overview")).toBeVisible();

    await expect
      .poll(async () =>
        page.evaluate(() => Boolean(window.__BUZZ_E2E_ORG_COMMANDS__)),
      )
      .toBe(true);

    await page.evaluate(
      async ({ draftId, itemId }) => {
        const commands = window.__BUZZ_E2E_ORG_COMMANDS__;
        if (!commands) throw new Error("org command hook was not installed");
        const command = commands.buildIoDone({ item: itemId, draftId });
        await commands.publish(command);
      },
      { draftId: DRAFT_ID, itemId: ITEM_ID },
    );

    const signed = await page.evaluate(() => {
      return (window.__BUZZ_E2E_SIGNED_EVENTS__ ?? []).filter(
        (event) => event.kind === 50009,
      );
    });
    expect(signed.length).toBeGreaterThan(0);
    expect(signed[0]?.kind).toBe(50009);
    expect(signed[0]?.tags).toEqual([
      ["i", ITEM_ID],
      ["e", DRAFT_ID, "", "draft"],
    ]);
    expect(signed[0]?.content).toBe("{}");
  });
});
