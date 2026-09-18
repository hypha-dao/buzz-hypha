import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import {
  ORG_AGENT_DISPLAY_NAME,
  ORG_AGENT_DM_CHANNEL_ID,
  ORG_AGENT_PUBKEY,
} from "../../src/testing/orgAgentFixture";
import { waitForAnimations } from "../helpers/animations";
import { installMockBridge } from "../helpers/bridge";

/**
 * Plan slice D-5 — Agents door, Hypha defaults (AGENTS.md § Hypha fork;
 * Design § Where it runs — Members' own agents stay; § Work sync):
 *
 *   - the door starts empty (no seeded persona, no running agent);
 *   - the Work sync template is present but disabled until wave 6;
 *   - the org agent never appears in the door;
 *   - DMs list the org agent first.
 */

const SHOTS = "test-results/org-agent-defaults";

// Mock-mode current-user pubkey and relay (see e2eBridge DEFAULT_MOCK_IDENTITY
// / DEFAULT_RELAY_WS_URL); sort preferences live under this relay-scoped key.
const MOCK_PUBKEY = "deadbeef".repeat(8);
const MOCK_RELAY_ENCODED = encodeURIComponent("ws://localhost:3000");
const SORT_STORAGE_KEY = `buzz-channel-sort.v1:${MOCK_PUBKEY}:${MOCK_RELAY_ENCODED}`;

// A member's own managed agent, unrelated to the org.
const SCOUT_PUBKEY =
  "6f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0";

async function openApp(page: Page) {
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
}

async function openAgentsDoor(page: Page) {
  await page.getByTestId("open-agents-view").click();
  await expect(page.getByTestId("unified-agents-groups")).toBeVisible({
    timeout: 15_000,
  });
}

/** Sidebar DM rows in display order: `[channelId, resolved label]`. */
function dmRows(page: Page) {
  return page
    .getByTestId("dm-list")
    .locator("[data-channel-id]")
    .evaluateAll((nodes) =>
      nodes.map((node) => [
        node.getAttribute("data-channel-id") ?? "",
        node.querySelector("[data-sidebar-row-label]")?.textContent?.trim() ??
          "",
      ]),
    );
}

