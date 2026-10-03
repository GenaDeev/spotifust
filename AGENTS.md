# AGENTS.md — Spotifust agent guide

Spotifust is a native Spotify desktop client in Rust: one binary, no web view, no JS runtime. This file is the shared instruction set for every coding agent (Claude Code, Codex/GPT, Cursor, …). It holds the rules that apply everywhere. Subsystem know-how lives in skills under `.agents/skills/` (see §8). Load the matching skill before you touch that subsystem.

If this file is ambiguous or out of date, fix it in the same change or log the gap in `TODO.md` → **Architectural Debt**. Don't guess silently.

## 0. Stack and layout

| Area | Choice | Where |
| :--- | :--- | :--- |
| UI | `iced` 0.14, **tiny-skia** (CPU) renderer, pure MVU | `src/app.rs` (model/update), `src/ui/` (views) |
| Audio | `librespot` (git, 0.8) → custom sink → `rodio` 0.21 | `src/audio/` |
| Web API | `rspotify` 0.16 (PKCE), `reqwest` 0.12 (rustls + http2) | `src/api/` |
| Secrets | OS keychain via `keyring` | `src/api/auth.rs`, `src/audio/credentials.rs` |
| Task list | `TODO.md` (state machine, see §3) | repo root |

Performance envelope: target **< 25 MB RSS at idle** (not met yet, see `TODO.md`) and a single process.

Terms used below:

- **Process**: an OS process. `tokio::spawn` tasks and threads are *not* processes, and they are expected.
- **Atomic task**: one `- [ ]` item in `TODO.md`'s backlog.

## 1. Core constraints

- **No web overhead.** No web views, embedded browsers or JS runtimes, not even for debugging.
- **Single process.** No `std::process::Command`, no sidecar binaries, no cross-process IPC. `librespot` and `rspotify` are linked in. Async work runs on `tokio::spawn` inside the process.
- **Elm rule (pure MVU).** UI state changes only through `Message` → `App::update`. Don't put `Rc<RefCell<_>>` or `Arc<Mutex<_>>` in UI-owned state. Background work talks to the UI through bounded `tokio::sync::mpsc` channels surfaced as `iced::Subscription`s. Immutable `OnceLock` caches (e.g. `ui::logo_handle`) are fine. One known exception, logged as debt: `AudioSession.events` is an `Arc<tokio::sync::Mutex<Receiver>>` so the subscription recipe can own it.
- **Derived side effects belong in the `App::update` wrapper.** It compares `panel_key`/`page_key` before and after `update_inner`, which is how lyrics/bio loading and scroll resets work. Extend that mechanism instead of sprinkling the same logic across message handlers.

## 2. Error-handling contract

- After the iced loop starts, `.unwrap()`, `.expect()` and `panic!()` are forbidden in non-test code. Fallible code returns `Result<T, AppError>`, and errors reach the user as `Message::ErrorEncountered(AppError)` or a toast.
- Bootstrap exception: in `main()`, before `iced::application(...).run()`, failing fast with `eprintln!` + `std::process::exit(1)` is acceptable. Keep that window minimal.
- `AppError` (`src/error.rs`, `thiserror`) has per-subsystem variants: `Auth`, `Playback`, `PlaybackPairing`, `Network`, `Cache`, `RateLimited`. Wrap third-party errors; never put `librespot`/`rspotify` error types in `Message`.
- `AppError::Auth` **logs the user out and clears the cache**. Only use it when credentials are truly invalid (HTTP 400/401 on refresh). Offline and 5xx are `Network`.
- Never use lock poisoning as control flow. `.lock().unwrap()` means shared state crept in; refactor to messages.

## 3. `TODO.md` protocol

`TODO.md` is the single source of truth for project state. The `spotifust-todo` skill has the full procedure. The essentials:

- **Session start:** if the user gave a concrete task, do it. Otherwise read `TODO.md`, summarize **Current Focus**, and ask whether to continue with it.
- **Finishing an atomic task:** tick it in the same change as the code. Then ask before starting the next item.
- **Only tick verified work.** An item is done when it works end to end and is wired into the UI or runtime, not when code for it merely exists. If a checked item turns out broken, untick it and add an indented `- Audit YYYY-MM-DD:` note saying what is missing.
- **New work found mid-task** goes into **Architectural Debt** immediately.
- **Editing:** re-read `TODO.md` right before editing and change only the relevant lines; never regenerate the file from memory. Keep the existing sections: Current Focus, Development Backlog (phases), Architectural Debt, Blocked / Needs Human Decision.

## 4. Subsystem rules (details in skills)

### A. UI (`src/app.rs`, `src/ui/`) — skill `spotifust-iced-ui`

- `view()` runs after **every** update, including ~4 playback ticks a second. Keep it cheap: no decoding, no network, no sorting large lists.
- Never create `image::Handle::from_bytes`/`from_path` inside `view()`. Each call mints a new id and forces a re-decode every frame. Store handles in the model (`loaded_images`) or a `OnceLock`.
- Long lists are virtualized (`virtual_rows` / `visible_row_range`). Don't nest a vertical scrollable inside another, and don't use horizontal scrollables for shelves (they swallow the mouse wheel; use `card_shelf`).

