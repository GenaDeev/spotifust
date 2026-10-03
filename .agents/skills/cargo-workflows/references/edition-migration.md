# Edition Migration

Read this file when you move one or more workspace crates to a new edition. The workflow applies
to any edition bump. The breaking-change list is for edition 2024, the latest edition as of Rust
1.98.1.

Contents:

- Per-crate workflow
- Migration order
- Edition 2024 changes that bite
- Formatting and the rustfmt edition

## Per-crate workflow

Do the migration once, workspace-wide, in a dedicated change. Run every command from the
workspace root and select the crate with `-p`.

```bash
# 1. Report the silent behaviour changes FIRST. These lints go quiet once the
#    crate is on edition 2024, because the behaviour has already changed.
cargo clippy --locked -p <crate> --all-targets --all-features -- -W rust_2024_compatibility

# 2. Rewrite the code. Start from a clean working tree.
cargo fix --edition --locked -p <crate> --all-features

# 3. Repeat for each shipping target that has cfg-gated code. The first run
#    left uncommitted edits, so later runs need --allow-dirty.
cargo fix --edition --locked -p <crate> --all-features --allow-dirty --target <triple>

# 4. Read the diff. cargo fix edits .rs files in place.
git diff

# 5. Set `edition = "2024"` in this crate's Cargo.toml (see the note below).

# 6. Verify.
cargo fmt -p <crate>
cargo clippy --locked -p <crate> --all-targets -- -D warnings
cargo nextest run --locked -p <crate>
cargo test --locked -p <crate> --doc
```

`cargo fix` cannot update code for inactive features or inactive `cfg` expressions. Replace
`--all-features` with one run per supported feature set when the features are not additive. Run
step 1 with `--target <triple>`, and step 3, once for each shipping target triple, for example the
Android and iOS triples of an FFI crate. A target run needs the target installed with `rustup target add`, and a C toolchain
for that target when a build script compiles C.

During the migration, replace `edition.workspace = true` in the crate with an explicit
`edition = "2024"`. When the last crate is done, set the edition in `[workspace.package]` and
restore `edition.workspace = true` in every crate. Crates on different editions interoperate, so
a staged migration builds at every step.

## Migration order

Work from the leaves inward. The FFI crate goes last, because it depends on everything and the
stricter `extern` rules hit it hardest.

1. Host-only or pure-logic crates, for example the CLI crate and the error crate.
2. Core logic crates under `#![forbid(unsafe_code)]`.
3. Mid-layer crates.
4. Backend and pipeline crates.
5. The FFI crate.

## Edition 2024 changes that bite

- **Stricter `unsafe` in `extern` blocks.** Write an `extern` block as `unsafe extern "C" { ... }`
  or `unsafe extern "system" { ... }`. Every item inside it is unsafe to call by default. Mark an
  item `safe fn ...` only when the callee has no safety contract. Review the FFI crate and every
  crate with platform C bindings.
- **Unsafe attributes.** Write `#[unsafe(no_mangle)]`, `#[unsafe(export_name = "...")]`, and
  `#[unsafe(link_section = "...")]`. Every raw FFI export changes. `cargo fix --edition` rewrites
  them.
- **`std::env::set_var` and `std::env::remove_var` are unsafe.** `cargo fix --edition` wraps each
  call in `unsafe { }` and adds a comment (measured on Rust 1.98.1):
  `// FIXME: Audit that the environment access only happens in single-threaded code.`
  A call that runs while another thread reads the environment is undefined behaviour, and the
  test harness runs tests on several threads. Resolve every FIXME: move the call to
  single-threaded startup, or replace it, for example with `Command::env` for a child process.
  Then write a `// SAFETY:` comment on the block that stays.
- **`unsafe_op_in_unsafe_fn` warns.** An unsafe operation inside an `unsafe fn` needs its own
  `unsafe { }` block. `cargo fix --edition` wraps the whole body in one block and writes no
  `// SAFETY:` comment. Narrow the block and add the comment before the lint gate runs:
  `clippy::undocumented_unsafe_blocks` rejects the bare block when the workspace enables it.
