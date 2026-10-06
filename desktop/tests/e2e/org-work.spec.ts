import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";
import {
  CHILD_ID,
  GRAND_ID,
  LINKED_GITHUB,
  OPEN_ROOT_ID,
  ROOT_ID,
  depth3WorkEvents,
  projectFilesFixture,
} from "./helpers/orgWork";

/**
 * Plan slice D-3 — Work door and the item page (Phase 0 § Work, § Project /
 * ticket page; Prototype map § Work, § Ticket; Protocol §6.5 / §4.2 / §4.7):
 *
 *   - the door lists projects only; tickets open from the project page;
 *   - item page: brief, holder, dates, breadcrumb, children with state chips;
 *   - trail stays hidden;
 *   - Open channel from home.channel when present, hidden when absent;
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

  test("01 — door lists projects and hides tickets", async ({ page }) => {
    await openWork(page);

    await expect(page.getByTestId("org-work-column-ongoing")).toContainText(
      "Weekday hall",
    );
    await expect(page.getByTestId("org-work-column-waiting")).toContainText(
      "Cold storage",
    );
    await expect(page.getByTestId(`org-work-row-holder-${ROOT_ID}`)).toHaveText(
      "npub1mock...",
    );
    await expect(page.getByTestId(`org-work-row-${ROOT_ID}`)).not.toContainText(
      "DRI:",
    );
    await expect(page.getByTestId(`org-work-row-due-${ROOT_ID}`)).toContainText(
      "review",
    );
    await expect(
      page.getByTestId(`org-work-row-review-${OPEN_ROOT_ID}`),
    ).toHaveText("review not set");
    await expect(page.getByTestId(`org-work-row-${CHILD_ID}`)).toHaveCount(0);
    await expect(page.getByTestId(`org-work-row-${GRAND_ID}`)).toHaveCount(0);
    await expect(
      page.getByTestId(`org-work-row-${OPEN_ROOT_ID}`),
    ).toContainText("Cold storage");
    await expect(
      page.getByTestId(`org-children-counts-${ROOT_ID}`),
    ).toHaveCount(0);
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
    await expect(page.getByTestId("org-item-facts")).toContainText("Review");
    await expect(page.getByTestId("org-item-due")).toBeVisible();
    await expect(page.getByTestId("org-modify-due")).toHaveCount(0);
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
    await expect(page.getByTestId(`org-work-row-${CHILD_ID}`)).toHaveCount(0);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();
    await page.getByTestId(`org-item-child-${CHILD_ID}`).click();

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

  test("04 — trail stays hidden on the item page", async ({ page }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();

    await expect(page.getByTestId("org-trail-row")).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "Trail" })).toHaveCount(0);
  });

  test("05 — Open channel follows home.channel and hides when missing", async ({
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
    await expect(
      page.getByRole("button", { name: "Open channel" }),
    ).toHaveCount(1);
    await expect(page.getByRole("button", { name: "Release" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Set due" })).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Modify due date" }),
    ).toHaveCount(0);
    await expect(page.getByTestId("org-item-title")).toHaveAccessibleName(
      "Weekday hall",
    );
  });

  test("10 — ticket holder sets a due date from the due card", async ({
    page,
  }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();
    await page.getByTestId(`org-item-child-${CHILD_ID}`).click();

    await expect(page.getByTestId("org-item-facts")).toContainText("Due");
    await expect(page.getByTestId("org-item-due")).toHaveText("Not set");
    await expect(page.getByRole("button", { name: "Release" })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "Trail" })).toHaveCount(0);

    const modify = page.getByRole("button", { name: "Modify due date" });
    await modify.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("org-set-due-date")).toBeVisible();
    await page.getByTestId("org-set-due-date").fill("2026-10-12");
    await page.getByRole("button", { name: "Save due date" }).click();

    const signed = await page.evaluate(() => {
      return (window.__BUZZ_E2E_SIGNED_EVENTS__ ?? []).filter(
        (event) => event.kind === 50011,
      );
    });
    expect(signed.length).toBeGreaterThan(0);
    expect(signed[0]?.tags?.[0]).toEqual(["i", CHILD_ID]);
    expect(signed[0]?.tags?.[1]?.[0]).toBe("due");
    expect(Number(signed[0]?.tags?.[1]?.[1])).toBe(
      Math.floor(Date.parse("2026-10-12T00:00:00Z") / 1000),
    );
  });
});

test.describe("Org project files (H-2)", () => {
  test.beforeEach(async ({ page }) => {
    const { events, announcement } = projectFilesFixture();
    await page.addInitScript((extra) => {
      (
        window as Window & {
          __BUZZ_E2E_EXTRA_PROJECT_EVENTS__?: unknown[];
        }
      ).__BUZZ_E2E_EXTRA_PROJECT_EVENTS__ = [extra];
    }, announcement);
    await installMockBridge(page, { org: { events } });
  });

  test("lists context files and the linked GitHub repository", async ({
    page,
  }) => {
    await openWork(page);
    await page.getByTestId(`org-work-row-${ROOT_ID}`).click();

    const files = page.getByTestId("org-project-files");
    await expect(files).toContainText("context/README.md");
    await expect(files).toContainText("context/decisions.md");
    await expect(files).toContainText("context/links.md");
    await expect(files).not.toContainText("context/drafts");
    const link = page.getByRole("link", { name: LINKED_GITHUB });
    await expect(link).toBeVisible();
    await link.focus();
    await expect(link).toBeFocused();
  });
});
