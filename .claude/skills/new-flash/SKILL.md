---
name: new-flash
description: Add a success message shown after a redirect (a session flash rendered as a toast) in this app. Use whenever a POST route or procedure should show "Saved", "Sent", etc. on the page it redirects to.
---

# New flash message

Success messages after a redirect are session flashes (`src/flash.rs`), rendered as toasts.
Never put message flags in the URL.

## 1. Add the variant — `src/flash.rs`

Add a variant to `enum Flash` and an arm to both matches:

- `name()` → a unique kebab-case toast name (`"invoice-sent"`).
- `message()` → the user-facing sentence.

## 2. Set it before redirecting (POST route)

```rust
set_flash(&session(cx)?, Flash::InvoiceSent).await?;
Ok(see_other(TARGET_URL))
```

## 3. Accept and show it on the target page

The target page lists what it accepts in a page `const` and takes it once:

```rust
const FLASHES: &[Flash] = &[Flash::InvoiceSent];

let flash = take_flash(cx, FLASHES).await?;
let toast = signal(cx, || toast_name(flash).to_string());
```

and renders `flash_toasts(toast: &toast, flashes: FLASHES)` in the view
(see `src/user/pages/account_profile.rs`). If the page already has a `FLASHES` list, add the
variant to it; there's only one `toast` signal per page.

## 4. JS path (procedure inside a modal)

When a `#[procedure]` succeeds without a redirect, show the same toast from the handler.
Precompute the name before `view!` (`let sent_toast = Flash::InvoiceSent.name().to_string();`),
then in the success branch `toast.set(sent_toast.clone())` and close the dialog (see lines
around `saved_toast` in `account_profile.rs`).
Errors from the procedure render inside the dialog, not as a toast.

## 5. Finish

- Add or extend an e2e test that asserts the toast text is visible after the action, and that
  it does not reappear on refresh.
- Run `./check.sh`.