- **`static mut` references stop the build.** The `static_mut_refs` lint is deny-by-default on
  edition 2024. `&mut COUNTER` fails with `error: creating a mutable reference to mutable static`.
  `println!("{}", COUNTER)` fails with `error: creating a shared reference to mutable static`,
  because the format machinery takes a reference. Only a direct read or write of the value inside
  `unsafe` still compiles. Do not use `#[allow(static_mut_refs)]` as the migration answer.
  `&raw mut COUNTER` and `&raw const COUNTER` create no reference, but a raw pointer keeps every
  data race that the `static mut` had. `cargo fix --edition` prints the `&raw mut` suggestion but
  does not apply it. The `memory-model` skill selects the real replacement.
- **`gen` is a reserved keyword.** `cargo fix --edition` rewrites an identifier `gen` to `r#gen`
  (measured on Rust 1.98.1). Rename it before the migration if you do not want the raw identifier.
- **`impl Trait` in return position captures all in-scope lifetimes.** A function that must
  capture fewer needs `use<'a, T>`. Iterator adapters are the usual site. `cargo fix --edition`
  normally adds the bound.
- **`if let` and `while let` chains stabilize (Rust 1.88).** Do not collapse nested patterns in
  the migration commit. Keep the migration diff surgical.
- **Tail-expression temporaries drop earlier.** A temporary in the tail expression of a block now
  drops before the block's local variables. Edition 2021 dropped it after them. A body of
  `let _local = Noisy("local"); temp().0.len() > 0` prints `drop local / drop temporary` on 2021 and
  `drop temporary / drop local` on 2024. In this example nothing fails to compile, so
  `cargo fix --edition` cannot repair it. The `tail_expr_drop_order` lint finds the sites. The
  change can also add a new E0716: `let x = { &String::from("1234") }.len();` compiles on 2021
  and fails on 2024 (measured on Rust 1.98.1). Move the block value into a named `let`:
  `let s = { &String::from("1234") }; let x = s.len();`.
- **`if let` releases its scrutinee before the `else` block.** A temporary in the `if let`
  scrutinee now drops before the `else` block runs. Edition 2021 held it to the end of the whole
  `if let`. A `MutexGuard` left as a temporary stops guarding the `else` branch: with a static
  `M: Mutex<Option<u32>>`, `if let Some(v) = *M.lock().unwrap() { .. } else { M.try_lock().is_ok() }`
  yields `false` on 2021 and `true` on 2024. The `if_let_rescope` lint finds the sites. Bind the
  guard to a named local, so the drop point is explicit and the same on both editions.

The last two entries can change run-time behaviour with a clean build and green tests. That is why
step 1 runs `-W rust_2024_compatibility` before `cargo fix --edition`, not after. Both lints are
allow-by-default members of the `rust-2024-compatibility` group.

## Formatting and the rustfmt edition

`cargo fmt` formats each crate with the style edition that the crate's own `Cargo.toml` edition
implies. A crate's edition bump therefore reformats that crate. Run `cargo fmt -p <crate>` in the
migration commit, so the next change does not carry an unrelated format diff.

Two `rustfmt.toml` keys exist, and they do different things:

| Key | Effect under `cargo fmt` | Rule |
|-----|--------------------------|------|
| `style_edition` | Forces one formatting style on every crate. | Do not set it to the new edition until every crate has migrated. An early bump reformats the crates that have not migrated and buries the real diff. |
| `edition` | No effect, because `cargo fmt` passes each crate's own edition. | A direct `rustfmt` call reads it as the parse edition, and also as the style edition when `style_edition` is absent. Keep it equal to the workspace steady-state edition. Bump it together with `style_edition` after the last crate migrates. |

Measured on Rust 1.98.1: a 2021 crate with `edition = "2024"` in `rustfmt.toml` kept
`use m::{Bar10, Bar9};` under `cargo fmt`. A direct `rustfmt src/lib.rs` call in the same crate
changed it to `use m::{Bar9, Bar10};` and rejected an identifier named `gen`. The same crate with
`style_edition = "2024"`, or with crate edition 2024, got `use m::{Bar9, Bar10};` under
`cargo fmt`.
