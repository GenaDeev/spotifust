# Contributing to Spotifust

First off — thanks for even considering it. This project exists because heavy web-wrapped music clients annoyed someone enough to write a whole audio pipeline in Rust instead. If that same kind of stubbornness resonates with you, you're in the right place.

This document is the human-facing companion to [`AGENTS.md`](./AGENTS.md). `AGENTS.md` is the operating manual for AI coding agents working on this repo (Codex, Claude Code, Cursor, whatever you point at it). This file is for you. Read both — most of what makes a good human contribution here is the same thing that makes a good agent contribution: understanding *why* the constraints exist before working around them.

---

## Before you write any code

Skim [`README.md`](./README.md) for the architecture overview and [`AGENTS.md`](./AGENTS.md) for the hard constraints (§1–§5 especially). The three rules that will save you the most back-and-forth in review:

1. **No shared mutable state in UI structs.** If you're reaching for `Rc<RefCell<T>>` or `Arc<Mutex<T>>` inside anything UI-owned, stop — that's a sign the change should go through `Message` → `update()` instead.
2. **No `.unwrap()` / `.expect()` / `panic!()`** outside the narrow bootstrap exception in `AGENTS.md` §2. Every fallible path returns `Result<T, AppError>`.
3. **No new dependencies without discussion.** Open an issue first if you think the project needs a crate that isn't already in the Tech Stack table in the README.

If a PR violates one of these, it'll bounce back for rework rather than get merged with a promise to "fix it later" — that's how a 25MB RAM budget turns into 200MB over six months.

---

## Ways to contribute

You don't have to write Rust to help:

- **Bug reports.** Precise repro steps beat a big writeup. Include your OS, audio backend (`rodio` vs `cpal`), and whether it reproduces in `--release` or only in debug.
- **Feature requests.** Open an issue before a PR for anything non-trivial — see [Architectural decisions](#architectural-decisions-please-discuss-first) below.
- **Documentation.** README clarity, this file, code-level docs — always welcome, always lower-stakes to review than engine code.
- **Testing on platforms you own.** Windows, macOS, and Linux all touch different audio backends. If you can test on a platform the maintainer doesn't have, that's genuinely valuable signal.
- **Code.** See the workflow below.

---

## Development setup

```bash
git clone https://github.com/your-username/spotifust.git
cd spotifust
cargo build --release
```

Full prerequisites and environment variables are in the README's [Getting Started](./README.md#-getting-started) section — don't duplicate them here, that's how docs drift out of sync.

---

## Workflow

1. **Fork the repo and branch off `main`.** Branch names aren't strictly enforced, but `feature/short-description` or `fix/short-description` keeps the history legible.
2. **Check `TODO.md`.** This project tracks state as a checklist, not a changelog. If your change maps to an existing item under **Development Backlog**, reference it. If it's new work, add it under **Architectural Debt** before you start — this applies to you too, not just AI agents.
3. **Write the change.** Follow the module specs in `AGENTS.md` §4 if you're touching `src/ui/`, `src/audio/`, or `src/api/` — each has specific constraints (bounded channels, fixed OAuth port, keyring-only token storage) that aren't obvious from reading the code alone.
4. **Self-check before opening the PR** — the same checklist `AGENTS.md` §7 asks of an AI agent applies to you:
   - `cargo build --release` succeeds
   - `cargo clippy --all-targets -- -D warnings` passes clean
   - `cargo fmt` applied
   - No pattern from the Forbidden Patterns list (`AGENTS.md` §5) snuck in
   - `TODO.md` updated to reflect what actually got done
5. **Open the PR.** Describe *what changed* and, more importantly, *why* — a diff without reasoning is much slower to review, especially for anything touching the canvas layout or the audio pipeline.

---

## Architectural decisions — please discuss first

Some choices are deliberately left open in `AGENTS.md` §6 rather than decided unilaterally by whoever gets there first:

- `rodio` vs `cpal` as the final playback backend
- Changing the fixed OAuth loopback port
- Any new external dependency

If your PR touches one of these, open an issue before writing code. It's not bureaucracy for its own sake — the port decision alone breaks the registered Spotify redirect URI if changed casually, and dependency creep is the single fastest way to blow past the RAM budget this project exists to avoid.

---

## Contributing with an AI coding agent

This repo is built to be agent-friendly on purpose — `AGENTS.md` is the shared instruction file read natively by Codex, Cursor, and Claude Code, and `.agents/` vendors a Rust knowledge base your agent can route through (see `AGENTS.md` §8). If you're using one:

- Let it read `AGENTS.md` before generating anything — that's the whole point of the file existing.
- You're still responsible for the PR. "The agent wrote it" isn't a defense for a violated constraint in review; treat agent output the same as your own first draft.
- If your agent proposes something that conflicts with `AGENTS.md`, the document wins (see §8.4) — that's true for the agent and it's true for you.

---

## Code style

Not duplicated here — it's `AGENTS.md` §9 (Delivery Style), plus whatever `rustfmt` enforces automatically. The short version: idiomatic, strongly-typed, self-documenting through naming rather than comments, and borrows over allocations in hot paths.

---

## Reporting security issues

Two things are unusually sensitive in this project: OAuth token handling and the fact that it talks to a reverse-engineered protocol. **Do not open a public issue for a security concern.** Instead, reach out privately to the maintainer first (see the contact info in [`LICENSE`](./LICENSE)) so there's time to land a fix before it's public knowledge.

---

## Licensing note

Spotifust is licensed under [GPLv3](./LICENSE). By submitting a contribution, you agree it's provided under the same license — that's what keeps every fork of this project, including yours, staying open. If that's not something you're comfortable with, this probably isn't the right project to contribute code to (documentation and bug reports are a different story — those aren't code contributions in the copyright sense).

---

## Questions?

If something in this file, in `AGENTS.md`, or in the codebase itself doesn't make sense, open a discussion or an issue asking *why* it's structured that way before assuming it's wrong. Sometimes it is wrong and worth changing. Sometimes there's a reason that just isn't written down yet — in which case, the follow-up fix is documenting it, not silently working around it.
