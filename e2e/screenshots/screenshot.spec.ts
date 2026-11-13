import { mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { createAccount, createAccountAtPasskeyPrompt } from "../support/flows";
import { test } from "../support/test";

const list = (value: string | undefined, fallback: string[]) =>
  value ? value.split(",").map((item) => item.trim()).filter(Boolean) : fallback;

const paths = list(process.env.SHOT_PATHS, ["/"]);
const themes = list(process.env.SHOT_THEMES, ["dark", "light"]);
const widths = list(process.env.SHOT_WIDTHS, ["390", "1280"]).map(Number);
const signedIn = process.env.SHOT_SIGNED_IN;
const selector = process.env.SHOT_SELECTOR;
const scale = Number(process.env.SHOT_SCALE ?? "1");
const outDir = resolve(process.env.SHOT_OUT ?? ".screenshots");
const variants: Record<string, string> = process.env.SHOT_VARIANTS
  ? JSON.parse(readFileSync(resolve(process.env.SHOT_VARIANTS), "utf8"))
  : { current: "" };

const settle = "*, *::before, *::after { transition: none !important; }";

function slug(path: string): string {
  return path.replace(/^\/+|\/+$/g, "").replace(/[^a-z0-9]+/gi, "-") || "home";
}

test.use({ deviceScaleFactor: scale });

test("screenshots", async ({ page, baseURL }, testInfo) => {
  test.setTimeout(120_000);
  mkdirSync(outDir, { recursive: true });

  if (signedIn === "new") {
    await createAccountAtPasskeyPrompt(page, testInfo, "Screenshot User");
  } else if (signedIn === "1") {
    await createAccount(page, testInfo, "Screenshot User");
  }

  const written: string[] = [];
  for (const path of paths) {
    for (const theme of themes) {
      await page.context().addCookies([{ name: "theme", value: theme, url: baseURL! }]);
      for (const width of widths) {
        await page.setViewportSize({ width, height: width < 760 ? 844 : 900 });
        for (const [name, css] of Object.entries(variants)) {
          await page.goto(path);
          await page.addStyleTag({ content: settle + css });
          await page.evaluate(() => document.fonts.ready);
          const file = `${outDir}/${slug(path)}-${name}-${theme}-${width}.png`;
          if (selector) {
            await page.locator(selector).first().screenshot({ path: file });
          } else {
            await page.screenshot({ path: file, fullPage: true });
          }
          written.push(file);
        }
      }
    }
  }
  console.log(`\nScreenshots:\n${written.join("\n")}`);
});
