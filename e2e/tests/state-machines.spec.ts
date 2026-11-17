import type { Page, Route } from "@playwright/test";
import { type Cell, type Machine, eachCell, writeResults } from "../support/state-machines";
import { expect, test } from "../support/test";

test.skip(({ browserName }) => browserName !== "chromium", "one browser writes the report");
test.describe.configure({ timeout: 180_000 });

const FORM = "/reset-password";
const SUBMIT = "Send reset link";

type SubmitState = "idle" | "busy";
type SubmitEvent =
  | "submit_valid"
  | "submit_invalid"
  | "submit_slow"
  | "press_again"
  | "response_arrives"
  | "back_after_submit";

const ONE_REQUEST = "A busy form sends one request, however often it's pressed.";
const BUSY_IS_SHOWN = "While busy, the button is disabled and shows its spinner.";
const NEVER_STUCK =
  "Once the answer arrives, or Back restores the page, the button is enabled and idle again.";

const formSubmit: Machine<SubmitState, SubmitEvent> = {
  name: "Form submit",
  summary:
    "A form's submit button while a request is on its way, using the reset-password request form: every page form shares the same busy state.",
  testedBy: "e2e/tests/state-machines.spec.ts (Chromium)",
  states: { idle: "Idle", busy: "Busy" },
  events: {
    submit_valid: "Submits a valid form",
    submit_invalid: "Submits a form the server sends back",
    submit_slow: "Submits while the server is slow",
    press_again: "Presses Enter again",
    response_arrives: "The answer arrives",
    back_after_submit: "Submits, then presses Back",
  },
  rules: [ONE_REQUEST, BUSY_IS_SHOWN, NEVER_STUCK],
  expect(from, event) {
    if (from === "idle") {
      if (event === "submit_slow") return { moves: "busy" };
      if (event === "press_again" || event === "response_arrives") {
        return { unreachable: "Only matters while a request is on its way." };
      }
      return { moves: "idle" };
    }
    if (event === "press_again") return { refused: true };
    if (event === "response_arrives") return { moves: "idle" };
    return { unreachable: "The form is busy: only pressing again or the answer can happen." };
  },
};

class SlowServer {
  held: Route[] = [];
  holding = false;
  posts = 0;

  async attach(page: Page) {
    page.on("request", (request) => {
      if (request.method() === "POST" && new URL(request.url()).pathname === FORM) this.posts++;
    });
    await page.route(`**${FORM}`, async (route) => {
      if (route.request().method() === "POST" && this.holding) {
        this.held.push(route);
        return;
      }
      await route.continue();
    });
  }

  async release() {
    this.holding = false;
    const held = this.held.splice(0);
    for (const route of held) await route.continue();
  }
}

class ButtonWatch {
  last = { busy: false, disabled: false };

  async attach(page: Page) {
    page.on("console", (message) => {
      const text = message.text();
      if (text.startsWith("BUTTON ")) this.last = JSON.parse(text.slice("BUTTON ".length));
    });
    await page.addInitScript((label) => {
      const report = () => {
        const button = [...document.querySelectorAll("button")].find((element) =>
          element.textContent?.includes(label),
        );
        const state = {
          busy: !!button?.classList.contains("a-button--is-busy"),
          disabled: !!button?.disabled,
        };
        console.log(`BUTTON ${JSON.stringify(state)}`);
      };
      document.addEventListener("submit", () => setInterval(report, 50), true);
      addEventListener("pageshow", report);
    }, SUBMIT);
  }

  async settle(page: Page) {
    await page.waitForTimeout(200);
    return this.last;
  }
}

async function submit(page: Page, email: string, valid: boolean) {
  const loaded = page.waitForEvent("load");
  if (!valid) await page.locator("form").last().evaluate((form: HTMLFormElement) => (form.noValidate = true));
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Email").press("Enter");
  await loaded;
}

async function applySubmit(page: Page, server: SlowServer, event: SubmitEvent, email: string) {
  switch (event) {
    case "submit_valid":
      return submit(page, email, true);
    case "submit_invalid":
      return submit(page, "not-an-email", false);
    case "submit_slow":
      server.holding = true;
      await page.getByLabel("Email").fill(email);
      await page.getByLabel("Email").focus();
      await page.keyboard.press("Enter");
      await expect.poll(() => server.held.length).toBe(1);
      return;
    case "press_again":
      await page.keyboard.press("Enter");
      await page.waitForTimeout(300);
      return;
    case "response_arrives": {
      const loaded = page.waitForEvent("load");
      await server.release();
      await loaded;
      return;
    }
    case "back_after_submit":
      await submit(page, email, true);
      await page.goBack();
      await page.waitForLoadState("load");
      await expect(page.getByRole("button", { name: SUBMIT })).toBeVisible();
      return;
  }
}

