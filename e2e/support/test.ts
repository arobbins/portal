import { test as base, expect } from "@playwright/test";

type Fixtures = {
  allowedConsoleErrors: RegExp[];
  failOnJavaScriptErrors: void;
};

export const test = base.extend<Fixtures>({
  allowedConsoleErrors: [[], { option: true }],

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
