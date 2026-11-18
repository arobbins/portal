import { expect, test } from "../support/test";
import { callMcp, createToken, getMe, bearer } from "../support/api";
import { createAccount } from "../support/flows";

test("an AI assistant can use the account's tools over MCP", async ({ page, request }, testInfo) => {
  const email = await createAccount(page, testInfo, "Mcp User");
  const token = await createToken(
    page,
    "Claude",
    "Read and update your profile (profile:write)",
    "AI assistants (MCP)",
  );

  const initialize = await callMcp(request, token, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "e2e", version: "1" },
  });
  expect(initialize.status()).toBe(200);
  expect((await initialize.json()).result.protocolVersion).toBe("2025-06-18");

  const list = await callMcp(request, token, "tools/list");
  const names = (await list.json()).result.tools.map((tool: { name: string }) => tool.name);
  expect(names).toEqual(["get_profile", "update_profile", "send_feedback"]);

  const rename = await callMcp(request, token, "tools/call", {
    name: "update_profile",
    arguments: { name: "Renamed By Claude" },
  });
  const result = (await rename.json()).result;
  expect(result.isError).toBe(false);
  expect(result.structuredContent).toEqual({ name: "Renamed By Claude", email });

  await page.goto("/account");
  await expect(page.getByText("Renamed By Claude").first()).toBeVisible();
  await page.goto("/account/developer");
  await expect(page.locator("#api-tokens").getByText("MCP", { exact: true })).toBeVisible();
});

test("tool problems come back as tool errors the model can read", async ({ page, request }, testInfo) => {
  await createAccount(page, testInfo, "Mcp Errors");
  const token = await createToken(
    page,
    "Claude",
    "Read and update your profile (profile:write)",
    "AI assistants (MCP)",
  );

  const blank = await callMcp(request, token, "tools/call", {
    name: "update_profile",
    arguments: { name: "   " },
  });
  const result = (await blank.json()).result;
  expect(result.isError).toBe(true);
  expect(JSON.parse(result.content[0].text).code).toBe("name_missing");

  const unknown = await callMcp(request, token, "tools/call", { name: "delete_everything" });
  expect((await unknown.json()).error.code).toBe(-32602);
});

test("API tokens and MCP tokens only work where they were issued for", async ({ page, request }, testInfo) => {
  await createAccount(page, testInfo, "Mcp Audience");
  const apiToken = await createToken(page, "Script");
  const mcpToken = await createToken(
    page,
    "Assistant",
    "Read your profile (profile:read)",
    "AI assistants (MCP)",
  );

  expect((await callMcp(request, apiToken, "ping")).status()).toBe(401);
  expect((await getMe(request, `Bearer ${mcpToken}`)).status()).toBe(401);
  expect((await callMcp(request, mcpToken, "ping")).status()).toBe(200);
  const malformed = await request.post("/mcp", {
    headers: { ...bearer(mcpToken), "Content-Type": "application/json" },
    data: "{not json",
  });
  expect(malformed.status()).toBe(400);
});
