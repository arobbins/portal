---
name: new-component
description: Scaffold a new reusable UI component in this Topcoat app — the Rust component in src/components/, its BEM CSS in assets/css/components/, and its styleguide demo section. Use whenever adding a new component (card, badge, tabs, dropdown, etc.).
---

# New component

A component is always three files plus three registrations. Use `modal` as the reference:
`src/components/modal.rs`, `assets/css/components/modal.css`, `src/pages/styleguide/modal.rs`.

## 1. Rust component — `src/components/<name>.rs`

- Function `#[component] pub async fn <name>(cx: &Cx, ...) -> Result<impl View>`; `cx` first, named exactly `cx`.
- Variants are enums, never strings, with one naming rule:
  - status colour (neutral / info / success / warning / error) is the shared
    `components::tone::Tone` prop named `tone`; build the class with
    `tone.class("a-<name>")` and style `a-<name>--<tone>` in CSS for every `Tone::ALL`;
  - visual style is `<Name>Variant` (`ButtonVariant::Secondary`), size is `<Name>Size`
    (`Sm`, `Md`, `Lg`), and an HTML-level choice is named after it (`ButtonType`,
    `InputType`). Never `<Name>Kind`.
  Each enum has `fn class(self)` / `fn modifier(self)` returning the BEM classes.
- Optional props use `#[default]`; children are `#[default] child: Child<'_>`; extra attributes
  on the root element are `#[default] attrs: Attributes`.
- Compute class strings in Rust before `view!` (`$(...)` can't concatenate strings).
- Interactivity: prefer native HTML (`<dialog>`, `popover`, `command`/`commandfor`, CSS
  animations) over JS. Widget state is a `signal(cx, ...)` inside the component; shared state
  is a `&Signal<T>` prop.
- No code comments.

Register it: add `pub mod <name>;` to `src/components.rs` (alphabetical). Import it elsewhere by
module path: `use crate::components::<name>::{<name>, <Name>Variant};`.

## 2. CSS — `assets/css/components/<name>.css`

- Block class `a-<name>`, elements `a-<name>__part`, modifiers `a-<name>--variant`. Target only
  these classes, never element structure or ids.
- Colors only through role variables in `assets/css/variables.css` (`--color-text`,
  `--color-surface-card`, `--color-border`, …). If a new role is needed, add it there once as
  `light-dark(<light>, <dark>)` built from `--palette-*` values, named by role, never by
  lightness. Component-only colours go in the component section of `variables.css`
  (`--color-<component>-<role>`).
- Sizes only through the scales: `--space-*` for gap/padding/margin/inset, `--text-*` for
  font sizes, `--radius-*`, `--duration-*` for transitions. Literal px only for `0`, `1px`
  hairlines and dimensions tied to another element; `em` spacing only when it must scale with
  the component's own font size (button and form padding).
- Focus: `&:focus-visible { outline: 2px solid var(--color-focus); }`.
- Check it at 390px wide; the one breakpoint is `@media (max-width: 760px)`.

Register it: add `@import "./components/<name>.css";` to `assets/css/app.css` with the other
component imports (before the `pages/` imports).

## 3. Styleguide demo — `src/pages/styleguide/<name>.rs`

```rust
use topcoat::{Result, context::Cx, view::{View, component, view}};

use super::styleguide_section;
use crate::components::<name>::<name>;

#[component]
pub async fn <name>_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "<Title>",
            <name>(/* show every variant */)
        )
    })
}
```

Register it in `src/pages/styleguide.rs`: `mod <name>;` and `<name>::<name>_section()` in the
page body.

## 4. Finish

- Format: `topcoat fmt --rustfmt <files>` (the edit hook does this automatically for `.rs`).
- If the component has behavior, add a Playwright spec in `e2e/` that drives the styleguide
  demo by role/label (import `test`/`expect` from `e2e/support/test.ts`).
- Accessibility: the styleguide demo is already scanned by `e2e/tests/accessibility.spec.ts`
  (via `/styleguide`); a new page or a new dialog that uses the component gets added there.
- Add a line to the component list in CLAUDE.md only if it introduces a new convention.
- Run `./check.sh` before calling it done.
