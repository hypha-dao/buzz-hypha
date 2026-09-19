import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";
import {
  CHILD_ID,
  GRAND_ID,
  OPEN_ROOT_ID,
  ROOT_ID,
  depth3WorkEvents,
} from "./helpers/orgWork";

/**
 * Plan slice D-3 — Work door and the item page (Phase 0 § Work, § Project /
 * ticket page; Prototype map § Work, § Ticket; Protocol §6.5 / §4.2 / §4.7):
 *
 *   - tree renders depth 3 from fixtures (roots + one level on the door;
 *     deeper on the item page);
 *   - item page: brief, holder, dates, breadcrumb, children with state chips;
 *   - trail from `{kinds:[50001–50021], "#i":[id]}` newest first;
 *   - Open room from home.channel when present, hidden when absent;
 *   - health card from the latest 50101; rows on hover;
 *   - Mark done emits 50009 on the same sign_event capture as D-0;
 *   - a11y (rule 7) + keyboard (rule 8).
 */

const SHOTS = "test-results/org-work";

async function openWork(page: Page) {
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
  await page.getByTestId("sidebar-org-work").click();
  await expect(page.getByTestId("org-work-tree")).toBeVisible();
}

test.describe("Org Work door and item page (D-3)", () => {
  test.beforeEach(async ({ page }) => {
    await installMockBridge(page, { org: { events: depth3WorkEvents() } });
  });

  test("01 — tree renders roots plus one level from fixtures", async ({
    page,
  }) => {
    await openWork(page);

    await expect(page.getByTestId(`org-work-row-${ROOT_ID}`)).toContainText(
      "Weekday hall",
    );
    await expect(page.getByTestId(`org-work-row-${CHILD_ID}`)).toContainText(
      "Electrics",
    );
    await expect(page.getByTestId(`org-work-row-${GRAND_ID}`)).toHaveCount(0);
    await expect(
      page.getByTestId(`org-work-row-${OPEN_ROOT_ID}`),
    ).toContainText("Cold storage");
    await expect(page.getByTestId(`org-children-counts-${ROOT_ID}`)).toHaveText(
      "0 open · 1 offered · 1 accepted · 3 done",
    );
    await expect(
      page.getByTestId(`org-work-row-${ROOT_ID}`).getByTestId("org-state-chip"),
    ).toHaveText("in progress");
    await expect(
      page
        .getByTestId(`org-work-row-${OPEN_ROOT_ID}`)
        .getByTestId("org-state-chip"),
    ).toHaveText("needs a DRI");

    await waitForAnimations(page);
    await page.getByTestId("org-work").screenshot({
      path: `${SHOTS}/01-work-tree.png`,
    });
  });

  test("02 — item page shows brief, holder, dates, breadcrumb, children", async ({
    page,
  }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();

    await expect(page.getByTestId("org-item-title")).toHaveText("Weekday hall");
    await expect(page.getByTestId("org-item-brief")).toContainText(
      "Book the hall for weekday evenings.",
    );
    await expect(page.getByTestId("org-holder-name")).toHaveText("You");
    await expect(page.getByTestId("org-item-due")).toBeVisible();
    await expect(page.getByTestId("org-item-approved")).toBeVisible();
    await expect(page.getByTestId("org-item-breadcrumb")).toContainText("Work");
    await expect(page.getByTestId(`org-item-child-${CHILD_ID}`)).toContainText(
      "Electrics",
    );
    await expect(
      page
        .getByTestId(`org-item-child-${CHILD_ID}`)
        .getByTestId("org-state-chip"),
    ).toHaveText("in progress");
    await expect(page.getByTestId("org-item-children-counts")).toHaveText(
      "0 open · 1 offered · 1 accepted · 3 done",
    );

    await waitForAnimations(page);
    await page.getByTestId("org-work-item").screenshot({
      path: `${SHOTS}/02-item-page.png`,
    });
  });

  test("03 — deeper level is on the child page, not the door", async ({
    page,
  }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${CHILD_ID}`).click();

    await expect(page.getByTestId("org-item-title")).toHaveText("Electrics");
    await expect(page.getByTestId("org-item-breadcrumb-parent")).toHaveText(
      "Weekday hall",
    );
    await expect(page.getByTestId(`org-item-child-${GRAND_ID}`)).toContainText(
      "Rota",
    );
    await expect(
      page
        .getByTestId(`org-item-child-${GRAND_ID}`)
        .getByTestId("org-state-chip"),
    ).toHaveText("waiting on a yes");
  });

  test("04 — trail is #i commands newest first", async ({ page }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();

    const labels = page.getByTestId("org-trail-row");
    await expect(labels).toHaveCount(3);
    await expect(labels.nth(0)).toContainText("io_set_due");
    await expect(labels.nth(1)).toContainText("io_accept");
    await expect(labels.nth(2)).toContainText("io_offer");
    await expect(labels.nth(0)).toHaveAttribute("data-kind", "50011");
  });

  test("05 — Open room follows home.channel and hides when missing", async ({
    page,
  }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();
    await expect(page.getByTestId("org-open-room")).toBeVisible();
    await page.getByTestId("org-open-room").click();
    await expect(page.getByTestId("chat-title")).toHaveText("general");

    await page.goto(`/#/org/work/${OPEN_ROOT_ID}`);
    await expect(page.getByTestId("org-item-title")).toHaveText("Cold storage");
    await expect(page.getByTestId("org-open-room")).toHaveCount(0);
  });

  test("06 — health card is the latest 50101; rows on hover and focus", async ({
    page,
  }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();

    await expect(page.getByTestId("org-health-card")).toBeVisible();
    await expect(page.getByTestId("org-health-band")).toHaveText("wobbly");
    const sentence = page.getByTestId("org-health-sentence");
    await expect(sentence).toHaveText("Two pieces are past their date.");

    await sentence.hover();
    await expect(page.getByTestId("org-health-rows")).toContainText(
      "row-overdue-1",
    );
    await expect(page.getByTestId("org-health-rows")).toContainText(
      "row-overdue-2",
    );

    await page.keyboard.press("Escape");
    await sentence.focus();
    await expect(page.getByTestId("org-health-rows")).toBeVisible();

    await waitForAnimations(page);
    await page.getByTestId("org-health-card").screenshot({
      path: `${SHOTS}/06-health-card.png`,
    });
  });

  test("07 — Mark done emits 50009 on the sign_event capture", async ({
    page,
  }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();
    await expect(page.getByTestId("org-mark-done")).toBeVisible();
    await page.getByTestId("org-mark-done").click();

    const signed = await page.evaluate(() => {
      return (window.__BUZZ_E2E_SIGNED_EVENTS__ ?? []).filter(
        (event) => event.kind === 50009,
      );
    });
    expect(signed.length).toBeGreaterThan(0);
    expect(signed[0]?.kind).toBe(50009);
    expect(signed[0]?.tags).toEqual([["i", ROOT_ID]]);
    expect(signed[0]?.content).toBe("{}");
  });

  test("08 — keyboard path opens a row and marks done (rule 8)", async ({
    page,
  }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).focus();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("org-item-title")).toHaveText("Weekday hall");

    await page.getByTestId("org-mark-done").focus();
    await page.keyboard.press("Enter");
    const signed = await page.evaluate(() => {
      return (window.__BUZZ_E2E_SIGNED_EVENTS__ ?? []).filter(
        (event) => event.kind === 50009,
      );
    });
    expect(signed[0]?.tags).toEqual([["i", ROOT_ID]]);
  });

  test("09 — one accessible name per action (rule 7)", async ({ page }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();

    await expect(
      page.getByRole("navigation", { name: "Breadcrumb" }),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Mark done" })).toHaveCount(
      1,
    );
    await expect(page.getByRole("button", { name: "Open room" })).toHaveCount(
      1,
    );
    await expect(page.getByRole("button", { name: "Release" })).toHaveCount(1);
    await expect(page.getByRole("button", { name: "Set due" })).toHaveCount(1);
    await expect(page.getByTestId("org-item-title")).toHaveAccessibleName(
      "Weekday hall",
    );
  });
});
