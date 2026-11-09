import { expect, test } from "../support/test";

test("the WebTransport page says the demo isn't connected yet", async ({ page }) => {
  await page.goto("/webtransport");

  await expect(
    page.getByText("The WebTransport demo isn't connected to a server yet."),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Connect" })).toHaveCount(0);
});
