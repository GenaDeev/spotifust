# Native Artifacts and the FFI Crate

Read this file when you choose a Cargo profile or the manifest crate type for a shared or static
library, set up an FFI or UniFFI crate, or map a Cargo output name to the name a platform loader
expects. The `rust-android-build` and `rust-ios-build` skills own platform packaging, when they are
installed.

Contents:

- Profiles for native artifacts
- Crate type in the manifest
- FFI crate rules
- Native artifact mapping

## Profiles for native artifacts

Two valid strategies exist. Choose one and write it down.

- **Stock profiles.** `dev` for local debug variants, `release` for shipping variants. Profile
  behaviour stays identical to host builds.
- **Custom inherited profiles.** Use them when the shipped library needs different codegen from
  the host build, for example a size-optimized mobile artifact:

```toml
# Workspace root Cargo.toml
[profile.mobile-release]
inherits = "release"
opt-level = "z"        # Size; measure against "s" and 3
panic = "unwind"       # Keep when an entry point must return an error on panic
debug = "line-tables-only"
strip = "none"         # Packaging strips the shipped copy

[profile.mobile-dev]
inherits = "dev"
opt-level = 1
panic = "unwind"
```

- For an Android or iOS artifact, tune the platform ship profile instead: `android-jni` in the
  `rust-android-build` skill, `ios-release` in the `rust-ios-build` skill, when they are
  installed. Do not add `mobile-release` next to it.
- Measure `opt-level = "z"` against `"s"` and `3` on the real artifact before you ship it. Both
  `"s"` and `"z"` disable loop vectorization, so a compute-bound path can prefer `3`. The
  `rust-performance` skill has the measurement workflow, when it is installed.
- Select the profile from a host build property. Give local development a separate default, so a
  debug loop does not pay for a release build.

## Crate type in the manifest

Two valid manifest forms exist. Choose one and write it down.

Option A: a plain Rust library. Request `cdylib` or `staticlib` per invocation with
`cargo rustc --crate-type`, as SKILL.md shows.

```toml
# crates/<ffi-crate>/Cargo.toml
[lib]
crate-type = ["lib"]
```

Option B: always produce the shared library. Build scripts are simpler, but every workspace
build links the artifact.

```toml
# crates/<ffi-crate>/Cargo.toml
[lib]
crate-type = ["cdylib", "lib"]
```

Keep `lib` in the list under option B. Without it, no other workspace crate can use the FFI
crate, and its doc-tests cannot compile.

## FFI crate rules

- Prefer exactly one FFI crate. Add a second only when the platform loads the libraries
  independently, for example one `.so` per background service. Every extra boundary duplicates
  the error mapping, the panic guard, and the lifetime rules.
- Keep business logic out of the FFI crate. It is a thin translation layer over the pure-logic
  crates.
- A raw JNI or C crate exports `extern "system" fn` or `extern "C" fn` entry points with
  `#[unsafe(no_mangle)]` (required on edition 2024, accepted on every edition since Rust 1.82).
  Do not put `pub` on an export that takes a raw pointer. The export does not need `pub`, and a
  `pub` safe entry point that dereferences a raw pointer argument fails the deny-by-default
  `clippy::not_unsafe_ptr_arg_deref`. On jni 0.22, do not type `Java_*` symbol names by hand:
  use `#[jni_mangle]` or `native_method!` with `RegisterNatives`. The `rust-jni` skill owns the
  entry-point design.
- Keep the unsafe-documentation lints at the workspace level, and satisfy them: write the
  `# Safety` section or the `// SAFETY:` comment. Do not relax them for a binding crate. The
  `rust-lints` skill has the binding-layer lint rules, when it is installed.
- A UniFFI crate uses the proc-macro path: `#[uniffi::export]`, `#[derive(uniffi::Record)]` and
  similar, and `uniffi::setup_scaffolding!()`. UniFFI generates the entry points, so write no
  `#[no_mangle]` functions. Gate the bindgen CLI behind a feature, so a default workspace build
  never compiles it:

```toml
[features]
cli = ["uniffi/cli"]

[[bin]]
name = "uniffi-bindgen"
required-features = ["cli"]
```

The `uniffi-packaging-versioning` skill owns binding generation and version pinning. The
`uniffi-boundary` and `ffi-error-progress-cancel` skills own the boundary design.

## Native artifact mapping

Cargo derives the library file name from the package name with hyphens replaced by underscores.
The platform may need a different name. Map the two explicitly and keep the table next to the
build logic.

| Cargo package | Cargo output | Platform artifact |
|---------------|--------------|-------------------|
| `my-ffi` | `libmy_ffi.so` / `libmy_ffi.a` | `libmy_ffi.so` / `libmy_ffi.a` |
| `my-engine` | `libmy_engine.so` | `libmyengine.so` (renamed) |

The loader name must match: `System.loadLibrary("my_ffi")` loads `libmy_ffi.so`. UniFFI-generated
Kotlin derives this name from the crate, so a renamed artifact breaks the generated bindings.
Setting `[lib] name` in the manifest changes the Cargo output name at the source, which beats a
copy-and-rename step. It also changes the crate name that Rust dependents import.

A privileged helper executable is not a `System.loadLibrary` target. Build it with a separate
task and package it as an asset, not into `jniLibs`.
