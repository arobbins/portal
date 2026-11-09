import { register, uniqueEmail } from "../support/flows";
import { expect, test } from "../support/test";

const CHECK_EMAIL = "Check your email to confirm your registration.";

test("a confirmation message shows once and doesn't come back on refresh", async ({
  page,
}, testInfo) => {
  await register(page, "Flash tester", uniqueEmail(testInfo, "flash"));
  await expect(page).toHaveURL("/register");
  await expect(page.getByText(CHECK_EMAIL)).toHaveCSS("opacity", "1");

  await page.reload();

  await expect(page.getByText(CHECK_EMAIL)).toHaveCSS("opacity", "0");
});

test("a message waits for the page it's meant for", async ({ page }, testInfo) => {
  await page.goto("/register");
  const response = await page.request.post("/register", {
    form: { name: "Flash tester", email: uniqueEmail(testInfo, "elsewhere") },
    headers: { Origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
  expect(response.status()).toBe(303);

  await page.goto("/login");
  await expect(page.getByText(CHECK_EMAIL)).toHaveCount(0);

  await page.goto("/register");
  await expect(page.getByText(CHECK_EMAIL)).toHaveCSS("opacity", "1");
});

test("typing a message flag into the URL shows nothing", async ({ page }) => {
  await page.goto("/register?sent=true");

  await expect(page.getByText(CHECK_EMAIL)).toHaveCSS("opacity", "0");
});
