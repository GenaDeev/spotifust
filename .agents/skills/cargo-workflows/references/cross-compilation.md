# Cross-Compilation and Host Build Integration

Read this file when you set up `.cargo/config.toml` for a non-host target, give cargo a linker or
C toolchain for a target, run tests for a target, or drive cargo from Gradle, Xcode, CMake, or
another host build system. Platform packaging lives elsewhere: the `rust-android-build` skill owns
the NDK, per-ABI rustflags, and Gradle `jniLibs` work, and the `rust-ios-build` skill owns
XCFramework and SwiftPM work, when they are installed.

Contents:

- Put shared config at the invocation root
- Target rustflags
- Linker and C toolchain per target
- Separate cross-compilation from test execution
- Apple targets
- Wiring a host build system to cargo

## Put shared config at the invocation root

Cargo searches `.cargo/config.toml` from the current directory through its ancestors. It does not
start again from each workspace member. A config under `crates/member/.cargo/` works when Cargo
runs from that member and disappears when CI runs from the workspace root.

Put shared target configuration at `<workspace>/.cargo/config.toml`. Run the same command from the
same directory in local development, CI, Gradle, and Xcode tasks. To share a checked-in fragment,
use the `include` key (Cargo 1.94+; paths are relative to the including file). For a one-off,
pass `--config` explicitly and record it as a task input.

```toml
# <workspace>/.cargo/config.toml
include = [
  { path = "ci.toml", optional = true },
]
```

`include` needs Cargo 1.94 for every toolchain that reads this config, the MSRV lane included.
Measured on Cargo 1.88.0: the table form above stops every command with
`` failed to parse key `include` ``, and the string form `include = ["ci.toml"]` is ignored
without a warning.

## Target rustflags

Put target-wide codegen flags in `[target.<triple>]` in `.cargo/config.toml`. Do not put linker
paths there: the host build system knows where the NDK or SDK lives, and a checked-in absolute
path breaks on every other machine. The Android block (16 KiB page alignment, build ID) lives in
the `rust-android-build` skill; profiling additions such as frame pointers live in the
`rust-performance` skill.

Cargo takes extra rustc flags from exactly one source, in this order: `CARGO_ENCODED_RUSTFLAGS`,
then `RUSTFLAGS`, then all matching `target.<triple>.rustflags` and `target.<cfg>.rustflags`
entries, then `build.rustflags`. An environment variable replaces the config entries; it does not
add to them. A CI job that sets `RUSTFLAGS=-Dwarnings` therefore drops the per-target link flags
without a message.

## Linker and C toolchain per target

Call cargo directly and set the linker through the environment at build time.

```bash
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
export CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
cargo build --locked --target aarch64-unknown-linux-gnu --profile <profile>
```

`CARGO_TARGET_<TRIPLE>_LINKER` uses the triple upper-cased with underscores; cargo reads it. The
`CC_<triple>`, `CXX_<triple>`, and `AR_<triple>` variables use the triple with underscores (the
`cc` crate also accepts hyphens); `cc`-based build scripts read them. Set both kinds, or the Rust
code links with one compiler and the bundled C code builds with another. For Android NDK drivers,
the `rust-android-build` skill has the table.

## Separate cross-compilation from test execution

`cargo test --target <triple>` builds a target test binary, then tries to run it. Since Rust 1.89
it also builds and runs doctests for that target. Configure `target.<triple>.runner` when an
emulator, device bridge, or remote executor can run the binaries:

```toml
[target.aarch64-unknown-linux-gnu]
runner = ["qemu-aarch64", "-L", "/usr/aarch64-linux-gnu"]
```

If no runner exists, build the tests without running them and report the lane as compile-only.
Do not call it a test pass.

```bash
cargo test --locked --no-run --target <triple>
```

Mark a doctest that cannot run on a target with an `ignore-<target-substring>` fence attribute,
for example `ignore-android`. Rust 1.88 and later honor it. Keep target execution in a separate
device or emulator lane and record the digest of the exact artifact that it runs.

## Apple targets

Install `aarch64-apple-ios` for devices and `aarch64-apple-ios-sim` for the simulator on Apple
silicon. Add `x86_64-apple-ios` only when the support matrix includes Intel simulators: Xcode 27
runs only on Apple silicon. Give pull-request CI a reduced mode that builds only
`aarch64-apple-ios-sim`, and build the full slice set on the release lane. The `rust-ios-build`
skill owns `IPHONEOS_DEPLOYMENT_TARGET`, `lipo`, and XCFramework assembly.

## Wiring a host build system to cargo

These rules apply to Gradle, Xcode, CMake, and similar drivers. The Gradle task layout lives in
the `rust-android-build` skill.

1. Register one task per variant and target. Independent tasks let the build system schedule the
   targets in parallel.
2. Give every target its own `CARGO_TARGET_DIR`. A shared target directory serializes parallel
   builds on the cargo lock.
3. Always pass `--locked`, and select the Cargo profile from a build property.
4. Declare task inputs as the recursive production-dependency closure of the FFI crate, not the
   whole `crates/` directory. Otherwise an unrelated crate invalidates the native task on every
   commit. Add a test that compares the declared inputs with the closure that
   `cargo metadata --locked --format-version 1` reports.
5. Keep the native task off the plain compile path of the host language. Wire it into the
   packaging tasks that consume native libraries.
6. Find the produced artifact as [SKILL.md](../SKILL.md) describes: never read `deps/` or `build/` inside the
   target directory. Map its name to the platform name as
   [native-artifacts.md](native-artifacts.md) describes.
7. Gate the release path: the native library exists, has the right format for its target, and
   stays under a size budget.

Expose few properties and fix the rest in the build logic, so a build cannot be misconfigured
silently:

| Property or setting | Rule |
|---------------------|------|
| Enable native build | A boolean that disables the native task entirely, default on. |
| Target override | Allowed for a debug build, default to the host or emulator target. Impossible on the release path: a release that ships a subset of targets is a shipping incident. |
| Cargo profile | One property for CI and release, one for local development (a dev-inherited profile). |
| Toolchain versions | Pinned in the version catalog or project config, not in a property. |
| Output path | A generated directory under the host build directory, wired into the variant. |
