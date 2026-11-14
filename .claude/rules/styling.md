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

- **Responsive layout**: one breakpoint, `max-width: 760px` (plus 1100px
  inside `l-docs`). Below it the header's links and theme switcher move into a native
  `popover` (`#site-menu`) opened by the Menu button, and the account
  sidebar becomes a row of tabs above the content. `.l-contain` uses
  `box-sizing: border-box`; check new pages at 390px (`mobile.spec.ts`).
- **Layouts**: the site header and footer (`l-header`, `l-footer`) are always
  full width with `--space-4` side padding, on every page. `l-contain` +
  `l-skinny` (centered, 800px) for ordinary
  pages; `l-docs` (`layout/docs.css`) for reference pages (`/docs/api`,
  `/styleguide`): a sticky
  `l-docs__sidebar` beside `l-docs__main`, which holds `l-docs__section`s,
  each with `l-docs__prose` and an optional sticky `l-docs__aside` for code
  (`l-docs__section--single` when there's no aside, `--first` for the top
  one). A page using `l-docs`
  goes full width automatically (the `:has()` rules at the top of
  `layout/docs.css` widen `main`; the account
  shell does the same at the top of `pages/account.css` to hide them: the
  only allowed structural selectors, because the root layout owns
  `l-contain`); asides drop below the prose under 1100px
  and the sidebar moves above the content at the phone breakpoint. Code
  panels use the `--color-code-*` variables (dark in both themes). Sidebars
  use `l-docs__sidebar-title` / `-subtitle` / `-group` and
  `l-nav__list--sidebar`; `l-docs__prose--wide` (900px) suits component
  demos; titles `page_heading(variant:
  PageHeadingVariant::Compact)`, and code goes in `code_panel` +
  `code_block`.
- **BEM classes are the styling contract**: components emit stable class
  names (e.g. `a-notice--success`); CSS targets only those, never element
  structure or ids. No inline styles; add a class instead. Every class has
  a prefix: `a-` components (`assets/css/components/`), `l-` layout and
  layout parts (`l-header`, `l-nav`, `l-footer`, `l-page-heading`,
  `l-contain`, `l-flex*`), `p-` one page (`p-account`, `p-auth`, `p-home`,
  in `assets/css/pages/`), `u-` utilities. State modifiers are
  `--is-<state>`.
- **Status colour is `Tone`** (`components/tone.rs`: Neutral, Info, Success,
  Warning, Error): the prop is always `tone`, the class comes from
  `tone.class("a-block")`, and CSS styles `a-block--<tone>` for every tone.
  Visual styles are `<Name>Variant`, sizes `<Name>Size`; never `*Kind`.
- **Colors are role-named variables** in `assets/css/variables.css`: raw
  values are `--palette-*` (used only inside `variables.css`); roles
  (`--color-text`, `--color-surface-card`, `--color-border-control`, …) are
  defined once as `light-dark(<light>, <dark>)` from them; component-only
  colours (`--color-button-*`, `--color-code-*`) sit in their own group
  there. Never hard-code a color in a component file; never name a variable
  by lightness.
- **Scales**: spacing `--space-0-5 … --space-24` (2px … 96px), font sizes
  `--text-2xs … --text-6xl`, `--radius-sm|md|lg|full`,
  `--duration-fast|normal`, `--z-*`. Use them for every gap, padding,
  margin, inset, font size, radius and transition. Literal px only for `0`,
  `1px` hairlines and dimensions tied to another element (a header offset);
  `em` spacing only where it must scale with the component's own font size
  (button and form padding); relative font sizes use `0.875em`.
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
