# Workspace Patterns

Read this file when you lay out a workspace, add or change a dependency, add a workspace member,
review a `Cargo.lock` diff, narrow a build to a subset of the workspace, or set up the
latest-dependencies or MSRV job.

Contents:

- Workspace layout
- Workspace dependency management
- When to pin exactly
- Adding a dependency
- Adding a member crate
- Selective build commands
- Reviewing a `Cargo.lock` diff
- Scheduled latest-dependencies job
- MSRV job without a download by an old Cargo

## Workspace layout

```text
<workspace-root>/
  Cargo.toml              # Virtual manifest: members, resolver, deps, lints, profiles
  Cargo.lock              # Committed, libraries included
  rust-toolchain.toml     # Pinned toolchain + components (rustfmt, clippy)
  rustfmt.toml            # Formatter config
  clippy.toml             # Clippy thresholds; msrv only to override rust-version
  deny.toml               # cargo-deny policy
  .cargo/config.toml      # Per-target rustflags and runners
  .config/nextest.toml    # nextest profiles
  crates/
    <leaf-crates>/        # Pure logic, no internal dependents
    <mid-layer-crates>/
    <ffi-crate>/          # cdylib / staticlib boundary, depends on everything
    <cli-crate>/          # Host-only binary
    <bench-crate>/        # Benchmarks
```

- `members = ["crates/*"]` and an explicit list both work. For an explicit list, the default here
  is leaf crates first and the FFI crate last, so the file records the dependency direction.
- Keep helper scripts and fixture generators outside `crates/`. Under a glob, a directory without
  a `Cargo.toml` stops every command with `failed to load manifest for workspace member`.

## Workspace dependency management

Centralize every shared dependency version in the root `Cargo.toml`. A member crate does not carry
its own version number for a shared dependency.

```toml
[workspace.dependencies]
# Internal crates: path only. cargo-deny accepts this in a `publish = false`
# crate with `allow-wildcard-paths = true` (the rust-security skill owns
# deny.toml). A published crate also needs `version`, equal to the member's own.
my-domain   = { path = "crates/my-domain" }
my-error    = { path = "crates/my-error" }

# External crates: compatible ranges. Turn default features off here, not in
# the member, when any member must build without them.
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
tokio      = { version = "1", default-features = false }
rusqlite   = { version = "0.40", default-features = false, features = ["bundled"] }
flate2     = { version = "1", default-features = false, features = ["rust_backend"] }

# Determinism-critical: Cargo.lock holds the exact version. Bump it alone.
libm       = "0.2.16"     # Bit-identical transcendentals
```

Members inherit the entry and can add `features` and `optional`. Nothing else:

```toml
[dependencies]
serde.workspace = true
my-domain.workspace = true
tokio = { workspace = true, features = ["rt", "net"] }
```

A member cannot turn off a default feature that the workspace entry keeps on. On Cargo 1.98 an
edition-2024 member gets a hard error, and an older-edition member gets a warning and keeps the
default features. The mechanism and the fix are in
[feature-resolution.md](feature-resolution.md).

## When to pin exactly

Use an `=` requirement only for a tightly coupled pair, such as a crate and its companion
proc-macro crate. An `=` pin stops `cargo update -p` from taking a security fix, and in a
published library it makes downstream resolution failures more likely. The `rust-security` skill
owns this rule.

Some crates can change program output in a patch release:

- Math and float formatting crates, when the output must be bit-identical across platforms.
- Text shaping and font parsing crates, when a rendered or measured result is compared against a
  stored snapshot.
- Any crate whose output feeds a golden test.

Keep a compatible range for them too. The committed `Cargo.lock` holds the exact version. Mark
each one in `[workspace.dependencies]`, bump it deliberately in its own change, and re-bless the
affected snapshots in the same commit.

## Adding a dependency

1. Confirm the exact crate name and owner on crates.io before you run `cargo add`. A plausible
   name can belong to an unrelated or malicious crate. The `rust-security` skill has the full
   new-crate review, when it is installed.
2. Do not add a crate to fix a compile error. For E0432 or E0433, first look for a missing
   `use std::...` path or a missing feature on a crate you already have.
