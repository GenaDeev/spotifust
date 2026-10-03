---
name: spotifust-todo
description: How to read, update and audit Spotifust's TODO.md state machine. Use at session start when the user hasn't given a concrete task, when finishing or ticking a backlog item, when logging newly discovered work, or when asked to audit or re-plan the backlog.
---

# TODO.md protocol

`TODO.md` is the project's state machine and the only memory that carries across agent sessions.

## Structure

```markdown
# Project State Machine

## Current Focus
- [ ] <the one backlog item being worked on, verbatim>

## Development Backlog
### Phase N: <name>
- [ ] / - [x] items (one item = one atomic task)

## Architectural Debt
- [ ] work discovered mid-task that isn't in the backlog

## Blocked / Needs Human Decision
- [ ] decisions agents must not make alone (AGENTS.md §6)
```

## Rules

1. **Re-read before editing.** Open `TODO.md` right before every edit, and change only the lines involved. Never regenerate the file from memory: items get lost.
2. **Tick only verified work.** Done means it works end to end and is reachable in the running app. Code that exists but isn't wired up (e.g. `src/api/updater.rs` with no caller), settings that persist but have no effect, and features that rely on fake data are **not** done.
3. **Tick in the same change** as the code that completes the item. Then ask the user before starting the next item.
4. **Partial or broken items** stay unticked, with an indented note:

   ```markdown
   - [ ] Item text
     - Audit 2026-10-03: what works, what is missing or broken, and where (file/function).
   ```

5. **New work** found while doing something else goes into **Architectural Debt** right away, as a one-line item with enough context (files, symptom) for a cold reader.
6. **Human decisions** (new dependencies, architecture or auth changes, third-party API keys) go into **Blocked / Needs Human Decision** with the options.
7. **Current Focus** holds exactly one item, copied verbatim from the backlog.
8. Edit at most once per completed item. Don't rewrite `TODO.md` after every compile.

## Auditing the backlog

When asked to "be honest about the TODO":

- For each `[x]`, find the code path and confirm it's reachable: grep for callers, check the message wiring in `src/app.rs`, check that settings are loaded at startup and applied, not just saved.
- Known traps: persisted toggles hard-coded at startup (`autoplay_enabled: true`), settings that need an audio reconnect, endpoints that are 403/404 for development-mode apps (see the `spotifust-spotify-api` skill), and platform-specific registration (the protocol handler exists only on Linux).
- Don't fix anything during an audit unless asked. Untick and annotate.
- Run `markdownlint-cli2 TODO.md` afterwards.
