---
paths:
  - "assets/css/**"
  - "assets/js/theme.js"
  - "src/theme.rs"
  - "src/components/**"
  - "src/layout/**"
  - "src/pages/styleguide/**"
  - "e2e/tests/mobile.spec.ts"
  - "e2e/tests/theme.spec.ts"
---

# Layout, CSS and theming

- **Responsive layout**: one breakpoint, `max-width: 760px` (600px for form
  rows). Below it the header's links and theme switcher move into a native
  `popover` (`#site-menu`) opened by the Menu button, and the account
  sidebar becomes a row of tabs above the content. `.l-contain` uses
  `box-sizing: border-box`; check new pages at 390px (`mobile.spec.ts`).
- **BEM classes are the styling contract**: components emit stable class
  names (e.g. `a-notice--success`); CSS targets only those, never element
  structure or ids. No inline styles; add a class instead.
- **Colors are role-named variables** in `assets/css/variables.css`
  (`--color-text`, `--color-surface-card`, `--border-control`, …), each
  defined once as `light-dark(<light>, <dark>)`. Never hard-code a color in
  a component file; never name a variable by lightness.
- **Theming**: `color-scheme` on `<html>` picks the `light-dark()` side.
  `data-theme` is `light`, `dark` (default) or `system` (follows the OS).
  The server reads the `theme` cookie in `inject_globals` (`src/theme.rs`)
  and renders `data-theme` + `<meta name="color-scheme">`, so there's no
  flash of the wrong theme. All theme JS lives in `assets/js/theme.js`
  (loaded in `<head>`): `theme_switcher` calls `window.setTheme`, and every
  page load or back/forward restore re-applies the cookie's theme, because
  browsers can show a cached copy rendered before the switch. Without JS the
  switcher falls back to `POST /theme`.
- Typography: Satoshi Bold headings, Diatype for body and all UI text; no
  monospace or all-caps UI text.
- New components follow the `new-component` skill (Rust + CSS + styleguide
  demo, all registered).
