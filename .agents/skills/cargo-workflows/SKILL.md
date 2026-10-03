---
name: cargo-workflows
description: Use when changing how Cargo builds, resolves, or tests a Rust workspace - Cargo.lock and --locked policy, dependency updates and workspace inheritance, feature-unification surprises, resolver 3 and MSRV, rust-toolchain.toml, Cargo profiles, cargo nextest config, GitHub Actions pinning and caching, cdylib or staticlib builds with cargo rustc, .cargo/config.toml for cross targets, driving cargo from a host build system, or an edition migration with cargo fix --edition. Not for Android or iOS packaging; use rust-android-build or rust-ios-build.
license: BSD-3-Clause
---

# Cargo Workflows

Run every cargo command from the directory that holds the workspace `Cargo.toml`. Cargo reads
`.cargo/config.toml` from the current directory upward, so a command run inside a member can
build with different settings.

## `Cargo.lock` and `--locked`

Commit `Cargo.lock` for every package, libraries included. `cargo new` tracks it by default, and
every `--locked` command needs it. Pass `--locked` on every cargo command that CI, a host build
system, or an agent runs. Without it, a build silently resolves new versions and stops being
reproducible. Omit it only in a command whose purpose is to change the lock: `cargo update`,
`cargo add`, the `cargo metadata` resolve in the triage table below, and the scheduled
latest-dependencies job.

Change the lockfile only on purpose, in a change that says why:

| Task | Command |
|------|---------|
| Move one dependency | `cargo update -p <dep>` or `cargo update -p <dep> --precise <version>` |
| Preview a full update | `cargo update --dry-run` |
| Full update within the semver ranges | `cargo update`, as a dedicated change |

`cargo generate-lockfile` rebuilds an existing lockfile with the latest version of every package.
It is a full update, not a repair. Do not run it to fix a `--locked` failure.

| Symptom | Cause | Fix |
|---------|-------|-----|
| `cannot create the lock file ... because --locked was passed to prevent this` | `Cargo.lock` is not in the checkout | Commit it. `git ls-files --error-unmatch Cargo.lock` confirms that git tracks it. |
| `cannot update the lock file ... because --locked was passed to prevent this` | A manifest edit needs a lock change | Resolve without building: `cargo metadata --format-version 1 > /dev/null`. Review the lock diff, and vet each new package name (the `rust-security` skill, when it is installed). Commit the lock with the manifest, then rerun the original command with `--locked`. |
| A lock diff touches many unrelated packages | `cargo update` or `cargo generate-lockfile` ran | Revert the lock change and run `cargo update -p <dep>`. |

Never repair the lock by running any command that compiles (`cargo build`, `check`, `test`,
`clippy`, `doc`, `run`, `nextest`) without `--locked`. That command compiles the new packages and
runs their build scripts and proc macros before anyone vets them.

A committed lockfile does not reach the users of a library: they resolve from `Cargo.toml`. Add a
scheduled CI job that runs `cargo +stable update` and then the tests on current stable, and let it
report, not block merges. This job builds and runs packages that nobody vetted, build scripts and
proc macros included. Run it on a GitHub-hosted runner, not on a self-hosted runner. On a
self-hosted runner, a build script can read the credentials of that machine, reach the internal
network, and stay on the machine after the job. Give it no secrets and
`permissions: { contents: read }`, and do not let it save a cache (`save-if: false` on
`Swatinem/rust-cache`). Vet each new package name in its lock diff before you adopt the update.

Read [references/workspace-patterns.md](references/workspace-patterns.md) when you write this job,
lay out a workspace, add or pin a dependency, add a member crate, or review a `Cargo.lock` diff.

## Verification

Iterate with the narrowest check. Run the workspace gate once before commit or merge. Add the
feature, cross-target, and MSRV rows when the change touches features, `cfg`-gated code,
dependencies, or manifests.

