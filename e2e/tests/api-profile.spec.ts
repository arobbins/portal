import { bearer, createToken, createTokenDialog, getMe } from "../support/api";
import { createAccount } from "../support/flows";
import { expect, test } from "../support/test";

test("a token with update access renames the user, and the app shows it", async ({
  page,
  request,
}, testInfo) => {
  const email = await createAccount(page, testInfo, "Api Renamer");
  const token = await createToken(page, "Renamer", "Read and update your profile (profile:write)");
  await createTokenDialog(page).getByRole("button", { name: "Done" }).click();

  const response = await request.patch("/api/v1/me", {
    headers: bearer(token),
    data: { name: "  Renamed By Api  " },
  });

  expect(response.status()).toBe(200);
  expect(await response.json()).toEqual({ name: "Renamed By Api", email });
  expect((await (await getMe(request, `Bearer ${token}`)).json()).name).toBe("Renamed By Api");
  await page.goto("/account");
  await expect(page.locator("#your-details")).toContainText("Renamed By Api");
});

test("a read-only token can't rename and is told which scope it needs", async ({
  page,
  request,
}, testInfo) => {
  await createAccount(page, testInfo, "Api Reader");
  const token = await createToken(page, "Reader");

  const response = await request.patch("/api/v1/me", {
    headers: bearer(token),
    data: { name: "Not Allowed" },
  });

  expect(response.status()).toBe(403);
  expect(response.headers()["www-authenticate"]).toContain('scope="profile:write"');
  expect(await response.json()).toMatchObject({
    status: 403,
    code: "insufficient_scope",
  });
  await page.goto("/account");
  await expect(page.locator("#your-details")).toContainText("Api Reader");
});

test("bad requests get errors that say what to fix", async ({ page, request }, testInfo) => {
  await createAccount(page, testInfo, "Api Mistakes");
  const token = await createToken(page, "Mistakes", "Read and update your profile (profile:write)");

  const tooLong = await request.patch("/api/v1/me", {
    headers: bearer(token),
    data: { name: "x".repeat(101) },
  });
  expect(tooLong.status()).toBe(422);
  expect(tooLong.headers()["content-type"]).toBe("application/problem+json");
  expect(await tooLong.json()).toMatchObject({
    code: "name_too_long",
    errors: [{ field: "name", code: "name_too_long" }],
  });

  const unknownField = await request.patch("/api/v1/me", {
    headers: bearer(token),
    data: { nickname: "Typo" },
  });
  expect(unknownField.status()).toBe(400);
  expect((await unknownField.json()).code).toBe("invalid_json");

  const notJson = await request.patch("/api/v1/me", {
    headers: { ...bearer(token), "Content-Type": "text/plain" },
    data: "name=Plain",
  });
  expect(notJson.status()).toBe(415);
  expect((await notJson.json()).code).toBe("unsupported_media_type");
});

test("unknown endpoints answer in the same error format", async ({ request }) => {
  const response = await request.get("/api/v1/nothing-here");

  expect(response.status()).toBe(404);
  expect(response.headers()["content-type"]).toBe("application/problem+json");
  expect((await response.json()).code).toBe("not_found");
});

test("a wrong method is refused with the methods that work", async ({ request }) => {
  const response = await request.delete("/api/v1/me");

  expect(response.status()).toBe(405);
  expect(response.headers()["allow"]).toBe("GET, PATCH, HEAD");
});
