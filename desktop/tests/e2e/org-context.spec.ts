import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { createContextGapEvents } from "../../src/testing/orgOverviewFixture";
import { installMockBridge } from "../helpers/bridge";

/**
 * G-3 — Overview shows done-when on an objective and a missing situation
 * on the Context line. The seed has no situation head.
 */

async function openOverview(page: Page) {
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
  await page.getByTestId("sidebar-org-overview").click();
  await expect(page.getByTestId("org-overview")).toBeVisible();
}

test.describe("Org context readiness (G-3)", () => {
  test("a seeded objective shows done when and situation is missing", async ({
    page,
  }) => {
    await installMockBridge(page, {
      org: { events: createContextGapEvents() },
    });
    await openOverview(page);

    await expect(page.getByTestId("org-direction-done-when-l_7f3a")).toHaveText(
      "Done when: the hall has hosted a weekday night",
    );
    await expect(page.getByTestId("org-direction-type-s_no")).toHaveText(
      "refusal",
    );
    await expect(page.getByTestId("org-context-situation")).toHaveText(
      "Situation missing",
    );
    await expect(page.getByTestId("org-direction-empty-situation")).toHaveText(
      "Not set yet.",
    );
  });
});