| Claim | Check | A green result does not prove |
|-------|-------|-------------------------------|
| The crate type-checks | `cargo check --locked -p <crate>` | Codegen, linking, or monomorphization |
| The workspace builds and links | `cargo build --locked --workspace` | Other targets or feature sets |
| Lints pass | `cargo clippy --locked --workspace --all-targets -- -D warnings` | Unselected features and targets; rustdoc lints |
| Doc links resolve | `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --document-private-items` (the `rust-lints` skill owns this gate) | Doc-test behaviour |
| Tests pass | `cargo nextest run --locked --workspace` and `cargo test --locked --workspace --doc` | Filtered-out profiles and targets |
| Formatting | `cargo fmt --check` | Anything semantic |
| Each feature set builds | The matrix in the Feature flags section | Unlisted combinations |
| A shipping target compiles | `cargo check --locked --target <triple>` | Linking, or any test on that target |
| Target tests build | `cargo test --locked --no-run --target <triple>` | That any test passes on the target: report the lane as compile-only |
| The MSRV holds | `cargo +<msrv> test --locked --workspace` | Newer dependency versions |
| Newest dependencies work | The scheduled latest-dependencies job | The committed lock |

Pass `--all-targets` to clippy. Without it, clippy skips tests, benches, and examples, and those
files then fail in CI on a lint that never showed locally.

## CI

```yaml
# Pin each action to a full commit SHA. Keep the version in a comment.
- uses: Swatinem/rust-cache@<full-commit-sha>   # v2.9.0 or later (node24)
  with:
    cache-on-failure: true
    workspaces: "<workspace-dir> -> target"

# Manual cache, when you need control over the key
- uses: actions/cache@<full-commit-sha>          # v5 or later (node24)
  with:
    path: |
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
      <workspace-dir>/target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('<workspace-dir>/rust-toolchain.toml', '<workspace-dir>/Cargo.lock') }}
```

- Pin every action to a full commit SHA, with the version in a comment. A tag can move to new
  code.
- GitHub-hosted runners removed Node 20 on 2026-09-23. Use the node24 majors: `actions/checkout`
  v5+, `actions/cache` v5+, `actions/upload-artifact` v6+. `Swatinem/rust-cache` runs on node24
  from v2.9.0; v2.8.x and older use node20 or node16.
- Set `cache-on-failure: true`. A failed job still compiled dependencies, and the next run can
  reuse them.
