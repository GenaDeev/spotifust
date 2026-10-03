# Vendored skills

These skills are copied unmodified from
[po4yka/rust-skills](https://github.com/po4yka/rust-skills) at commit `9f4a3a4`
(2026-09-25), under the BSD 3-Clause license (`LICENSE-po4yka-rust-skills`).

| Skill | Why it's here |
| :--- | :--- |
| `rust-performance` | Measuring memory and CPU (DHAT, heaptrack, flamegraph, cargo-bloat) for the < 25 MB target |
| `rust-hot-path` | Cutting allocations in hot paths a profile already names (audio sink, `view()`) |
| `rust-async-internals` | tokio `select!`, `spawn_blocking`, shutdown and cancellation in the audio session task |
| `cargo-workflows` | Cargo profiles, features, lockfile and CI hygiene |
| `rust-security` | `cargo deny` / `cargo audit` triage, crate vetting |

Notes:

- `AGENTS.md` takes precedence. When these skills suggest a crate the project doesn't use yet (`tokio-util`, `tracing`, `criterion`, …), that's a dependency decision for a human (`AGENTS.md` §6).
- They mention other upstream skills (`rust-unsafe`, `rust-observability`, Android/iOS builds, …) that aren't vendored here. Fetch them from upstream if a task really needs one.
- Mobile-specific references (`rust-performance/references/android-profiling.md`, `ios-profiling.md`) don't apply to Spotifust, but are kept so the copy stays identical to upstream.
- They're excluded from markdownlint and typos (`.markdownlint-cli2.jsonc`, `.typos.toml`) so they stay byte-identical to upstream. To update, copy the directories again from a newer upstream commit and update the commit above.