test.describe("Agents door — Hypha defaults (D-5)", () => {
  test("01 — the door starts empty, with the Work sync template present but disabled", async ({
    page,
  }) => {
    // Default mock store: no active persona, no managed agent — the state a
    // fresh Hypha install is in (the Tauri store seeds nothing).
    await installMockBridge(page);
    await openApp(page);
    await openAgentsDoor(page);

    const groups = page.getByTestId("unified-agents-groups");
    await expect(groups.getByTestId("new-agent-card")).toBeVisible();
    await expect(
      groups.locator('[data-testid^="persona-agent-row-"]'),
    ).toHaveCount(0);
    await expect(groups.locator('[data-testid^="managed-agent-"]')).toHaveCount(
      0,
    );
    await expect(groups.getByTestId("agents-door-empty-state")).toContainText(
      "No agents yet.",
    );

    // The one template the Hypha desktop offers: present, named, disabled.
    const templates = groups.getByTestId("agent-templates-group");
    await expect(templates).toContainText("Templates");
    const card = templates.getByTestId("work-sync-template-card");
    await expect(card).toBeVisible();
    await expect(card).toContainText("Work sync");
    await expect(card).toContainText("Coming soon");
    const button = card.getByRole("button", { name: "Work sync template" });
    await expect(button).toBeDisabled();
    await expect(button).toHaveAccessibleDescription(/Not available yet/);
    // Keyboard users still reach it and hear why it does nothing.
    await button.focus();
    await expect(button).toBeFocused();
    await page.keyboard.press("Enter");
    await button.click({ force: true });
    await expect(page.getByTestId("community-catalog-dialog")).toHaveCount(0);
    await expect(page.getByRole("dialog")).toHaveCount(0);

    // Tall enough that the whole door (through the Templates group) is in view.
    await page.setViewportSize({ width: 1280, height: 1100 });
    await waitForAnimations(page);
    await groups.screenshot({ path: `${SHOTS}/01-door-empty-state.png` });
  });

  test("02 — the org agent never appears in the door, even as a local managed agent", async ({
    page,
  }) => {
    // A Shapers decision can point `39103.agent` at a key a member runs
    // locally; the door must still not list it (Design § Where it runs).
    await installMockBridge(page, {
      org: { seedAgentDm: false },
      managedAgents: [
        { pubkey: ORG_AGENT_PUBKEY, name: ORG_AGENT_DISPLAY_NAME },
        { pubkey: SCOUT_PUBKEY, name: "Scout" },
      ],
    });
    await openApp(page);
    await openAgentsDoor(page);

    const groups = page.getByTestId("unified-agents-groups");
    await expect(
      groups.getByTestId(`managed-agent-${SCOUT_PUBKEY}`),
    ).toBeVisible();
    await expect(groups).toContainText("Custom agents");
    await expect(groups).toContainText("(1)");
    await expect(
      groups.getByTestId(`managed-agent-${ORG_AGENT_PUBKEY}`),
    ).toHaveCount(0);
    await expect(groups).not.toContainText(ORG_AGENT_DISPLAY_NAME);

    await page.setViewportSize({ width: 1280, height: 1100 });
    await waitForAnimations(page);
    await groups.screenshot({ path: `${SHOTS}/02-door-hides-org-agent.png` });
  });

  test("03 — the same managed agent is listed when 39103 does not name it", async ({
    page,
  }) => {
    // Falsifiability check for 02: the only difference is `mock.org`, so the
    // hidden card in 02 is hidden by the `39103.agent` read, not by the seed.
    await installMockBridge(page, {
      managedAgents: [
        { pubkey: ORG_AGENT_PUBKEY, name: ORG_AGENT_DISPLAY_NAME },
      ],
    });
    await openApp(page);
    await openAgentsDoor(page);

    await expect(
      page
        .getByTestId("unified-agents-groups")
        .getByTestId(`managed-agent-${ORG_AGENT_PUBKEY}`),
    ).toBeVisible();
  });

  test("04 — DMs list the org agent first under the A–Z sort", async ({
    page,
  }) => {
    await installMockBridge(page, { org: {} });
    await openApp(page);

    const dmList = page.getByTestId("dm-list");
    await expect(dmList).toBeVisible();
    await expect(dmList).toContainText(ORG_AGENT_DISPLAY_NAME);
    // Wait for every DM label to resolve before reading the order.
    await expect(dmList).toContainText("charlie");

    const rows = await dmRows(page);
    expect(rows[0]).toEqual([ORG_AGENT_DM_CHANNEL_ID, ORG_AGENT_DISPLAY_NAME]);
    // "Org agent" sorts after every other label alphabetically, so a top
    // position can only come from the pin.
    const labels = rows.map(([, label]) => label);
    expect([...labels.slice(1)].sort((a, b) => a.localeCompare(b))).toEqual(
      labels.slice(1),
    );
    expect(labels.slice(1)).not.toContain(ORG_AGENT_DISPLAY_NAME);

    await waitForAnimations(page);
    await dmList.screenshot({
      path: `${SHOTS}/04-dm-list-org-agent-first.png`,
    });
  });

  test("05 — the pin also wins under the Recent sort, where a quiet DM would sink", async ({
    page,
  }) => {
    await page.addInitScript(
      ({ key }) => {
        window.localStorage.setItem(
          key,
          JSON.stringify({ version: 1, groups: { dms: "recent" } }),
        );
      },
      { key: SORT_STORAGE_KEY },
    );
    await installMockBridge(page, { org: {} });
    await openApp(page);

    const dmList = page.getByTestId("dm-list");
    await expect(dmList).toContainText(ORG_AGENT_DISPLAY_NAME);
    await expect(dmList).toContainText("charlie");

    const rows = await dmRows(page);
    expect(rows[0]?.[0]).toBe(ORG_AGENT_DM_CHANNEL_ID);
    expect(rows.length).toBeGreaterThan(1);
  });

  test("06 — without a 39103 the DM list keeps its plain order", async ({
    page,
  }) => {
    // Falsifiability check for 04/05: the community was never bootstrapped, so
    // there is no org agent to pin and the list is the resolved-label order.
    await installMockBridge(page);
    await openApp(page);

    const dmList = page.getByTestId("dm-list");
    await expect(dmList).toContainText("charlie");
    const labels = (await dmRows(page)).map(([, label]) => label);
    expect(labels).not.toContain(ORG_AGENT_DISPLAY_NAME);
    expect([...labels].sort((a, b) => a.localeCompare(b))).toEqual(labels);
  });
});