- Never expose secrets to a job that writes a cache that pull requests can read. Tools can copy
  the environment into `target/`: until the 2026-09-22 nightly, `cargo miri` stored every
  environment variable there ([Rust security advisory, 2026-09-21](https://blog.rust-lang.org/2026/09/21/github-actions-leaking-secrets-when-miri-output-is-cached/)).
  Scope a secret to one step that does not run cargo or Miri into the cached `target/`, or do not
  cache `target/` in that job. Clear the cache and rotate the secret if a cached job had one.
- Do not set `RUSTFLAGS=-Dwarnings`. A `RUSTFLAGS` change rebuilds every dependency, and the
  variable replaces the per-target `rustflags` in `.cargo/config.toml`. Use
  `CARGO_BUILD_WARNINGS=deny` (Cargo 1.97+) or clippy's `-- -D warnings`. The `rust-lints` skill
  owns the lint gate.

## Toolchain, MSRV, and resolver

```toml
# rust-toolchain.toml. Example pin: use the release the project has tested.
[toolchain]
channel = "1.98.1"
components = ["rustfmt", "clippy"]
```

- Pin an exact release. Do not pin 1.98.0: it can emit a trait-object vtable with a null function
  pointer, and 1.98.1 fixes it.
- In CI, run `rustup toolchain install` with no arguments before the first cargo command. It
  installs the toolchain that `rust-toolchain.toml` names (rustup 1.28+), so the install is a
  logged step and not an implicit side effect. rustup 1.29.1 warns about implicit installs.
- Set `rust-version` in `[workspace.package]` and inherit it with `rust-version.workspace = true`.
  Clippy reads it and does not suggest newer APIs. Do not set `msrv` in `clippy.toml`: when the
  two differ, Clippy warns and uses `clippy.toml`. The `rust-lints` skill owns that file.
- `rust-version` is a declaration, not proof. Resolver 3 prefers dependency versions that support
  it, but picks an incompatible version when no compatible version satisfies the requirement.
  Prove the MSRV with the real toolchain: run `rustup toolchain install <msrv>`, then
  `cargo +<msrv> test --locked --workspace`. The `+<msrv>` override beats `rust-toolchain.toml`.
  When a dev-dependency needs a newer Rust than the MSRV, run
  `cargo +<msrv> check --locked --workspace --lib --bins` instead. When cargo-hack is installed,
  `cargo hack check --rust-version --workspace --ignore-private` does the same per package.
- Do not let a Cargo older than 1.96.1 download from a third-party registry or over SSH git. It has
  credential-leak, crate-extraction, and libssh2 CVEs; the `rust-security` skill has the list.
  Read [references/workspace-patterns.md](references/workspace-patterns.md) when the MSRV is below
  1.96.1 and the workspace uses such a source: it has the offline vendor procedure.

Resolver rules:

- Edition 2024 implies resolver `"3"` (MSRV-aware, Rust 1.84+) in a package manifest.
- A virtual workspace has no edition. Set `resolver = "3"` in its `[workspace]` table. Without it,
  Cargo falls back to resolver `"1"` and prints a `virtual workspace defaulting to` warning.
- Use `"2"` only when the workspace must build with Cargo older than 1.84.

## Feature flags

- Features are additive. Once any crate in a build enables a feature, it is on for every consumer
  in that build. Never use a feature to remove behaviour.
- Use `dep:<name>` in a feature list. A bare optional dependency name creates an implicit public
  feature with the same name.
- Add `required-features` to a `[[bin]]` that needs an optional dependency. Otherwise
  `cargo build --workspace` tries to build it and fails.
- If a dependency's default features change the output bit-for-bit, pin the feature set and write
  down why. A GPU or SIMD backend on a crate that must produce byte-identical output breaks
  reproducibility.
- Do not model loom as a Cargo feature. Loom code builds under `--cfg loom`; the `rust-test-tools`
  skill has the setup.
- Test the project-owned matrix: default features, `--no-default-features`, and each supported
  feature set. Use `--all-features` only when the features are additive and the combination is a
  supported product. When cargo-hack is installed,
  `cargo hack check --locked -p <crate> --each-feature --exclude-all-features` runs one feature
  at a time.

Read [references/feature-resolution.md](references/feature-resolution.md) when a crate compiles
features it did not ask for, a build passes with `-p <crate>` but fails with `--workspace`, or an
inherited dependency rejects or ignores `default-features = false`.

## Testing with cargo-nextest

When cargo-nextest is installed, use it as the main runner. It runs each test in its own process,
which isolates crashes and gives per-test timeouts. Without it, use `cargo test --locked`.

```bash
cargo nextest run --locked --workspace
cargo nextest run --locked --profile ci
cargo nextest run --locked -p <crate> --test <integration-test>
cargo test --locked --workspace --doc
```

Run `cargo test --doc` as its own step. nextest skips doc-tests, so a nextest-only gate lets
doc examples rot.

Example `.config/nextest.toml`:

```toml
# Fail loudly on a nextest too old for the keys below.
nextest-version = { required = "0.9.131" }

[profile.default]
fail-fast = true
slow-timeout = { period = "60s" }
# Keep the opt-in tests out of the default lane.
default-filter = 'not test(/^network_integration_/)'

[profile.ci]
fail-fast = false
retries = 2
flaky-result = "fail"
slow-timeout = { period = "60s", terminate-after = 3 }

# Opt-in profile for tests that touch the network.
[profile.network-integration]
default-filter = 'test(/^network_integration_/)'

[test-groups]
network = { max-threads = 1 }

[[profile.network-integration.overrides]]
filter = 'test(/^network_integration_/)'
test-group = 'network'
```

- Keep network-dependent or otherwise flaky tests behind an opt-in profile and a single-threaded
  test group.
- Set `retries` only on the CI profile. A retry that passes hides a race, on a developer machine
  and in CI. `flaky-result = "fail"` (nextest 0.9.131+) keeps the retry for diagnosis and still
  fails the run.

The `rust-test-tools` and `rust-tdd` skills own test design.

## Native artifacts and the FFI crate

`cargo rustc --crate-type` overrides the crate type for one invocation (stable since 1.64). The
manifest can then keep a plain Rust library, so `cargo build --workspace` does no linking work:

```bash
# Shared library (Android, JVM, desktop hosts)
cargo rustc --locked --profile <profile> --target <triple> \
    --crate-type cdylib -p <ffi-crate> --lib

# Static library (iOS)
cargo rustc --locked --profile <profile> --target aarch64-apple-ios \
    --crate-type staticlib -p <ffi-crate> --lib
```

- Find the produced file from the target directory's final-artifact path or from
  `--message-format=json`. Never read `deps/` or `build/` inside the target directory:
  `build.build-dir` (Cargo 1.91+) moves them, and their layout is not stable.
- Put `[profile.*]` sections only in the root manifest. Cargo ignores a member's profiles and
  warns `profiles for the non root package will be ignored`.
- `panic = "abort"` turns every `catch_unwind` guard into dead code. Keep `panic = "unwind"` on
  a profile that builds an FFI artifact when an entry point must return an error to the host
  instead of aborting the process. Since Rust 1.81, a panic that reaches an `extern "C"` or
  `extern "system"` boundary aborts. The guard pattern lives in the `rust-panic-safety` skill.
- Keep one FFI crate, a thin translation layer over the pure-logic crates. The `rust-jni`,
  `uniffi-boundary`, `uniffi-packaging-versioning`, and `ffi-error-progress-cancel` skills own the
  entry points and the boundary design.

Read [references/native-artifacts.md](references/native-artifacts.md) when you choose a profile
or the manifest crate type for a shared or static library, write FFI exports or a UniFFI crate,
or map a Cargo output name to the name a platform loader expects.

## Cross targets and host build systems

Read [references/cross-compilation.md](references/cross-compilation.md) when you set up
`.cargo/config.toml` for a non-host target, give cargo a linker or `CC_<triple>`/`AR_<triple>`
for a target, run tests for a target (a runner or a compile-only `--no-run` lane), or drive cargo
from Gradle, Xcode, or CMake.

## Rust edition

Edition 2024 is stable since Rust 1.85.0 and is the latest edition as of Rust 1.98.1. Keep the
steady state at one edition in `[workspace.package]`, inherited with `edition.workspace = true`.
Treat an edition bump as a workspace-wide contract change, in a dedicated change with formatting,
clippy, and test evidence.

- `cargo fix --edition` cannot fix code behind inactive features or `cfg` expressions. Run it
  with `--all-features` when the features are additive, or once per supported feature set
  otherwise, and once per shipping `--target`.
- Two edition-2024 changes alter drop order with a clean build and green tests: `if let`
  scrutinee temporaries drop before the `else` block, and tail-expression temporaries drop before
  the block's locals. Run `-W rust_2024_compatibility` before `cargo fix --edition`, because the
  lints go quiet after the bump.
- Do not set `style_edition` or `edition` in `rustfmt.toml` to the new edition until every crate
  has migrated. A direct `rustfmt` call from an editor or a pre-commit hook reads both keys, and
  then formats and parses the unmigrated crates as the new edition.

Read [references/edition-migration.md](references/edition-migration.md) when you migrate a
crate, before the first command. It has the command sequence, the migration order, and the edition-2024 breaking changes.

## Workspace commands cheat sheet

```bash
cargo build --locked --workspace --exclude <crate>
cargo bench --locked -p <bench-crate>             # Host only
cargo tree --locked --duplicates                  # Find duplicate versions
cargo tree --locked -i <dep>                      # Who depends on <dep>?
cargo tree --locked -e features -i <dep>          # Which features are active, and who enables them
cargo metadata --locked --no-deps --format-version 1   # Member list, JSON
cargo deny --config deny.toml --locked check      # cargo-deny >= 0.20; all four checks
cargo audit                                       # RustSec advisories only
```

The `rust-security` skill owns the `deny.toml` policy and advisory triage.

## Related skills

Each applies when it is installed.

- `rust-lints` - lint levels, clippy configuration, and the lint gate
- `rust-security` - cargo-deny policy, cargo-audit, advisory triage, new-crate review
- `rust-crate-architecture` - crate boundaries and dependency direction
- `rust-test-tools`, `rust-tdd` - test design, loom, golden files
- `rust-performance` - profiling and build-time tuning
- `rust-android-build`, `rust-ios-build` - platform packaging
- `rust-jni`, `uniffi-boundary`, `uniffi-packaging-versioning`, `ffi-error-progress-cancel` -
  the FFI boundary itself
- `rust-panic-safety` - panic guards at the boundary
