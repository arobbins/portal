import type { Page } from "@playwright/test";
import { createAccount } from "../support/flows";
import { expect, test } from "../support/test";

async function submitName(page: Page, name: string) {
  await page.getByLabel("Name").fill(name);
  await page.getByLabel("Name").press("Enter");
}

async function saveName(page: Page, name: string) {
  await page.getByRole("button", { name: "Edit details" }).click();
  await submitName(page, name);
}

test("saving a new name updates the page without reloading", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Before Name");
  await page.evaluate(() => {
    (window as unknown as { marker: string }).marker = "same-page";
  });

  await saveName(page, "After Name");

  await expect(page.getByText("Profile saved.")).toHaveCSS("opacity", "1");
  await expect(page.getByText("Name: After Name")).toBeVisible();
  await expect(page.getByRole("button", { name: "After Name" })).toBeVisible();
  expect(await page.evaluate(() => (window as unknown as { marker?: string }).marker)).toBe(
    "same-page",
  );
});

test("a name error is shown inside the dialog, not behind it", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Error Tester");

  await saveName(page, "   ");

  const dialog = page.getByRole("dialog", { name: "Edit profile details" });
  await expect(dialog.getByText("Name is required.")).toBeVisible();

  await submitName(page, "Valid Name");

  await expect(page.getByText("Profile saved.")).toHaveCSS("opacity", "1");
  await expect(dialog).toBeHidden();
  await expect(page.getByText("Your email is confirmed. Welcome!")).toHaveCSS("opacity", "0");
});

test.describe("when the server can't be reached", () => {
  test.use({
    allowedConsoleErrors: [/Failed to load resource|NetworkError|network connection/i],
  });

  test("the save button recovers and explains what happened", async ({ page }, testInfo) => {
    await createAccount(page, testInfo, "Offline Tester");
    await page.route("**/_topcoat/runtime/procedures/**", (route) => route.abort());

    await saveName(page, "Offline Name");

    const dialog = page.getByRole("dialog", { name: "Edit profile details" });
    await expect(
      dialog.getByText("We couldn't save your profile. Please try again."),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Save profile" })).toBeEnabled();
  });
});

test("closing the dialog without saving discards the edit", async ({ page }, testInfo) => {
  await createAccount(page, testInfo, "Kept Name");

  await page.getByRole("button", { name: "Edit details" }).click();
  await page.getByLabel("Name").fill("Abandoned edit");
  await page.getByRole("button", { name: "Close" }).click();
  await page.getByRole("button", { name: "Edit details" }).click();

  await expect(page.getByLabel("Name")).toHaveValue("Kept Name");
});
