---
paths:
  - "assets/css/layout/**"
  - "assets/css/pages/**"
  - "src/layout/**"
  - "src/account/**"
  - "src/pages/api_docs.rs"
  - "src/pages/api_docs/**"
  - "src/pages/styleguide.rs"
  - "src/pages/styleguide/**"
  - "e2e/tests/mobile.spec.ts"
---

# Responsive layout and page layouts

- **Responsive layout**: one breakpoint, `@media (--phone)` (760px; plus
  `--narrow`, 1100px, inside `l-docs`), defined with `@custom-media` in
  `variables.css` and inlined by `build.rs`; never write the px value. Below it the header's links and theme switcher move into a native
  `popover` (`#site-menu`) opened by the Menu button, and the account
  sidebar becomes a row of tabs above the content. Everything is
  `box-sizing: border-box` (`reset.css`); check new pages at 390px (`mobile.spec.ts`).
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
