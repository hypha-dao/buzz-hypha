import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { waitForAnimations } from "../helpers/animations";
import { installMockBridge, TEST_IDENTITIES } from "../helpers/bridge";

/**
 * Plan slice D-4 — About & skills on the existing profile (Phase 0 § The
 * principle, My Profile row; Protocol §4.7a / §4.8 `50021` / §7):
 *
 *   - Save emits `50021` with the full profile (skill chips, `open_limit`);
 *   - another member's profile has no form (read-only);
 *   - keyboard path for Add / Save (rule 8); one label owner per control
 *     (rule 7).
 */

const SHOTS = "test-results/org-about-skills";
const SELF_PUBKEY = "deadbeef".repeat(8);
const ABOUT =
  "I run the Tuesday kitchen and can write a grant if someone checks my numbers.";

async function openApp(page: Page) {
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await expect(page.getByTestId("chat-title")).toHaveText("general");
}

async function openMemberProfile(page: Page, pubkey: string) {
  const hash = new URL(page.url()).hash.replace(/^#/, "");
  const path = hash.split("?")[0] || "/";
  await page.goto(`/#${path}?profile=${pubkey}`);
  await expect(page.getByTestId("user-profile-panel")).toBeVisible();
  await expect(page.getByTestId("org-about-skills")).toBeVisible();
}

async function signedProfileSets(page: Page) {
  return page.evaluate(() =>
    (window.__BUZZ_E2E_SIGNED_EVENTS__ ?? []).filter(
      (event) => event.kind === 50021,
    ),
  );
}

test.describe("About & skills on the existing profile (D-4)", () => {
  test("01 — Save emits 50021 with the full profile", async ({ page }) => {
    await installMockBridge(page);
    await openApp(page);
    await openMemberProfile(page, SELF_PUBKEY);

    const form = page.getByTestId("org-about-skills-form");
    await expect(form).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "About & skills" }),
    ).toBeVisible();
    await expect(page.getByLabel("About", { exact: true })).toBeVisible();
    await expect(page.getByLabel("Skills", { exact: true })).toBeVisible();
    await expect(page.getByLabel("Open limit", { exact: true })).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Save About & skills" }),
    ).toBeVisible();

    await page.getByTestId("org-about-skills-about").fill(ABOUT);
    await page
      .getByTestId("org-about-skills-skill-input")
      .fill("grant writing");
    await page.getByTestId("org-about-skills-skill-input").press("Enter");
    await expect(page.getByTestId("org-about-skills-chips")).toContainText(
      "grant writing",
    );
    await page
      .getByTestId("org-about-skills-skill-input")
      .fill("hosting events");
    await page.getByTestId("org-about-skills-skill-add").click();
    await expect(page.getByTestId("org-about-skills-chips")).toContainText(
      "hosting events",
    );

    await page.getByTestId("org-about-skills-open-limit").fill("3");
    await page.getByTestId("org-about-skills-open-limit").press("Enter");

    await expect
      .poll(async () => (await signedProfileSets(page)).length)
      .toBe(1);
    const signed = await signedProfileSets(page);
    expect(signed[0]?.kind).toBe(50021);
    expect(signed[0]?.tags).toEqual([]);
    expect(signed[0]?.content).toBe(
      JSON.stringify({
        about: ABOUT,
        skills: ["grant writing", "hosting events"],
        open_limit: 3,
      }),
    );

    await waitForAnimations(page);
    await page.getByTestId("org-about-skills").screenshot({
      path: `${SHOTS}/01-save-full-profile.png`,
    });
  });

  test("02 — another member's profile has no form", async ({ page }) => {
    await installMockBridge(page);
    await openApp(page);
    await openMemberProfile(page, TEST_IDENTITIES.alice.pubkey);

    await expect(page.getByTestId("org-about-skills")).toBeVisible();
    await expect(page.getByTestId("org-about-skills-readonly")).toBeVisible();
    await expect(page.getByTestId("org-about-skills-form")).toHaveCount(0);
    await expect(page.getByTestId("org-about-skills-save")).toHaveCount(0);
    await expect(page.getByTestId("org-about-skills-skill-input")).toHaveCount(
      0,
    );
    await expect(page.getByTestId("org-about-skills-about")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Save About & skills" }),
    ).toHaveCount(0);

    await waitForAnimations(page);
    await page.getByTestId("org-about-skills").screenshot({
      path: `${SHOTS}/02-other-member-readonly.png`,
    });
  });

  test("03 — keyboard path adds a chip and saves", async ({ page }) => {
    await installMockBridge(page);
    await openApp(page);
    await openMemberProfile(page, SELF_PUBKEY);

    const about = page.getByTestId("org-about-skills-about");
    await about.focus();
    await expect(about).toBeFocused();
    await about.fill("I wire halls.");

    const skill = page.getByTestId("org-about-skills-skill-input");
    await skill.focus();
    await skill.fill("electrics");
    await skill.press("Enter");
    await expect(
      page.getByRole("list", { name: "Added skills" }),
    ).toContainText("electrics");

    await page.getByTestId("org-about-skills-remove-electrics").focus();
    await expect(
      page.getByTestId("org-about-skills-remove-electrics"),
    ).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("org-about-skills-chips")).toHaveCount(0);

    await skill.focus();
    await skill.fill("electrics");
    await skill.press("Enter");

    const limit = page.getByTestId("org-about-skills-open-limit");
    await limit.focus();
    await limit.fill("2");
    await page.getByRole("button", { name: "Save About & skills" }).focus();
    await expect(
      page.getByRole("button", { name: "Save About & skills" }),
    ).toBeFocused();
    await page.keyboard.press("Enter");

    await expect
      .poll(async () => (await signedProfileSets(page)).length)
      .toBe(1);
    const signed = await signedProfileSets(page);
    expect(signed[0]?.content).toBe(
      JSON.stringify({
        about: "I wire halls.",
        skills: ["electrics"],
        open_limit: 2,
      }),
    );
  });
});
