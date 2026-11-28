---
paths:
  - "e2e/**"
---

# Browser quirks in Playwright tests

Work around these in the test, never by weakening what it checks.

- Submit forms with `press("Enter")` on the last field you typed in, never a
  click on the submit button: Firefox under Playwright sometimes drops the
  first click after typing (on any page, not just modals), so the form is
  never posted. Shared helpers in `flows.ts` already do this.
- A tab opened from a link (modifier-click) is slow in Firefox under load:
  it can take 15–60 s to come up (fresh content process per tab). Give that
  test `test.slow(browserName === "firefox")`; never retry the click, which
  opens more slow tabs and makes it worse.
- A new tab may start with JavaScript on even when the context turned it off
  (Chromium, rarely): assert either form of the control (the no-JS link or the
  JS switch), with `.or(…).first()`.
- Popovers and menus must not move while opening (fade, don't slide): a
  moving target makes Playwright press outside it, which light-dismisses it.
- Layout checks measure `document.body`, never `innerWidth`/`clientWidth`:
  CI's scrollbars plus `scrollbar-gutter: stable` make those 15px wider.
