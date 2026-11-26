import { expect, test } from "../support/test";

const modifier = process.platform === "darwin" ? "Meta" : "Control";

test("a modifier-click on Yearly opens it in a new tab and leaves this one on Monthly", async ({
  page,
  context,
}) => {
  await page.goto("/pricing");
  const periods = page.getByRole("group", { name: "Billing period" });
  const monthly = periods.getByRole("link", { name: "Monthly" });
  const yearly = periods.getByRole("link", { name: /Yearly/ });
  await expect(monthly).toHaveAttribute("aria-current", "page");

  const opened = context.waitForEvent("page");
  await yearly.click({ modifiers: [modifier] });
  const tab = await opened;
  await tab.waitForLoadState();

  await expect(tab).toHaveURL(/\/pricing\?billing=yearly$/);
  await expect(
    tab.getByRole("group", { name: "Billing period" }).getByRole("link", { name: /Yearly/ }),
  ).toHaveAttribute("aria-current", "page");
  await expect(page).toHaveURL(/\/pricing$/);
  await expect(monthly).toHaveAttribute("aria-current", "page");
  await expect(yearly).not.toHaveAttribute("aria-current", "page");
});