3. Add the entry to `[workspace.dependencies]`, then inherit it in the member:

```bash
# After the [workspace.dependencies] edit:
cargo add -p <member> <crate> --dry-run   # Prints "Adding <crate> (workspace)"
cargo add -p <member> <crate>             # Writes <crate>.workspace = true
cargo tree --locked -i <crate>            # Confirms who pulls it in
```

`cargo add` inherits the workspace entry when one exists (measured on Rust 1.98.1). It updates
`Cargo.lock`, so commit the lock change with the manifest change.

## Adding a member crate

- Add `[lints] workspace = true` to the new member. Without it, the member silently builds with
  default lint levels. The lint table itself lives in the `rust-lints` skill.
- Set `edition.workspace = true` and `rust-version.workspace = true`, so the member follows the
  workspace MSRV and edition.
- Put `#![forbid(unsafe_code)]` in the crate root of every crate with no hand-written `unsafe`.
  A crate that calls FFI only through a safe wrapper crate qualifies. Do not set it in the
  workspace lint table: every member inherits that table, and the FFI crate needs `unsafe`. The
  `rust-unsafe` skill owns this rule.

## Selective build commands

```bash
# Build or check one member
cargo build --locked -p <crate>
cargo check --locked -p <crate>

# Test one member, or one integration test inside it
cargo nextest run --locked -p <crate>
cargo nextest run --locked -p <crate> --test <integration-test>

# Exclude an expensive member from a workspace build
cargo build --locked --workspace --exclude <bench-crate>

# Cross-compile type-check. This needs no target linker. `cargo check` still
# builds and links build scripts and proc macros, but only for the host.
cargo check --locked --target aarch64-linux-android -p <crate>
cargo check --locked --target aarch64-apple-ios     -p <crate>
```

`cargo check --target <triple>` is the cheapest guard against a cross-compilation break. Run it
in CI for every shipping target, even when the full native build runs only on the release lane.
Build scripts still run under `cargo check`. A dependency whose build script compiles C (`cc`, a
`bundled` feature such as the `rusqlite` entry above) still needs the target C compiler: set
`CC_<triple>` and `AR_<triple>`, or limit the check to pure-Rust crates.

`cargo check` does not prove that the code links or that monomorphization succeeds. Run
`cargo build` for that.

## Reviewing a `Cargo.lock` diff

- Check every version bump on a security-sensitive crate: TLS, HTTP clients, compression, image
  and font parsers, and anything that parses untrusted input.
- Check that a determinism-critical crate did not move. If it did, the change must also carry
  re-blessed snapshots.
- Check for a new duplicate version of a crate already in the graph. Run
  `cargo tree --locked --duplicates` to confirm. Record an unavoidable split in the cargo-deny
  `skip` list with its cause (policy in the `rust-security` skill).
- A large unexplained lock diff usually means somebody ran `cargo update` or
  `cargo generate-lockfile` instead of `cargo update -p <dep>`. Ask for the reason.
- A new package in the diff that no manifest change explains is a red flag. Run
  `cargo tree --locked -i <package>` to find who pulls it in.

## Scheduled latest-dependencies job

SKILL.md has the runner, secret, and cache rules for this job. The job body:

```bash
export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=allow   # Ignore rust-version when choosing
rustup toolchain install stable
cargo +stable update --verbose
cargo +stable test --workspace
```

Run this job on current stable, not on the pinned toolchain. With `allow`, the update can select a
dependency whose `rust-version` is newer than the pin, and the job then fails on the compiler
version instead of on the breakage it exists to find. A `+stable` override beats
`rust-toolchain.toml`; `rustup default stable` does not.

## MSRV job without a download by an old Cargo

Use this procedure when SKILL.md says that the MSRV Cargo must not download:

1. Run `cargo vendor --locked vendor` with the pinned toolchain, which must be 1.96.1 or later.
2. Put the `[source]` tables that it prints in `.cargo/config.toml` for the MSRV job.
3. Add `--offline` to the MSRV command.

Do not use `cargo fetch` for this: Cargo 1.85 changed the hash in the registry cache path, so an
MSRV below 1.85 does not find the cache that a newer Cargo fills.
