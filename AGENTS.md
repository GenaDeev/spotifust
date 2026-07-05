# AGENTS.md — AI Agent Operating Manual

Welcome, Agent. You are tasked with developing and maintaining **Spotifust**. These instructions are deterministic and take precedence over generic best-practice defaults you might otherwise apply. Ambiguity in this document is a bug — if you find a case it doesn't cover, add a clarifying note to `TODO.md` under **Architectural Debt** instead of guessing.

Target performance envelope: **< 25MB RAM baseline**, single binary, zero external runtime dependencies (no Node, no bundled browser engine).

---

## 0. Definitions (read before anything else)

To avoid the two most common failure modes — treating an async task as a process, and treating every keystroke as a "task" — the following terms are fixed for the rest of this document:

| Term | Means | Does NOT mean |
| :--- | :--- | :--- |
| **Process** | A separate OS-level process (`std::process::Command`, a sidecar binary, a spawned executable) | A `tokio::spawn`ed async task — those are explicitly required (see §5.B) and run inside the same process/address space |
| **Atomic task** | One checklist item (`- [ ]`) as it appears in `TODO.md`'s Development Backlog | A single line of code, a single function, or a single file edit |
| **Phase boundary** | Completion of every item within a numbered Phase in `TODO.md` | Reaching a compilable intermediate state mid-phase |

---

## 1. Core Operating Constraints

* **No Web Overhead.** Absolute prohibition of web-views, embedded browser engines, or any JS runtime, including for debugging/devtools purposes.
* **Single-Process Monolith.** No `std::process::Command`, no sidecar binaries, no IPC across process boundaries. `librespot` and `rspotify` are compiled directly into the application. `tokio::spawn` for async tasks inside the same process is not only allowed but required — see §5.B.
* **The Elm Rule (Pure MVU).** All UI-relevant state changes flow through `Message` → `update()`. Do not introduce `Rc<RefCell<T>>` or `Arc<Mutex<T>>` inside UI-owned structs to shortcut this. Cross-thread communication (e.g., audio task → UI) must go through `tokio::sync::mpsc` channels surfaced as `iced::Subscription`s, never through shared mutable state.
* **Zero Crashing Policy — with one explicit exception.** See §2.

---

## 2. Error Handling Contract

* **Inside the running application (after the `iced::Application` loop has started):** `.unwrap()`, `.expect()`, and `panic!()` are forbidden. Every fallible operation returns `Result<T, AppError>`, and errors are surfaced to the user via `Message::ErrorEncountered(AppError)`.
* **Bootstrap exception (before the event loop exists):** in `main()`, prior to `iced::Application::run()`, fail-fast with a clear `eprintln!` and `std::process::exit(1)` is acceptable — there is no UI yet to route an error message to. Keep this window as small as possible; move config/env loading into the update loop where feasible instead of expanding this exception.
* **Define one central error type.** Use `thiserror` for `AppError`, with variants per subsystem (`AppError::Auth`, `AppError::Playback`, `AppError::Network`, `AppError::Cache`). Do not let raw `librespot` or `rspotify` error types leak into `Message` variants — wrap them.
* **Never use `Mutex`/`RwLock` poisoning as a control-flow signal.** If you find yourself calling `.lock().unwrap()`, that's a sign shared-state locking crept in where it shouldn't have (see the Elm Rule) — refactor to message passing instead of handling the poison case.

---

## 3. `TODO.md` Synchronization Protocol

`TODO.md` is the single source of truth for project state. Treat it as a state machine, not a changelog.

### 3.1 When to touch it

* **On session start:** if `TODO.md` doesn't exist, create it using the schema in §3.3.
* **On completing one atomic task** (per the §0 definition — one checklist item): mark it `[x]` in the same edit that completes the corresponding code change. Don't batch multiple completed items into a single later rewrite.
* **On discovering new work mid-task** (a missing edge case, a follow-up refactor): append it to **Architectural Debt** immediately, don't just remember it — you won't carry memory into the next session.
* **Not on every intermediate compile or every function written.** If you're rewriting `TODO.md` more than once per checklist item, you're over-triggering this rule.

### 3.2 How to update it safely

Always re-read `TODO.md` immediately before editing it, even if you wrote it earlier in the same session — do not trust a stale in-context copy. Edit surgically (only the relevant checkbox/section); don't regenerate the whole file from memory, since that risks silently dropping items you didn't fully recall.

### 3.3 Mandatory schema