test("the form submit state machine", async ({ page }) => {
  const server = new SlowServer();
  await server.attach(page);
  const watch = new ButtonWatch();
  await watch.attach(page);
  const cells: Cell<SubmitState, SubmitEvent>[] = [];
  let count = 0;
  for (const { from, event, expected } of eachCell(formSubmit)) {
    if ("unreachable" in expected) {
      cells.push({ from, event, actual: null, accepted: false, broken: [] });
      continue;
    }
    const email = `submit-${count++}@example.test`;
    await page.goto(FORM);
    if (from === "busy") await applySubmit(page, server, "submit_slow", email);
    expect((await watch.settle(page)).busy ? "busy" : "idle").toBe(from);
    const postsBefore = server.posts;
    await applySubmit(page, server, event, email);
    const button = await watch.settle(page);
    const actual: SubmitState = button.busy ? "busy" : "idle";
    const broken: string[] = [];
    const sent = server.posts - postsBefore;
    if (sent > (event === "press_again" ? 0 : 1)) broken.push(ONE_REQUEST);
    const { disabled } = button;
    if (actual === "busy" && !disabled) broken.push(BUSY_IS_SHOWN);
    const settles = event === "response_arrives" || event === "back_after_submit";
    if (settles && (actual !== "idle" || disabled)) broken.push(NEVER_STUCK);
    cells.push({ from, event, actual, accepted: true, broken });
    await server.release();
  }
  expect(writeResults(formSubmit, cells)).toEqual([]);
});

type Theme = "dark" | "light" | "system";
type ThemeEvent =
  | "pick_light"
  | "pick_dark"
  | "pick_system"
  | "post_light"
  | "bad_cookie"
  | "clear_cookie"
  | "back_after_pick";

const SERVER_AGREES = "After a reload the server renders the same theme, so it never flashes.";
const ONE_PRESSED = "Exactly the shown theme's button is pressed in the switcher.";
const COLOR_SCHEME = "The page's color-scheme matches the theme.";
const SCHEMES: Record<Theme, string> = { dark: "dark", light: "light", system: "light dark" };

const theme: Machine<Theme, ThemeEvent> = {
  name: "Theme",
  summary:
    "The color theme a browser shows: picked in the switcher (with or without JavaScript) and kept in a cookie.",
  testedBy: "e2e/tests/state-machines.spec.ts (Chromium)",
  states: { dark: "Dark (default)", light: "Light", system: "System" },
  events: {
    pick_light: "Picks Light",
    pick_dark: "Picks Dark",
    pick_system: "Picks System",
    post_light: "Picks Light without JavaScript",
    bad_cookie: "Cookie holds an unknown theme",
    clear_cookie: "Cookies are cleared",
    back_after_pick: "Picks Light on another page, then presses Back",
  },
  rules: [SERVER_AGREES, ONE_PRESSED, COLOR_SCHEME],
  expect(_from, event) {
    switch (event) {
      case "pick_dark":
      case "bad_cookie":
      case "clear_cookie":
        return { moves: "dark" };
      case "pick_system":
        return { moves: "system" };
      default:
        return { moves: "light" };
    }
  },
};

async function shownTheme(page: Page): Promise<Theme> {
  return (await page.locator("html").getAttribute("data-theme")) as Theme;
}

function switcher(page: Page) {
  return page.getByRole("group", { name: "Color theme" }).first();
}

async function pick(page: Page, label: string) {
  await switcher(page).getByRole("button", { name: label }).click();
}

async function setThemeCookie(page: Page, value: string | null) {
  await page.context().clearCookies();
  if (value) {
    await page.context().addCookies([{ name: "theme", value, url: new URL(page.url()).origin }]);
  }
}

async function applyTheme(page: Page, event: ThemeEvent) {
  switch (event) {
    case "pick_light":
      return pick(page, "Light");
    case "pick_dark":
      return pick(page, "Dark");
    case "pick_system":
      return pick(page, "System");
    case "post_light": {
      const loaded = page.waitForEvent("load");
      await page.evaluate(() => {
        const form = document.createElement("form");
        form.method = "post";
        form.action = "/theme";
        for (const [name, value] of [["theme", "light"], ["return_to", "/"]]) {
          const input = document.createElement("input");
          input.type = "hidden";
          input.name = name;
          input.value = value;
          form.append(input);
        }
        document.body.append(form);
        form.submit();
      });
      await loaded;
      return;
    }
    case "bad_cookie":
      await setThemeCookie(page, "purple");
      await page.reload();
      return;
    case "clear_cookie":
      await setThemeCookie(page, null);
      await page.reload();
      return;
    case "back_after_pick":
      await page.goto("/about");
      await pick(page, "Light");
      await page.goBack();
      await page.waitForLoadState("load");
      return;
  }
}

