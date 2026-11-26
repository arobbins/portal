---
paths:
  - "src/components/**"
  - "src/*/pages/**"
  - "src/pages/**"
  - "src/layout/**"
  - "assets/css/**"
---

# Components

- `a-lead` (CSS only, `components/lead.css`): the intro paragraph right
  under a page heading.
- `src/components/` — reusable UI, one file per component; each has a matching
  CSS file in `assets/css/components/` and a demo in `src/pages/styleguide/`.
  Import components by module path: `components::button::{button, ButtonSize}`
  (`#[component]` generates a module named after the function; a
  multi-file component such as `form` re-exports its parts with `pub use`). Available: `button` (Primary /
  Secondary / Danger; pass `href` to render it as a link, sized to its content, which ignores
  `busy`, `disabled` and `button_type`), `form` controls (incl. `code_field`; `form_field` / `form_input` take
  `max_length`; `form_field(hide_label: true)` keeps the label for screen
  readers only, for a search box with a placeholder; modal forms use `DialogForm` + `dialog_input`,
  `dialog_form_error`, `dialog_error_toast` from `form/dialog.rs`, and render
  through `modal_form` + `ModalSubmit` from `form/submit.rs`), `card` +
  `card_header` + `card_body` (every settings block), `badge`, `avatar`,
  `tone` (`Tone`: the `tone` prop of `notice`, `badge`, `toaster` and
  property tags),
  `label` (`LabelSize::Sm` above lists, demos and sidebar groups; `Md`
  above fields; pass `for_id` to tie it to a control, which `form_label`
  and fieldset legends already do),
  `menu` (popover, opened by any button with `popovertarget`), `tabs` + `tab`
  (link tabs), `table` (columns are `Column` consts built with
  `.sortable(key)` / `.number()` / `.date()` / `.wide_only()` /
  `.truncated()` / `.grow()` (takes the spare width: long text); the component renders the header from them, rows use
  `table_row` + `table_row_heading` for the row's main cell (give it
  `href` to make the whole cell the row's one link; never make the whole
  row clickable, so other cells stay selectable) + `table_cell`,
  each given its `Column`, so alignment and phone visibility follow the
  column; props: required `caption` (`caption_hidden` to hide it visually),
  `sorting`, `empty`, `density`, `layout: TableLayout::Stacked` for rows
  that become cards on phones, `sticky_header`; list pages read
  `?q&sort&dir&page` with `listing::ListQuery<K>` (`K: SortKey`), which
  builds the sort links, page links and the `FindX` action input, so never
  parse those params by hand), `pager` (Previous / Next links),
  `empty_state`, `toggle` (a `role="switch"` checkbox), `setting_toggle` (a `toggle` that saves itself through a `#[procedure]` taking one `bool` as soon as it's switched: disabled while saving, `success_toast` on success, flips back with an error toast on failure; put it in a plain POST form with a `<noscript>` Save button for the no-JS path), `copy_field`,
  `property_list` + `property` (name, badge tags, description, detail and
  actions: lists of fields, passkeys, tokens), `code_panel` + `code_block`
  (dark code with a copy button; `language: CodeLanguage::Json | Shell`
  highlights it on the server via `src/syntax.rs`, `line_numbers: true`
  adds CSS-only line numbers that are never copied; its Copy button is
  its own small dark-panel button on purpose, because `button` variants are
  styled for light surfaces, while `copy_field` uses a Secondary `button`),
  `modal`, `confirm_dialog` (destructive confirmations), `notice`, `toaster`,
  `icon`, `theme_switcher`. Use these before writing page-specific markup.
