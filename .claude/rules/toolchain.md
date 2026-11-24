---
paths:
  - "rust-toolchain.toml"
  - "Cargo.toml"
  - "Cargo.lock"
  - ".cargo/**"
  - ".github/**"
  - "check.sh"
---

# Toolchain and dependencies

- Rust is pinned in `rust-toolchain.toml` (rustup installs it
  automatically). To upgrade: change it there, restart `topcoat dev`, run
  `cargo clean` once (old build files aren't reused), then `./check.sh`.
- The Topcoat CLI is `cargo install topcoat-cli --version 0.12.1` (it uses
  the crate's own lockfile); after upgrading it, restart `topcoat dev`.
- `sqlx-cli` is 0.9.0 locally and in CI (`.github/actions/setup`);
  change both together, or CI's `.sqlx` check fails on regenerated files.
- CI (`check.yml`) runs `./check.sh --part rust` and `--part e2e` as
  parallel jobs: Chromium only on push, all three browsers nightly (07:00
  UTC) and on manual runs. Shared setup lives in `.github/actions/setup`.
  A new `check.sh` step goes inside the `--part` it belongs to.
- `.cargo/config.toml` sets `global-min-publish-age = "3 days"`: Cargo won't
  pick a crate version newer than that (a hijacked release is usually caught
  within days). `Cargo.lock` versions are unaffected; to take a fresh fix
  sooner, run that one update with
  `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow`.