async function themeRules(page: Page, shown: Theme): Promise<string[]> {
  const broken: string[] = [];
  const html = await (await page.request.get(new URL(page.url()).pathname)).text();
  if (!html.includes(`data-theme="${shown}"`)) broken.push(SERVER_AGREES);
  const pressed = await switcher(page).locator('button[aria-pressed="true"]').all();
  const values = await Promise.all(pressed.map((button) => button.getAttribute("value")));
  if (values.length !== 1 || values[0] !== shown) broken.push(ONE_PRESSED);
  const scheme = await page.locator('meta[name="color-scheme"]').getAttribute("content");
  if (scheme !== SCHEMES[shown]) broken.push(COLOR_SCHEME);
  return broken;
}

test("the theme state machine", async ({ page }) => {
  const cells: Cell<Theme, ThemeEvent>[] = [];
  for (const { from, event } of eachCell(theme)) {
    await page.goto("/");
    await setThemeCookie(page, from === "dark" ? null : from);
    await page.reload();
    expect(await shownTheme(page)).toBe(from);
    await applyTheme(page, event);
    const actual = await shownTheme(page);
    cells.push({ from, event, actual, accepted: true, broken: await themeRules(page, actual) });
  }
  expect(writeResults(theme, cells)).toEqual([]);
});

type ToastState = "hidden" | "open";
type ToastEvent = "flash_arrives" | "click" | "time_runs_out" | "reload";

const AT_MOST_ONE = "At most one toast is open on a page.";
const ANNOUNCED = "An open toast sits in a live region, so screen readers announce it.";
const HIDDEN_IS_GONE = "A closed toast can't be seen or read by screen readers.";

const toast: Machine<ToastState, ToastEvent> = {
  name: "Toast",
  summary:
    "The toast a page shows for a flash message: open on arrival, closed by a click or after a few seconds.",
  testedBy: "e2e/tests/state-machines.spec.ts (Chromium)",
  states: { hidden: "Closed", open: "Open" },
  events: {
    flash_arrives: "A page arrives with a flash",
    click: "Clicks the toast",
    time_runs_out: "Waits for it to time out",
    reload: "Reloads the page",
  },
  rules: [AT_MOST_ONE, ANNOUNCED, HIDDEN_IS_GONE],
  expect(from, event) {
    if (event === "flash_arrives") return { moves: "open" };
    if (from === "hidden") {
      return event === "click" ? { unreachable: "There's no toast to click." } : { refused: true };
    }
    return { moves: "hidden" };
  },
};

async function showResetToast(page: Page, email: string) {
  await page.goto(FORM);
  await submit(page, email, true);
}

async function toastState(page: Page): Promise<ToastState> {
  return (await page.locator(".a-toaster--is-open").count()) > 0 ? "open" : "hidden";
}

test.describe("toasts that time out", () => {
  test.use({ holdToasts: false });

  test("the toast state machine", async ({ page }) => {
    const cells: Cell<ToastState, ToastEvent>[] = [];
    let count = 0;
    for (const { from, event, expected } of eachCell(toast)) {
      if ("unreachable" in expected) {
        cells.push({ from, event, actual: null, accepted: false, broken: [] });
        continue;
      }
      const email = () => `toast-${count++}@example.test`;
      if (from === "open") await showResetToast(page, email());
      else await page.goto(FORM);
      expect(await toastState(page)).toBe(from);
      switch (event) {
        case "flash_arrives":
          await showResetToast(page, email());
          break;
        case "click":
          await page.locator(".a-toaster--is-open").click();
          break;
        case "time_runs_out":
          await page.waitForTimeout(6_000);
          break;
        case "reload":
          await page.reload();
          break;
      }
      await page.waitForTimeout(400);
      const actual = await toastState(page);
      const broken: string[] = [];
      const open = page.locator(".a-toaster--is-open");
      if ((await open.count()) > 1) broken.push(AT_MOST_ONE);
      if (actual === "open") {
        const live = await open.first().evaluate(
          (element) => !!element.closest('[role="status"], [role="alert"], [aria-live]'),
        );
        if (!live) broken.push(ANNOUNCED);
      }
      const closed = page.locator(".a-toaster:not(.a-toaster--is-open) .a-notice");
      for (const element of await closed.all()) {
        if (await element.isVisible()) {
          broken.push(HIDDEN_IS_GONE);
          break;
        }
      }
      cells.push({ from, event, actual, accepted: true, broken });
    }
    expect(writeResults(toast, cells)).toEqual([]);
  });
});

