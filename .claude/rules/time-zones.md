---
paths:
  - "src/time_zone.rs"
  - "src/*/pages/**"
  - "src/pages/**"
  - "src/*/emails.rs"
  - "src/mail/**"
---

# Time zones

- **Time zones** (`src/time_zone.rs`): every date shown to a person goes
  through a `Zone` (`zone.date(at)` / `zone.date_time(at)`, e.g. "Nov 17,
  2026 at 14:03 CET"), taken from the viewer: `caller.user.zone()` or
  `viewer_zone(cx)` (UTC when unset or signed out). New accounts start in
  the browser's zone: Register's hidden `time_zone` field is filled by
  `assets/js/time-zone.js` and stored if `Zone::parse` knows it. Emails use the
  recipient's zone. Users choose theirs on Settings (`preferences/`), with a
  "Use this device's time zone" button; never format a date with `time`
  directly.
