# cargo-deny Policy Reference

Contents: [version boundaries](#version-boundaries), [`[advisories]`](#advisories),
[`[licenses]`](#licenses), [`[bans]`](#bans), [`[sources]`](#sources),
[CI job](#ci-job), [policy change checklist](#policy-change-checklist).

This reference explains the `deny.toml` sections that matter for supply-chain policy, and
the choices you must make in each. The examples need cargo-deny 0.19.0 or later. Keep
`deny.toml` next to the workspace `Cargo.toml`, and run the check from the workspace root.
From 0.19.0, a relative `--config` path resolves against the current directory. When no
file is there, cargo-deny warns `falling back to default config` and checks the default
policy instead of yours. Treat that warning as a failure. Run the check with cargo-deny 0.20
or later:

```bash
cargo deny --config deny.toml --locked check
```

On a pin before 0.20, `--config` goes after `check`. The table below lists the other
differences.

## Version boundaries

| Version | Change | Failure across the boundary |
|---|---|---|
| 0.19.0 | Adds the `[advisories]` keys `unsound` (a scope, default `workspace`) and `unused-ignored-advisory`. Resolves a relative `--config` path against the current directory, not the manifest directory (PR#802). | Do not add these keys for an older pin. A relative `--config` path that does not exist falls back to the default policy. |
| 0.19.1 | Enforces `maximum-db-staleness` (before, the limit was over 14 years, PR#833). Makes `--frozen` stop the advisory database fetch (PR#841). Fixes the `unsound` scope (PR#839). | On an older pin, an offline check never fails for a stale database, and `--frozen` still fetches. On 0.19.0 with the default `unmaintained`, `unsound = "workspace"` reports no unsound advisory and `"transitive"` reports all of them. |
| 0.20.0 | Moves `--config` and `--metadata-path` to the root command. Removes `check --disable-fetch` (use `--offline`) and the `ban` and `license` check aliases. | On 0.20, `check --config` fails with `unexpected argument '--config' found`. On 0.19, root `--config` fails with the same message. |

For a key or flag that neither this table nor the examples in this reference show, confirm
in the cargo-deny `CHANGELOG.md` that the pinned version has it.

## `[advisories]`

Controls RustSec advisory enforcement. A vulnerability advisory is always an error.

| Field | Value | Why |
|---|---|---|
| `yanked` | `"deny"` | The publisher withdrew the version. The default is `"warn"`. |
| `unmaintained` | `"all"` (default) | Fails on unmaintained crates anywhere in the graph. The other scopes are `workspace`, `transitive`, and `none`. |
| `unsound` | `"all"` | The default `"workspace"` fails only on direct dependencies of workspace crates. It misses an unsound crate that arrives transitively. Needs cargo-deny 0.19.0; the `workspace` and `transitive` scopes work as documented from 0.19.1 (PR#839). |
| `unused-ignored-advisory` | `"deny"` | An `ignore` entry fails the check after its advisory leaves the graph. The default `"warn"` lets stale entries pile up. Needs cargo-deny 0.19.0. |
| `ignore` | Empty by default | Every entry is a time-boxed exemption, not a policy. |

```toml
[advisories]
yanked = "deny"
unsound = "all"
unused-ignored-advisory = "deny"
ignore = [
    # Tracking: <issue-url>. Re-evaluate by <YYYY-MM-DD>.
    { id = "RUSTSEC-0000-0000", reason = "proc-macro only, compile-time, no runtime code path; no upstream fix published" },
]
```

The rules for an ignore entry and the default time box are in section 4 of
[SKILL.md](../SKILL.md). The reason must say why the advisory is safe in this workspace,
not why the advisory is low severity in general.

To ignore a yanked version, use the package-spec form:
`{ crate = "name@1.2.3", reason = "..." }`.

With `--offline`, cargo-deny does not fetch the database. From cargo-deny 0.19.1, it fails
when the cached database is older than `maximum-db-staleness` (default `"P90D"`). Set a shorter value, for example
`"P7D"`, when CI runs offline.

## `[licenses]`

Controls which SPDX licenses may appear in the dependency graph.

```toml
[licenses]
confidence-threshold = 0.8
unused-allowed-license = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "0BSD",
    "Zlib",
    "Unicode-3.0",
    "CDLA-Permissive-2.0",
]

[licenses.private]
ignore = true
```

- `confidence-threshold = 0.8` is the default. A lower value accepts weaker license-text
  matches. A higher value produces more manual review.
- `allow` is a graph-minimal allowlist. List only the licenses your current graph needs.
  An allowlist that lists unused licenses hides the moment a new license enters the tree.
  `unused-allowed-license = "deny"` fails the check when an allowlist entry has no user.
  The default `"warn"` only prints `license-not-encountered`. `CDLA-Permissive-2.0` is
  there for `webpki-roots`; remove it if the graph does not need it.
- `licenses.private.ignore = true` skips workspace members that declare `publish = false`.
  Third-party dependencies stay subject to the allowlist.

Licenses that need an explicit decision before you add them:

| License | Consideration |
|---|---|
| `Apache-2.0 WITH LLVM-exception` | Common in compiler-adjacent and codegen crates. Add it when the graph needs it. |
| `MPL-2.0` | Weak, file-level copyleft. Acceptable in many products, but record the decision. Some binding-generator crate families need it. |
| `OpenSSL` | Add it only when a TLS or crypto dependency needs it, and only after a license review. |

Do not widen the allowlist to clear one failing crate without a recorded license review.
Find the crate first:

```bash
cargo tree --locked -i <crate>
```

## `[bans]`

Controls duplicate versions, wildcard requirements, and named crate bans.

```toml
[bans]
multiple-versions = "deny"
wildcards = "deny"
allow-wildcard-paths = true
skip = [
    { crate = "some-transitive-crate@0.4.9", reason = "<crate-a> 1.x pins it and <crate-b> 2.x has moved on; unify after <crate-a> 2.0, tracking <issue-url>" },
]
```

The target state for `multiple-versions` is `"deny"` with reviewed `skip` entries. With
`"warn"`, a new duplicate looks exactly like the twenty existing ones, and nobody sees it.
Use `"warn"` only during an initial cleanup, then move to `"deny"`.

Write each `skip` entry in the package-spec form, with an exact version and a `reason`.
`crate = "name@0.4.9"` matches only `=0.4.9`. A range-based skip silently covers future
versions that nobody reviewed. The old `{ name = "...", version = "..." }` form is
deprecated. cargo-deny warns with `unmatched-skip` when an entry no longer matches the
graph. Make that an error with
`cargo deny --config deny.toml --locked check -D unmatched-skip bans`.

`wildcards = "deny"` blocks `version = "*"` requirements. A wildcard requirement accepts
any future release, including a compromised one. Set `allow-wildcard-paths = true` as well.
Without it, every internal `path` dependency that has no `version` fails the check with
`error[wildcard]`. The exemption covers path and git dependencies of `publish = false`
crates and dev-dependencies. A published crate still fails, because crates.io rejects such
dependencies.

To find the cause of a duplicate, run `cargo tree --locked -d` and
`cargo tree --locked -i <crate>@<version>`. The `highlight` field only colors the dot graph
that `check -g <dir>` writes.

Use the `deny` list to forbid a specific crate. An entry takes `crate`, `reason`,
`wrappers`, and `use-instead`. Explicit lists express policy more precisely than duplicate
detection alone.

## `[sources]`

Controls where crates may come from.

```toml
[sources]
unknown-registry = "deny"
unknown-git = "deny"
required-git-spec = "rev"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

- `unknown-registry = "deny"` blocks any registry that `allow-registry` does not list.
  The default is `"warn"`.
- `unknown-git = "deny"` blocks git dependencies from remotes that `allow-git` does not
  list. Use `"warn"` only while the graph still has unreviewed git sources.
- `required-git-spec = "rev"` rejects a git dependency pinned to a branch, a tag, or
  nothing. A branch pin is unpinned code execution, and a tag can move. The default
  `"any"` allows all of them.

A `sources` failure often means an accidental `[patch]` section, a deliberate `[patch]`
whose remote is missing from `allow-git`, a local `path` override that leaked into a
commit, or a git dependency that a transitive crate introduced. Check
the manifests before you change the policy.

## CI job

Run the policy check as its own job, so a failure names its cause without a log hunt.

1. Check out the repository.
2. Install the exact pinned `cargo-deny` version with a pre-built installer. On GitHub
   Actions, use `taiki-e/install-action@<full-commit-sha> # v2.x.y` with
   `tool: cargo-deny@<version>`. Pin every action to a full commit SHA with a version
   comment. A policy can require SHA pins, and then a floating tag fails.
3. Run `cargo deny --config deny.toml --locked check`.

Run the job on pull requests, on pushes to the default branch, and on a schedule, because
new advisories land against unchanged lockfiles. Without `--offline`, cargo-deny fetches
the advisory database on every run, so a cache of `$CARGO_HOME/advisory-dbs` only saves
clone time. With `--offline`, the check fails when the cached database is older than
`maximum-db-staleness` (see [`[advisories]`](#advisories)).

## Policy change checklist

Before you commit a `deny.toml` change:

1. Run the full check with the CI pin, and read the whole output, not only the exit code.
2. Confirm that the CI pin supports every key you added. Use the
   [version table](#version-boundaries). Check the cargo-deny `CHANGELOG.md` only for a key
   that neither the table nor the examples here show.
3. Confirm that every new `ignore` entry has an `id`, a `reason`, a tracking link, and a
   re-evaluation date.
4. Confirm that every new `skip` entry has an exact version and a causal `reason`.
5. Confirm that no allowlist got wider without a recorded review.