type NoticeState = "shown" | "dismissing" | "dismissed";
type NoticeEvent = "click" | "keyboard" | "fade_ends" | "reload";

const DISMISSIBLE = "Info: This is a dismissible notice.";
const PLAIN = "Info: This is a notice.";
const GONE_TAKES_NO_SPACE = "A dismissed notice takes no space and can't be reached.";
const PLAIN_NOTICES_STAY = "A plain notice never reacts to clicks.";
const SLOW_FADE = ".a-notice--is-dismissing { transition-duration: 2s !important; }";

const notice: Machine<NoticeState, NoticeEvent> = {
  name: "Notice",
  summary:
    "A dismissible notice: shown, fading out after it's dismissed, then gone. Dismissal isn't remembered.",
  testedBy: "e2e/tests/state-machines.spec.ts (Chromium)",
  states: { shown: "Shown", dismissing: "Fading out", dismissed: "Gone" },
  events: {
    click: "Clicks it",
    keyboard: "Dismisses it from the keyboard",
    fade_ends: "The fade-out finishes",
    reload: "Reloads the page",
  },
  rules: [GONE_TAKES_NO_SPACE, PLAIN_NOTICES_STAY],
  expect(from, event) {
    if (event === "reload") return from === "shown" ? { refused: true } : { moves: "shown" };
    if (from === "shown") return event === "fade_ends" ? { refused: true } : { moves: "dismissing" };
    if (from === "dismissing") return event === "fade_ends" ? { moves: "dismissed" } : { refused: true };
    return event === "fade_ends" ? { refused: true } : { unreachable: "The notice is gone." };
  },
};

function dismissible(page: Page) {
  return page.locator(".a-notice--is-dismissible");
}

async function noticeState(page: Page): Promise<NoticeState> {
  const classes = (await dismissible(page).getAttribute("class")) ?? "";
  if (classes.includes("a-notice--is-dismissed")) return "dismissed";
  if (classes.includes("a-notice--is-dismissing")) return "dismissing";
  return "shown";
}

async function openStyleguide(page: Page) {
  await page.goto("/styleguide");
  await page.addStyleTag({ content: SLOW_FADE });
  await expect(page.getByText(DISMISSIBLE)).toBeVisible();
}

async function dismissFromKeyboard(page: Page) {
  const target = page.getByText(DISMISSIBLE);
  for (let step = 0; step < 200; step++) {
    await page.keyboard.press("Tab");
    const inside = await target.evaluate((element) => element.contains(document.activeElement));
    if (inside) {
      await page.keyboard.press("Enter");
      return;
    }
  }
}

test("the notice state machine", async ({ page }) => {
  const cells: Cell<NoticeState, NoticeEvent>[] = [];
  for (const { from, event, expected } of eachCell(notice)) {
    if ("unreachable" in expected) {
      cells.push({ from, event, actual: null, accepted: false, broken: [] });
      continue;
    }
    await openStyleguide(page);
    if (from !== "shown") await page.getByText(DISMISSIBLE).click();
    if (from === "dismissed") await expect.poll(() => noticeState(page)).toBe("dismissed");
    expect(await noticeState(page)).toBe(from);
    switch (event) {
      case "click":
        await page.getByText(DISMISSIBLE).click({ force: true });
        break;
      case "keyboard":
        await dismissFromKeyboard(page);
        break;
      case "fade_ends":
        await page.waitForTimeout(2_500);
        break;
      case "reload":
        await page.reload();
        await page.addStyleTag({ content: SLOW_FADE });
        break;
    }
    const actual = await noticeState(page);
    const broken: string[] = [];
    if (actual === "dismissed" && (await page.getByText(DISMISSIBLE).isVisible())) {
      broken.push(GONE_TAKES_NO_SPACE);
    }
    const plain = page.getByText(PLAIN, { exact: true });
    await plain.click();
    if (!(await plain.isVisible())) broken.push(PLAIN_NOTICES_STAY);
    cells.push({ from, event, actual, accepted: true, broken });
  }
  expect(writeResults(notice, cells)).toEqual([]);
});