### B. Audio (`src/audio/`) — skill `spotifust-audio`

- The audio backend is `rodio`. Don't introduce `cpal` directly and don't propose switching.
- Playback needs credentials issued to Spotify's desktop (keymaster) client id. They come from the one-time device pairing (`spotify.com/pair`) and are stored in the keychain. Tokens from our own Web API client id fail login5 and every track reports `Unavailable`.
- Every PCM path is bounded: `sync_channel(8)` plus the rodio queue cap. `unbounded_channel` is forbidden here.
- The UI position comes from `PlaybackClock` (samples actually played), never from `PlayerEvent` `position_ms` or wall clocks.

### C. Web API (`src/api/`) — skill `spotifust-spotify-api`

- OAuth: Authorization Code **with PKCE**, our own client id, no client secret. The redirect is the `spotifust://callback` custom protocol, with no local ports. It is registered only on Linux today (`TODO.md`).
- Secrets live only in the OS keychain, under `api::auth::keyring_service()`. Unit tests automatically use a separate service and cache dir, so never hard-code `"spotifust"`.
- Spotify restricts development-mode apps: several endpoints return 403/404 and some fields are always empty. Check the table in the skill before building on an endpoint.
- Every request costs ~0.5 s from the developer's network. Batch (`search_multiple`), run independent requests concurrently, and cache.
- `src/api/cache.rs` is for metadata and images only, never credentials.

### D. Scripts and CI (`scripts/`)

- Developer, test and CI scripts live in `scripts/`. The exception is `install.sh`, which stays at the root for end users.

## 5. Forbidden patterns (scan your diff)

- `.unwrap()` / `.expect()` / `panic!()` outside tests and the §2 bootstrap exception.
- `Rc<RefCell<_>>` / `Arc<Mutex<_>>` in UI-owned state.
- `std::process::Command` or spawned binaries.
- `tokio::sync::mpsc::unbounded_channel` (or any unbounded queue) in the audio or decoder path.
- Plaintext tokens or secrets on disk.
- Raw `librespot`/`rspotify` error types in `Message` variants.
- Image handles built in `view()`; `.clone()`/`.to_string()` in the audio callback or other hot paths where a borrow works.
- UI that pretends: fake stats, placeholder data shown as real, buttons whose action is only a toast. Hide the element instead and log the gap in `TODO.md`.

## 6. Decisions that need a human

Add these to `TODO.md` → **Blocked / Needs Human Decision** instead of choosing yourself:

- Any new external dependency, or a new feature flag that pulls in new crates, not implied by the Tech Stack table in `README.md`. Enabling a feature whose crates are already in `Cargo.lock` is fine; say so in the PR.
- Architectural changes to the MVU data flow, the audio pipeline topology, the auth flows, or the §2 error contract.
- Anything that needs a third-party API key or account (Genius, Last.fm, …).

## 7. Pre-delivery self-check

Before you call a task done, run these and confirm they pass. Don't just assert it. The `spotifust-verify` skill has the details.

1. `rustup update stable`. CI runs the latest stable clippy, and new lints appear without warning.
2. `scripts/test.sh` (fmt check, clippy `-D warnings`, tests, deny, audit, typos, lychee).
3. `cargo build --release` (slow: LTO, ~6 min).
4. If any `.md` changed: `markdownlint-cli2 "**/*.md"`. Config: `.markdownlint-cli2.jsonc`.
5. Scan your diff for §5 patterns, and update `TODO.md` per §3.
6. For UI or audio changes, run the app (`cargo run --release`) and ask the user to confirm. There is no screenshot tooling for the native window. Don't claim visual results you haven't seen.

## 8. Skills

Skills live in `.agents/skills/<name>/SKILL.md`, which is the path Codex/GPT discover. `.claude/skills` is a symlink to the same directory for Claude Code. Agents load them automatically by `description`. You can also open the file directly.

| Skill | Use when |
| :--- | :--- |
| `spotifust-iced-ui` | Editing `src/app.rs` or `src/ui/`: views, layout, scrolling, images, performance, glitches |
| `spotifust-audio` | Editing `src/audio/`: playback, pairing, position or sync, memory of the PCM path |
| `spotifust-spotify-api` | Editing `src/api/`: endpoints, auth and tokens, latency, lyrics and bio providers, images |
| `spotifust-verify` | Before finishing any task, or when CI fails |
| `spotifust-todo` | Reading, ticking, auditing or extending `TODO.md` |

Precedence: if a skill conflicts with this file, this file wins. Fix the skill in the same change.

When you learn something non-obvious that the next agent would otherwise rediscover the hard way (an API quirk, a renderer pitfall, a debugging recipe), add it to the matching skill in the same PR.

## 9. Delivery style

- Idiomatic, strongly typed Rust. Prefer borrows (`&str`, `&[u8]`) over owned copies in hot paths.
- Match the surrounding code: comment density, naming, error style.
- Commits and PRs: Conventional Commits (`fix:`, `feat:`, `perf:`, `docs:`, …). PR bodies follow `.github/PULL_REQUEST_TEMPLATE.md`.
