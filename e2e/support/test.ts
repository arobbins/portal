import { test as base, expect } from "@playwright/test";

type Fixtures = {
  allowedConsoleErrors: RegExp[];
  holdToasts: boolean;
  failOnJavaScriptErrors: void;
  holdToastsOpen: void;
};

const HELD_TOAST_CSS =
  ".a-toaster--is-open { animation-play-state: paused !important; pointer-events: none !important; }";

export const test = base.extend<Fixtures>({
  allowedConsoleErrors: [[], { option: true }],
  holdToasts: [true, { option: true }],

  holdToastsOpen: [
    async ({ page, holdToasts }, use) => {
      if (holdToasts) {
        await page.addInitScript((css) => {
          const sheet = new CSSStyleSheet();
          sheet.replaceSync(css);
          document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
        }, HELD_TOAST_CSS);
      }
      await use();
    },
    { auto: true },
  ],

  failOnJavaScriptErrors: [
    async ({ page, allowedConsoleErrors }, use) => {
      const errors: string[] = [];

      page.on("pageerror", (error) => {
        errors.push(`Uncaught exception: ${error.message}`);
      });

      page.on("console", (message) => {
        if (message.type() !== "error") return;
        const text = message.text();
        if (allowedConsoleErrors.some((pattern) => pattern.test(text))) return;
        errors.push(`Console error: ${text}`);
      });

      await use();

      expect(errors, "the page logged JavaScript errors").toEqual([]);
    },
    { auto: true },
  ],
});

export { expect };
