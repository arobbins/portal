---
paths:
  - "assets/css/**"
  - "assets/icons/*.svg"
  - "assets/js/theme.js"
  - "src/theme.rs"
  - "src/components/**"
  - "src/pages/styleguide/**"
  - "e2e/tests/theme.spec.ts"
---

# CSS and theming

- **BEM classes are the styling contract**: components emit stable class
  names (e.g. `a-notice--success`); CSS targets only those, never element
  structure or ids. **Never inline styles**: no `style="…"` on any element, in
  views, JS or SVG files in `assets/icons/`; an icon's spacing belongs to the
  stylesheet that uses it (`check_conventions.py` fails on any). Use a class. Every class has
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
  `--text-2xs … --text-6xl` (rem, so they follow the browser's font size), `--radius-sm|md|lg|full`,
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
- Text meets 4.5:1 contrast and control borders, toggles and focus rings
  3:1 on every surface they sit on, in both themes. Anything that moves
  (translate, spin) stops under `prefers-reduced-motion: reduce`, in the
  component's own file; colour and opacity fades may stay.
- Only plain links (`a:not([class])`) dim on hover; a classed link (button,
  tab, nav, menu item) styles its own hover. Form-width blocks use
  `--width-form`. A component restyled in another's context exposes
  `--<component>-*` custom properties instead (see `--button-busy-*`).
- Headings are left-aligned; `page_heading` centres by default with
  `l-page-heading--centered`. The styleguide's Tokens section lists every
  `--color-*`, `--space-*`, `--text-*` and `--radius-*` from `variables.css`;
  a new token needs its sample class in `pages/styleguide-tokens.css` (a test
  checks).
- Don't use cross-document view transitions (`@view-transition`): Chromium
  flashes white between pages.

Page and responsive layout: `css-layout.md`.
