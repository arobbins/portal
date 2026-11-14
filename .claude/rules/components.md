---
paths:
  - "src/components/**"
  - "src/*/pages/**"
  - "src/pages/**"
  - "src/layout/**"
  - "assets/css/**"
---

# Components

- `src/components/` — reusable UI, one file per component; each has a matching
  CSS file in `assets/css/components/` and a demo in `src/pages/styleguide/`.
  Import components by module path: `components::button::{button, ButtonSize}`
  (`#[component]` generates a module named after the function, so they can't
  be re-exported from `components.rs`). Available: `button` (Primary /
  Secondary / Danger; pass `href` to render it as a link, which ignores
  `busy`, `disabled` and `button_type`), `form` controls (incl. `code_field`), `card` +
  `card_header` + `card_body` (every settings block), `badge`, `avatar`,
  `tone` (`Tone`: the `tone` prop of `notice`, `badge`, `toaster` and
  property tags),
  `label` (`LabelSize::Sm` above lists, demos and sidebar groups; `Md`
  above fields; pass `for_id` to tie it to a control, which `form_label`
  and fieldset legends already do),
  `menu` (popover, opened by any button with `popovertarget`), `tabs` + `tab`
  (link tabs), `table` + `table_row`/`table_heading`/`table_cell`,
  `empty_state`, `toggle` (a `role="switch"` checkbox), `copy_field`,
  `property_list` + `property` (name, badge tags, description, detail and
  actions: lists of fields, passkeys, tokens), `code_panel` + `code_block`
  (dark code with a copy button; `language: CodeLanguage::Json | Shell`
  highlights it on the server via `src/syntax.rs`, `line_numbers: true`
  adds CSS-only line numbers that are never copied; its Copy button is
  its own small dark-panel button on purpose, because `button` variants are
  styled for light surfaces, while `copy_field` uses a Secondary `button`),
  `modal`, `confirm_dialog` (destructive confirmations), `notice`, `toaster`,
  `icon`, `theme_switcher`. Use these before writing page-specific markup.
