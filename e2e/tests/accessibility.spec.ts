import AxeBuilder from "@axe-core/playwright";
import type { BrowserContext, Page, TestInfo } from "@playwright/test";
import { openAdmin } from "../support/admin";
import { createToken } from "../support/api";
import {
  ACCOUNT_PASSWORD,
  createAccount,
  createAccountAtPasskeyPrompt,
  logIn,
  logOut,
  register,
  skipPasskeyPrompt,
  uniqueEmail,
} from "../support/flows";
import { startLoopback } from "../support/loopback";
import { resetLink, verificationLink, waitForEmailAbout, waitForEmails } from "../support/mail";
import {
  authorizePath,
  exchangeCode,
  expectTokens,
  pkce,
  randomState,
  registerClient,
} from "../support/oauth";
import { addPasskeyToAccount, addVirtualAuthenticator, withoutPasskeyAutofill } from "../support/passkeys";
import { totp, turnOnTwoFactor } from "../support/totp";
import { adminUser, verifiedUser } from "../support/users";
import { expect, test } from "../support/test";

const FAILING_IMPACTS = ["serious", "critical"];
const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"];

const LOOKS = [
  { theme: "light", width: 390, height: 844 },
  { theme: "dark", width: 390, height: 844 },
  { theme: "light", width: 1280, height: 800 },
  { theme: "dark", width: 1280, height: 800 },
] as const;

type Look = (typeof LOOKS)[number];

type Finding = {
  where: string;
  rule: string;
  impact: string;
  help: string;
  targets: string[];
};

class Audit {
  readonly findings: Finding[] = [];
  readonly scanned: string[] = [];

  constructor(readonly look: Look) {}

  async pages(page: Page, paths: string[]) {
    for (const path of paths) {
      await page.goto(path);
      await this.page(page, shortPath(path));
    }
  }

  async page(page: Page, where: string) {
    await settle(page);
    await this.scan(page, where);
    await this.dialogs(page, where);
    await this.menus(page, where);
  }

  async dialogs(page: Page, where: string) {
    const ids = await page
      .locator("dialog:not([open])")
      .evaluateAll((dialogs) => dialogs.map((dialog) => dialog.id).filter(Boolean));
    for (const id of ids) {
      const dialog = page.locator(`[id="${id}"]`);
      const trigger = page.locator(`[command="show-modal"][commandfor="${id}"]`).first();
      if (await trigger.isVisible()) {
        await trigger.click();
      } else {
        await dialog.evaluate((element: HTMLDialogElement) => element.showModal());
      }
      await expect(dialog).toBeVisible();
      await settle(page);
      await this.scan(page, `${where} #${id}`, `[id="${id}"]`);
      await dialog.evaluate((element: HTMLDialogElement) => element.close());
    }
  }

  async menus(page: Page, where: string) {
    for (const trigger of await page.locator("button[popovertarget]").all()) {
      if (!(await trigger.isVisible())) continue;
      const id = await trigger.getAttribute("popovertarget");
      const menu = page.locator(`[id="${id}"]`);
      await trigger.click();
      await expect(menu).toBeVisible();
      await settle(page);
      await this.scan(page, `${where} #${id}`, `[id="${id}"]`);
      await menu.evaluate((element: HTMLElement) => element.hidePopover());
    }
  }

  async scan(page: Page, where: string, include?: string) {
    let builder = new AxeBuilder({ page }).withTags(WCAG_TAGS);
    if (include) builder = builder.include(include);
    const { violations } = await builder.analyze();
    this.scanned.push(where);
    for (const violation of violations) {
      this.findings.push({
        where,
        rule: violation.id,
        impact: violation.impact ?? "unknown",
        help: violation.help,
        targets: violation.nodes.map(
          (node) => `${node.target.join(" ")}: ${node.failureSummary?.split("\n").slice(1).join(" ").trim()}`,
        ),
      });
    }
  }

  async finish(testInfo: TestInfo) {
    const failing = this.findings.filter((finding) => FAILING_IMPACTS.includes(finding.impact));
    const listed = this.findings.filter((finding) => !FAILING_IMPACTS.includes(finding.impact));
    await testInfo.attach("accessibility.json", {
      body: JSON.stringify({ look: this.look, scanned: this.scanned, failing, listed }, null, 2),
      contentType: "application/json",
    });
    console.log(`a11y scanned ${this.scanned.length}: ${this.scanned.join(", ")}`);
    for (const line of summarize(listed)) console.log(line);
    expect(failing.map(describe), "serious or critical accessibility violations").toEqual([]);
  }
}

function shortPath(path: string): string {
  const url = new URL(path, "http://localhost");
  const pathname = url.pathname.replace(/\/[A-Za-z0-9_-]{32,}$/, "/{token}");
  return pathname.startsWith("/oauth/") ? pathname : pathname + url.search;
}

function summarize(findings: Finding[]): string[] {
  const byRule = new Map<string, Set<string>>();
  for (const finding of findings) {
    const key = `${finding.impact} ${finding.rule}: ${finding.help}`;
    const wheres = byRule.get(key) ?? new Set<string>();
    wheres.add(finding.where);
    byRule.set(key, wheres);
  }
  return [...byRule].map(([rule, wheres]) => `a11y listed, ${rule} on ${[...wheres].join(", ")}`);
}

function describe(finding: Finding): string {
  return `${finding.rule} on ${finding.where}: ${finding.help} [${finding.targets.join("; ")}]`;
}

async function useLook(page: Page, look: Look) {
  await page.setViewportSize({ width: look.width, height: look.height });
  await page.emulateMedia({ colorScheme: look.theme, reducedMotion: "reduce" });
  await setTheme(page.context(), look.theme);
}

