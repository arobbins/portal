import { bearer, callMcp, createToken, createTokenDialog } from "../support/api";
import { openAdmin } from "../support/admin";
import { createAccount } from "../support/flows";
import { expect, test } from "../support/test";

test.use({ allowedConsoleErrors: [/status of 40[34]/] });

test("an agent's feedback reaches the admin feedback page without its token", async ({
  page,
  browser,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Feedback Agent");
  const token = await createToken(page, "Feedback agent");
  await createTokenDialog(page).getByRole("button", { name: "Done" }).click();

  const response = await request.post("/api/v1/feedback", {
    headers: bearer(token),
    data: {
      category: "confusing_docs",
      endpoint: "update_profile",
      problem_code: "name_too_long",
      message: `I called it with Bearer ${token} and couldn't tell how long a name may be.`,
    },
  });

  expect(response.status()).toBe(200);
  expect((await response.json()).id).toMatch(/^fbk_[A-Za-z0-9]{20}$/);
  const admin = await (await browser.newContext()).newPage();
  await openAdmin(admin);
  await admin
    .getByRole("navigation", { name: "Admin" })
    .getByRole("link", { name: "API feedback" })
    .click();
  await expect(admin).toHaveURL("/admin/feedback");
  const row = admin.getByRole("row", { name: new RegExp(email) });
  await expect(row).toContainText("Bearer [redacted] and couldn't tell how long a name may be.");
  await expect(row).toContainText("update-profile");
  await expect(row).toContainText("name_too_long");
  await expect(admin.getByText(token)).toHaveCount(0);
});

test("feedback about an endpoint that doesn't exist names the field to fix", async ({
  page,
  request,
}, testInfo) => {
  await createAccount(page, testInfo, "Feedback Typo");
  const token = await createToken(page, "Typo agent");

  const response = await request.post("/api/v1/feedback", {
    headers: bearer(token),
    data: { category: "other", endpoint: "delete-everything", message: "Where is it?" },
  });

  expect(response.status()).toBe(422);
  expect(await response.json()).toMatchObject({
    code: "unknown_endpoint",
    errors: [{ field: "endpoint", code: "unknown_endpoint" }],
  });
});

test("every MCP token can find and use the feedback tool", async ({ page, request }, testInfo) => {
  await createAccount(page, testInfo, "Feedback Mcp");
  const token = await createToken(
    page,
    "Assistant",
    "Read your profile (profile:read)",
    "AI assistants (MCP)",
  );

  const listed = await (await callMcp(request, token, "tools/list")).json();
  const names = listed.result.tools.map((tool: { name: string }) => tool.name);
  expect(names).toContain("send_feedback");
  const called = await (
    await callMcp(request, token, "tools/call", {
      name: "send_feedback",
      arguments: { category: "missing_capability", message: "I can't list my API tokens." },
    })
  ).json();

  expect(called.result.isError).toBeFalsy();
  expect(called.result.content[0].text).toMatch(/fbk_/);
});
