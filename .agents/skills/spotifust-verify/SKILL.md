---
name: spotifust-verify
description: Pre-delivery verification for Spotifust: toolchain update, fmt, clippy, tests, release build, markdownlint, forbidden-pattern scan, running the app, and fixing CI failures. Use before declaring any task done, before committing or opening a PR, or when a CI check fails.
---

# Verify a Spotifust change

Run every step and read the output. CI ("Build and Test" on ubuntu, macos and windows) runs the same checks with the **latest stable** toolchain.

## 1. Toolchain

```bash
rustup update stable
```

New clippy lints land with each stable release, and CI picks them up immediately. Skipping this step made PRs pass locally and fail in CI (for example `clippy::assert_is_empty`).

## 2. Standard checks

```bash
scripts/test.sh
```

It runs `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`, and, when installed, `cargo deny check`, `cargo audit`, `typos` and `lychee`. If fmt fails, run `cargo fmt` and re-check. Formatting alone has failed CI before.

Clippy runs with `all` and `pedantic` at warn plus `-D warnings`. Fix lints rather than allowing them. A local `#[allow(...)]` needs a one-line reason comment, and only for lints that don't fit, such as `too_many_lines` on large views or `struct_excessive_bools` on flag structs.

## 3. Release build

```bash
cargo build --release
```

It takes ~5–6 min (LTO, `codegen-units = 1`). Run it in the background and keep working.

## 4. Markdown

When any `.md` file changed:

```bash
markdownlint-cli2 "**/*.md"
```

The config is `.markdownlint-cli2.jsonc`: MD013, MD033, MD041 and MD042 are off, and `target/` is ignored. Keep blank lines around headings, lists and fenced blocks, and give fenced blocks a language.

## 5. Diff scan

```bash
git diff main... | grep -nE '^\+.*(\.unwrap\(\)|\.expect\(|panic!\(|unbounded_channel|process::Command|Handle::from_bytes)'
```

Matches are fine only inside tests (or `Handle::from_bytes` outside `view()`). Also check `AGENTS.md` §5 by eye: shared mutable state in UI structs, raw third-party errors in `Message`, plaintext secrets, fake UI.

## 6. Running the app

- Build with `cargo build --release`, then run `./target/release/spotifust` in the background and send output to a log file.
- You can't see the window. Tell the user exactly what to check and ask them. Never report visual or audio behaviour you haven't confirmed.
- Memory: `ps -o rss=,%cpu= -C spotifust`, sampled over a minute of playback.
- First launch on a new machine may need the OAuth login and the one-time playback pairing (banner with a code).

## 7. Live checks (optional, real account)

```bash
SPOTIFUST_LIVE_KEYRING=1 cargo test -- --ignored --nocapture
```

These hit Spotify, LRCLIB and Wikipedia using the developer's stored login. They're read-only.

## 8. CI failure triage

1. `gh pr checks <n>` → `gh run view <run-id> --log-failed`.
2. Reproduce locally after `rustup update stable`.
3. Fix it, re-run steps 2–5, then commit and push. For conflicts, merge the base branch in; never rebase or force-push a shared PR.