async function setTheme(context: BrowserContext, theme: string) {
  const url = test.info().project.use.baseURL!;
  await context.addCookies([{ name: "theme", value: theme, url }]);
}

async function settle(page: Page) {
  await page.waitForLoadState("load");
  await page.mouse.move(0, 0);
  await page.evaluate(async () => {
    await document.fonts.ready;
    const running = document
      .getAnimations()
      .filter((animation) => animation.playState === "running")
      .map((animation) => animation.finished.catch(() => undefined));
    await Promise.race([Promise.all(running), new Promise((resolve) => setTimeout(resolve, 1_000))]);
  });
}

for (const look of LOOKS) {
  test.describe(`${look.theme} at ${look.width}px`, () => {
    let audit: Audit;

    test.beforeEach(async ({ page, browserName }) => {
      test.skip(browserName !== "chromium", "axe results don't depend on the engine; Chromium is enough");
      test.setTimeout(180_000);
      audit = new Audit(look);
      await useLook(page, look);
    });

    test.afterEach(async ({}, testInfo) => {
      if (testInfo.status === "skipped") return;
      await audit.finish(testInfo);
    });

    test.describe("signed out", () => {
      test.use({ allowedConsoleErrors: [/status of 40[04]/] });

      test("public pages, Pricing billed yearly and monthly, and the styleguide", async ({ page }) => {
        await audit.pages(page, [
          "/",
          "/about",
          "/pricing",
          "/pricing?billing=monthly",
          "/docs/api",
          "/styleguide",
          "/webtransport",
          "/no-such-page",
        ]);
      });

      test("sign in, register, reset password and expired links", async ({ page }) => {
        await audit.pages(page, [
          "/login",
          "/register",
          "/reset-password",
          "/reset-password/not-a-token",
          "/verify-email/not-a-token",
          "/confirm-email/not-a-token",
        ]);
      });
    });

    test("the link-sent notice and the emailed set-password and choose-password pages", async ({
      page,
    }, testInfo) => {
      const email = uniqueEmail(testInfo, "a11y-links");
      await register(page, "Link Tester", email);
      await expect(page.getByText(email)).toBeVisible();
      await audit.page(page, "/register sent");
      await audit.pages(page, [verificationLink((await waitForEmails(email))[0])]);

      const account = await createAccount(page, testInfo, "Reset Tester");
      await logOut(page, "Reset Tester");
      await page.goto("/reset-password");
      await page.getByLabel("Email").fill(account);
      await page.getByLabel("Email").press("Enter");
      await expect(page.getByText(account)).toBeVisible();
      await audit.pages(page, [resetLink(await waitForEmailAbout(account, "Reset your password"))]);
    });

    test("the passkey prompt after a new account", async ({ page }, testInfo) => {
      await createAccountAtPasskeyPrompt(page, testInfo, "Prompt Tester");
      await audit.pages(page, ["/passkey-setup"]);
      await skipPasskeyPrompt(page);
    });

    test("the OAuth approval screen, every account tab and its dialogs, and the two-factor step", async ({
      page,
      request,
    }, testInfo) => {
      await addVirtualAuthenticator(page);
      await withoutPasskeyAutofill(page);
      const email = await createAccount(page, testInfo, "Account Tester");
      await createToken(page);

      const loopback = await startLoopback();
      try {
        const clientId = await registerClient(request, "Claude a11y", [loopback.redirectUri]);
        const { verifier, challenge } = pkce();
        const resource = `${testInfo.project.use.baseURL}/mcp`;
        const approval = authorizePath({
          clientId,
          redirectUri: loopback.redirectUri,
          scope: "profile:read",
          state: randomState(),
          challenge,
          resource,
        });
        await audit.pages(page, [approval]);
        await page.getByRole("button", { name: "Allow access" }).click();
        const code = (await loopback.nextCallback()).get("code")!;
        await expectTokens(
          await exchangeCode(request, { clientId, redirectUri: loopback.redirectUri, code, verifier, resource }),
        );
      } finally {
        await loopback.close();
      }

      const { secret } = await turnOnTwoFactor(page);
      await addPasskeyToAccount(page);

      await audit.pages(page, [
        "/account",
        "/account/settings/authentication",
        "/account/settings/devices",
        "/account/settings/notifications",
        "/account/settings/time-zone",
        "/account/billing/plan",
        "/account/billing/invoices",
        "/account/developer",
        "/pricing",
      ]);

      await page.goto("/account");
      await logOut(page, "Account Tester");
      await logIn(page, email, ACCOUNT_PASSWORD);
      await expect(page).toHaveURL("/login/two-factor");
      await audit.page(page, "/login/two-factor");
      await page.getByLabel("Code").fill(totp(secret, Date.now() + 30_000));
      await page.getByLabel("Code").press("Enter");
      await expect(page).toHaveURL("/account");
    });

    test("the admin area, locked and unlocked", async ({ page, browser }) => {
      const lockedContext = await browser.newContext({ baseURL: test.info().project.use.baseURL });
      const locked = await lockedContext.newPage();
      await useLook(locked, look);
      await logIn(locked, adminUser.email, adminUser.password);
      await expect(locked).toHaveURL("/account");
      await audit.pages(locked, ["/account", "/admin/confirm"]);
      await lockedContext.close();

      await openAdmin(page);
      await page.goto(`/admin/users?q=${encodeURIComponent(verifiedUser.email)}`);
      await page.getByRole("link", { name: verifiedUser.name }).first().click();
      await expect(page).toHaveURL(/\/admin\/users\/usr_/);
      const userPage = new URL(page.url()).pathname;
      await audit.pages(page, ["/admin/users", userPage, "/admin/log", "/admin/feedback"]);
    });
  });
}
