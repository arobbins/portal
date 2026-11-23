---
paths:
  - "src/data_export.rs"
  - "src/data_export/**"
  - "src/*/export.rs"
---

# Data export

- Spec: `specs/export-my-data.md` (approved; change it first).
- `export_data` (`data_export/actions.rs`) is browser-only
  (`Permission::ExportData`): it re-authenticates (password or passkey via
  `check_reauth`), then allows 5 per hour per account (`allow_export`),
  builds a ZIP of CSVs in memory and returns it; nothing is stored or
  logged. It records `data_exported` and queues the "Your account data was
  downloaded" security email (no opt-out) in one transaction.
- **Every feature that stores data about a user contributes a dataset**:
  `<feature>/export.rs` exposes `<thing>_export(env, user) ->
  Result<Dataset, sqlx::Error>` (file, one-line `about` for the README, the
  `tables` it covers, `columns`, rows as strings; dates via `utc()` /
  `maybe_utc()`, booleans via `yes_no()`), listed in
  `data_export/datasets.rs::collect`. Secrets never go in a dataset. A
  table that holds only secrets goes in `LEFT_OUT` with a reason the README
  shows. `datasets/tests.rs` fails when a table with a `user_id` column is
  in neither. Every cell goes through `spreadsheet_safe` (R13): a value
  starting with `= + - @`, tab or CR gets a leading `'` unless it's a plain
  number; the README explains it.
- Page: the "Export data" row (in the "Your data" card) and dialog sit above Delete account on
  Profile; the JS path gets the file from `export_data_js` (base64) and
  starts the download in the browser; the no-JS path posts to
  `/account/data/export`, which answers with the file and
  `Cache-Control: no-store` (personal data is never cached). The Delete account
  dialog links to it ("Want a copy first?").