```markdown
# Project State Machine

## Current Focus
- [ ] Single atomic task currently in progress (must match one item below verbatim)

## Development Backlog

### Phase 1: Bootstrapping & Core Architecture
- [x] Configure Cargo.toml with feature flags for Iced (tiny-skia backend to save RAM), RSpotify, and Librespot
- [ ] Define central `AppError` enum (thiserror) with per-subsystem variants
- [ ] Set up base Model-View-Update loop in `src/app.rs`

### Phase 2: Custom Canvas Layout Engine
- [ ] Implement bounding-box tracking struct for responsive cards
- [ ] Handle PointerPressed / Moved / Released inside `canvas::Program::update`
- [ ] Wire `canvas::Cache` invalidation to interaction messages only

### Phase 3: Audio & Session Pipeline
- [ ] Spawn librespot session on tokio::spawn, bridged via mpsc → iced::Subscription
- [ ] Wire rodio/cpal sink for decoded PCM playback

### Phase 4: Web API & Auth
- [ ] Implement PKCE flow with fixed-port loopback TcpListener
- [ ] Store refresh token via OS keychain (not plaintext)

## Architectural Debt
- [ ] (Anything discovered mid-implementation that isn't yet in the backlog above)

## Blocked / Needs Human Decision
- [ ] (Anything that isn't safe for the agent to decide unilaterally — see §6)
```

---

## 4. Module-by-Module Specifications

### A. Card Canvas System (`src/ui/canvas_view.rs`)

* Implement a struct satisfying `iced::widget::canvas::Program`. Cards are geometric entities drawn into a single canvas buffer — never real OS windows.
* Card state (`position`, `size`, `dragging: bool`, `hovered: bool`) lives in the central `Model`, not inside the canvas program struct itself, per the Elm Rule.
* Use `canvas::Cache` and invalidate it **only** on messages that actually change geometry (drag/resize/add/remove). A hover-only redraw should not bust the cache if it doesn't change layout.

### B. Audio & Session Pipeline (`src/audio/`)

* Run the Spotify session on `tokio::spawn` — this is a task within the same process, not a violation of §1's process rule.
* Bridge the async task to the UI exclusively via bounded `tokio::sync::mpsc` channels, exposed to `iced` as a `Subscription`. No shared state.
* Feed decoded PCM arrays directly into a `rodio::Sink` or `cpal` stream, bypassing any intermediate buffering layer that would add latency versus the system mixer.
* Bound the channel (`mpsc::channel(N)`, not `unbounded_channel`) — an unbounded channel between a fast producer (decoder) and a slower consumer (UI) is a memory-growth footgun that directly threatens the 25MB baseline.

### C. Web API Layer (`src/api/`)

