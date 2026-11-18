---
paths:
  - "src/account/**"
  - "src/account.rs"
  - "src/*/pages/*_tab.rs"
  - "src/*/pages/*_page.rs"
  - "e2e/tests/account-tabs.spec.ts"
  - "e2e/tests/settings.spec.ts"
---

# Account settings layout

- **Account settings** (`account/`) have three levels, each from one enum:
  sections (`AccountTab`: Profile, Security, Billing, Settings, Developer;
  plus Admin for admins), pages (`AccountPage`, each with its own URL:
  `/account/<section>` when the section has one page,
  `/account/<section>/<page>` when it has several, and the bare section URL
  redirects to its first page) and cards (`AccountSection`: `label()`,
  `anchor()`, `path()` = its page + `#anchor`). The sidebar lists sections
  and, under a section with several pages, its pages (never cards); every
  page opens with `section_header` (the section's H1, plus underline tabs
  of its pages when there are several, which are the only page navigation
  on phones). **Which page a card belongs on**: a page answers one question
  the user arrives with ("What's my info?", "How do I sign in?", "Where am
  I signed in?"); lists that keep growing (devices, activity, invoices,
  users) get their own page; split a page past 4–5 cards; destructive
  actions go last on the page they belong to. A new card is a new
  `AccountSection` in exactly one `AccountPage::sections()`; a new page is
  an `AccountPage` in one `AccountTab::pages()` (the state machines check
  both). Pages are `<feature>/pages/<name>_tab.rs` / `account/pages/<name>_page.rs`
  components (`profile_tab`, `sign_in_page`, `devices_page`, `developer_tab`)
  with no heading of their own. Each setting is a card with an action button on the
  right of its header that opens a `modal` with the form. **Exception, inline settings**: a single independent choice that takes effect immediately, is as easy to undo as to make, can't fail on a field and needs no password (notification switches) is changed in place with `setting_toggle`, no modal. Typed input, validation that can fail, re-authentication, destructive changes or several fields saved together always use a modal.
  Inside the modal, submit through a `#[procedure]` (errors render in the
  dialog, success closes it and shows a toast) and keep a plain POST route
  as the no-JS fallback (errors come back as an error toast). Procedures
  return a `FieldError` (`field`, `message`) so the right input gets
  `aria-invalid`.
