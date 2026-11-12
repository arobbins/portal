import { createAccount } from "../support/flows";
import { expect, test } from "../support/test";

test.use({ allowedConsoleErrors: [/status of 400/] });

test("every page tells browsers not to frame it or sniff its content", async ({ page }) => {
  const response = await page.goto("/");
  const headers = response!.headers();

  expect(headers["x-frame-options"]).toBe("DENY");
  expect(headers["content-security-policy"]).toBe("frame-ancestors 'none'");
  expect(headers["x-content-type-options"]).toBe("nosniff");
  expect(headers["referrer-policy"]).toBe("strict-origin-when-cross-origin");
});

test("signed-in pages are never cached, public pages can be", async ({ page }, testInfo) => {
  const publicPage = await page.goto("/about");
  expect(publicPage!.headers()["cache-control"]).toBeUndefined();

  await createAccount(page, testInfo, "Cache Tester");
  const accountPage = await page.goto("/account");

  expect(accountPage!.headers()["cache-control"]).toBe("no-store");
});

test("an absurdly long email is rejected before it's used", async ({ page }) => {
  const longEmail = `${"a".repeat(300)}@example.test`;

  await page.goto("/register");
  await page.getByLabel("Name").fill("Long Email");
  await page.getByLabel("Email").fill(longEmail);
  await page.getByLabel("Email").press("Enter");
  await expect(page.getByText("Please enter a valid email address.")).toBeVisible();

  await page.goto("/login");
  await page.getByLabel("Email").fill(longEmail);
  await page.getByLabel("Password").fill("whatever-password");
  await page.getByLabel("Password").press("Enter");
  await expect(page.getByText("Invalid email or password.")).toBeVisible();
});

test("a made-up link can't be used to submit a password", async ({ page }) => {
  for (const path of ["/user/verify/not-a-real-token", "/reset-password/not-a-real-token"]) {
    await page.goto("/login");
    const response = await page.request.post(path, {
      form: { password: "a-strong-password", confirm_password: "a-strong-password" },
      headers: { Origin: new URL(page.url()).origin },
    });

    expect(response.status()).toBe(400);
    expect(await response.text()).toContain("This link has expired");
  }
});

test("pages opened on another address move to the site's own address", async ({ page }) => {
  await page.goto("http://127.0.0.1:3100/about?from=ip");

  await expect(page).toHaveURL("http://localhost:3100/about?from=ip");
});
