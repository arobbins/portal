import { expect, test } from "../support/test";

test("the API reference lists every endpoint with what it needs and returns", async ({
  page,
}) => {
  await page.goto("/docs/api");

  await expect(page.getByRole("heading", { name: "API reference", level: 1 })).toBeVisible();
  const sidebar = page.getByRole("navigation", { name: "API reference" });
  await expect(sidebar.getByRole("link", { name: /Get your profile/ })).toBeVisible();
  await sidebar.getByRole("link", { name: /Update your profile/ }).click();
  await expect(page).toHaveURL(/#update-profile$/);

  const rename = page.getByRole("region", { name: "Update your profile" });
  await expect(rename.getByRole("heading", { name: "Update your profile" })).toBeVisible();
  await expect(rename.getByText("profile:write", { exact: true })).toBeVisible();
  await expect(rename.getByText("1–100 characters", { exact: true })).toBeVisible();
  await expect(rename.getByText("name_too_long", { exact: true })).toBeVisible();
  await expect(rename.getByText(/-X PATCH/)).toBeVisible();
});

test("the OpenAPI document describes the same endpoints", async ({ request }) => {
  const response = await request.get("/api/v1/openapi.json");

  expect(response.status()).toBe(200);
  const spec = await response.json();
  expect(spec.openapi).toBe("3.1.0");
  expect(Object.keys(spec.paths).sort()).toEqual(["/api/v1/feedback", "/api/v1/me"]);
  expect(spec.paths["/api/v1/me"].patch["x-required-scope"]).toBe("profile:write");
  expect(spec.paths["/api/v1/feedback"].post["x-required-scope"]).toBe("any");
});

test("code examples can be copied", async ({ page, browserName, context }) => {
  test.skip(browserName !== "chromium", "Clipboard permissions are only grantable in Chromium");
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/docs/api");

  const rename = page.getByRole("region", { name: "Update your profile" });
  await rename.getByRole("button", { name: "Copy code" }).first().click();

  await expect(rename.getByRole("status").first()).toHaveText("Copied to clipboard");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain("-X PATCH");
});

test("the reference starts with links to get a token and the OpenAPI spec", async ({ page }) => {
  await page.goto("/docs/api");

  await expect(page.getByRole("link", { name: "OpenAPI spec (JSON)" })).toHaveAttribute(
    "href",
    "/api/v1/openapi.json",
  );
  await page.getByRole("link", { name: "Create API token" }).click();

  await expect(page).toHaveURL(/\/login(#api-tokens)?$/);
});