* Implement `rspotify` Authorization Code Flow **with PKCE** — never the implicit or plain Authorization Code flow (no client secret should ever be required for the desktop app's own auth).
* Intercept the OAuth callback via a **custom protocol handler** (e.g., `spotifust://callback`) registered at the OS level by an installer, guaranteeing cross-platform compatibility without relying on local open ports. Ensure the URL scheme is uniquely identifiable.
* Extract the auth code directly from the incoming OS invocation arguments.
* Persist the refresh token via the OS credential store (`keyring` crate: Credential Manager / Keychain / Secret Service) — never as plaintext in `src/api/cache.rs` or any repo-adjacent file.
* `src/api/cache.rs` is for metadata/image caching only, not credentials.

---

## 5. Forbidden Patterns (quick scan before any commit)

* [ ] No `.unwrap()` / `.expect()` / `panic!()` outside the bootstrap exception in §2
* [ ] No `Rc<RefCell<T>>` or `Arc<Mutex<T>>` inside UI-owned structs
* [ ] No `std::process::Command` / spawned sidecar binaries anywhere
* [ ] No `tokio::sync::mpsc::unbounded_channel` for audio-to-UI or decoder pipelines
* [ ] No plaintext token/secret storage
* [ ] No raw third-party error types (`librespot::Error`, `rspotify::ClientError`) exposed directly in `Message` variants — wrap in `AppError`
* [ ] No `.clone()` / `.to_string()` inside canvas render loops or audio callback hot paths where a borrow (`&str`, `&[u8]`) would do

---

## 6. Things the Agent Should NOT Decide Alone

Add these to **Blocked / Needs Human Decision** in `TODO.md` instead of picking silently:

* Choosing between `rodio` and `cpal` as the final playback backend (both are listed as acceptable — pick one per-project, not per-file, and flag the choice for confirmation the first time it comes up).
* Adding any new external dependency not already implied by the Tech Stack table in `README.md`.

---

## 7. Pre-Delivery Self-Check

Before declaring an atomic task complete, confirm all of the following — don't just assert it, actually verify:

1. `cargo build --release` succeeds.
2. `cargo clippy --all-targets -- -D warnings` passes clean.
3. The diff contains none of the patterns in §5.
4. `TODO.md` reflects the completed item as `[x]` and any newly discovered work is logged under Architectural Debt.
5. If the task touched `src/audio/` or `src/api/`, re-confirm the channel is bounded (§5.B) and no secret is written to disk in plaintext.
6. If any `.md` files were modified, ensure they strictly follow `markdownlint` rules (e.g., blank lines around fenced code blocks and headings). Do not introduce any formatting violations.

---

## 8. Rust Knowledge Base (`.agents/`)

This repository vendors the [actionbook/rust-skills](https://github.com/actionbook/rust-skills) knowledge base directly under `.agents/`. These are plain Markdown files — no plugin runtime, no proprietary loader. **Any agent capable of reading a file and following a pointer can use them**, whether that's Codex, Claude Code, Cursor, or anything else that clones this repo. Treat this section as mandatory routing, not optional reading.

### 8.1 Entry point — always start here

Before writing or modifying any `.rs` file, read `.agents/rust-router/SKILL.md` first. It classifies the task into one of the `m01`–`m15` categories below and tells you which specific skill file to open next. **Do not jump straight to a specialized skill on your own** — the router exists precisely because guessing the right category from a vague task description is what causes agents to load the wrong context and give confidently wrong answers.

### 8.2 Category → Spotifust subsystem map

Use this table to translate a task to the right skill without waiting for a prompt to spell it out:

| When touching... | Consult | Why it matters here |
| :--- | :--- | :--- |
| `src/ui/canvas_view.rs`, bounding-box structs | `m01-ownership`, `m03-mutability` | Card state lives in `Model`, not in the canvas struct (§1 Elm Rule) — ownership mistakes here are exactly what reintroduce `Rc<RefCell<T>>` by accident |
| Any `Result<T, AppError>` / `thiserror` work | `m06-error-handling` | Cross-check against §2 of this document before inventing a new `AppError` variant — this document's error contract wins on conflict (see §8.4) |
| `src/audio/` (tokio::spawn, mpsc channels) | `m07-concurrency`, `domain-web` (for the session's network calls) | Bounded-channel and cross-task patterns directly affect the 25MB baseline (§4.B) |
| Any `unsafe` block, FFI into `librespot`/OS audio APIs (`cpal`, `rodio` backends) | `unsafe-checker` (full checklist tree: `checklists/`, `rules/ffi-*`, `rules/mem-*`, `rules/ptr-*`) | This is the highest-stakes category in the whole knowledge base — an `unsafe` mistake here is a memory-safety bug shipped in a desktop binary, not a linter warning |
| `src/api/` (rspotify, PKCE, TcpListener) | `domain-web` | Covers REST/OAuth-shaped concerns generically; still defer to §4.C of this document for the Spotifust-specific fixed-port and keyring rules |
| Anything with `Box<dyn Trait>`, generics, trait bounds | `m05-type-driven`, `rust-trait-explorer` | Relevant when abstracting over `rodio`/`cpal` backends behind a common playback trait |
| Before adding or bumping any crate version | `rust-learner` + `core-dynamic-skills` | Verifies current crate versions/APIs instead of relying on training-data memory — required before touching any dependency per §6 |
| Reviewing your own diff before calling a task done | `m15-anti-pattern`, `coding-guidelines/clippy-lints` | Run this *in addition to*, not instead of, the checklist in §7 |
| Genuinely stuck on which category applies | `meta-cognition-parallel` | Fans out across candidate skills in parallel rather than guessing serially |

### 8.3 What NOT to do with this knowledge base

* Don't dump the entire contents of a `SKILL.md` and its subfolders into context "just in case." Read the router's classification, then open only the specific file(s) it points to.
* Don't treat `m01`–`m15` as a checklist to run sequentially on every task — they're a lookup table, not a pipeline.
* Don't use `core-agent-browser` / `core-actionbook` to fetch live documentation for anything covered by this repository's own `AGENTS.md` (architecture, module layout, error contract) — those are internal decisions, not upstream API facts, and this document is authoritative for them.

### 8.4 Precedence

If anything in `.agents/*/SKILL.md` conflicts with a rule stated elsewhere in this `AGENTS.md` (the Elm Rule, the error contract in §2, the module specs in §4, the Forbidden Patterns in §5), **this document wins**. The knowledge base teaches general Rust competence; it does not know Spotifust's specific architectural constraints, and it was not written with this project in mind.

---

## 9. Delivery Style

* Idiomatic, strongly-typed Rust. Prefer borrows (`&str`, `&[u8]`) over owned allocations (`.to_string()`, `.clone()`) in hot paths, per §5.
