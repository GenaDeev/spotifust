---
name: rust-performance
description: Use when a Rust workload is slow, a benchmark regressed, a binary or app bundle grew, or a build is slow, and the next step is to measure it with a flamegraph, samply, perf, simpleperf, Perfetto, Instruments, DHAT, Criterion, Gungraun, cargo-bloat, cargo-llvm-lines, or cargo --timings, or to tune LTO, codegen-units, opt-level, PGO, or a rayon thread pool. Not for choosing the code change after a profile names the hotspot; use `rust-hot-path`.
license: BSD-3-Clause
---

# Rust Performance

Measure Rust speed, heap use, binary size, and build time on the host, on Android, and on iOS. The `rust-hot-path` skill, when it is installed, turns a named hotspot into a code change.

## Rules of engagement

1. Name one metric and record a baseline before you change code: items/s, MB/s, ms per pass, peak MB, or KB added to the app bundle. A change without a before and an after number is a guess.
2. Measure the build that ships. Numbers from `dev` do not transfer to `release`. On the host, run profilers on the `profiling` profile below, which is the release build plus symbols. On Android and iOS, profile the platform ship profile (sections 2 and 3), which keeps line tables and does not strip.
3. Change one thing per measurement. Keep the machine, the power state, and the device the same for the A and B runs.
4. Fix the algorithm or the data structure before you tune build flags.

## Build a profile that the profiler can read

A profile without symbols shows hex addresses and truncated stacks. A release build has no debug info: Cargo passes `-C strip=debuginfo` whenever `debug` is off (checked on 1.98.1), and many ship profiles also set `strip = "symbols"`. Define this profile once in the workspace-root `Cargo.toml`:

```toml
[profile.profiling]
inherits = "release"
debug = true      # "line-tables-only" is enough when file:line is all you need
strip = "none"    # required: `strip` is inherited, so a stripping release strips this too
```

Build it with `--locked`, then point every profiler at `target/profiling/<name>`:

```bash
cargo build --locked --profile profiling --bin myapp
```

Check the build before you read a profile. On Linux, `readelf -S target/profiling/myapp | grep -c debug_info` must print a non-zero count. On every host, the flamegraph must show function names, not hex addresses.

## Tool selection

| Target | CPU profile | Heap profile | Notes |
|--------|-------------|--------------|-------|
| Host (Linux) | `samply`, `cargo flamegraph`, `perf record` | `heaptrack`, DHAT | Set `kernel.perf_event_paranoid` to 1 or lower once, as root |
| Host (macOS) | `samply`, `cargo flamegraph` (`xctrace`), Instruments | DHAT through the `dhat` crate, Instruments Allocations | Grant the profiling permission when prompted; do not weaken SIP |
| Android | `simpleperf`, Perfetto | Android Studio native allocations | `perf`, `heaptrack` and DHAT do not run there |
| iOS | Instruments Time Profiler, `os_signpost` | Instruments Allocations and Leaks | MetricKit for data from shipped devices |

## Verification

Before you claim a performance change, have evidence for each line that applies:

- The hotspot: a profile of the `profiling` profile (host) or of the ship profile (device), or a benchmark, named it before the change.
- The win: `cargo bench --locked -p <crate> --bench <b> -- --save-baseline before` before the change and `-- --baseline before` after, with a change interval that does not cross zero; or a Gungraun instruction-count delta. A green `cargo bench --no-run` proves only that the benchmarks compile.
- The benchmark measured work: the two-size scaling check in section 6 moved the time.
- The size: `cargo bloat --locked --profile <ship-profile> --target <triple> --crates` captured before and after, when the change touches generics, dependencies, or profile settings.
- The profiles: `panic` and `strip` did not change silently in a profile that a symbolication or FFI-catch path depends on.
- New `unsafe` or new parallelism: checked with the sanitizers in the `rust-sanitizers-miri` skill.

## Failure triage

| Symptom | Cause | Fix |
|---------|-------|-----|
| Flamegraph shows hex addresses | The profiled build has no symbols | Build and profile with `--profile profiling`; check it with `readelf -S` |
| Linux stacks wrong, or no function names, under `cargo flamegraph` or `perf` | `rust-lld` without `--no-rosegment`, or a build without debug info | Profile the `profiling` profile; pass `-Wl,--no-rosegment` through `CARGO_TARGET_<TRIPLE>_RUSTFLAGS` |
| Stacks stop at the same depth in deep call chains | DWARF mode copies only 64000 bytes of stack per sample | Add `-Cforce-frame-pointers=yes` to `CARGO_TARGET_<TRIPLE>_RUSTFLAGS`, rebuild, and run `cargo flamegraph --profile profiling --cmd "record -F 997 --call-graph fp" --bin myapp` |
| Stacks cut off only in `--call-graph fp` mode | No frame pointers in your code | Add `-C force-frame-pointers=yes` through `CARGO_TARGET_<TRIPLE>_RUSTFLAGS` or the config file, not `RUSTFLAGS` |
| A flag from `.cargo/config.toml` has no effect | `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` is set and replaces it | Unset it, or move every flag to one config level |
| Frames or reports show raw `_R...` names | The tool predates v0 mangling, the default since Rust 1.97 | Upgrade the tool, or pipe text output through `rustfilt`; see `rust-debugging` |
| Symbolication shows `<unknown>` | Stripped library | Use the unstripped `.so` from `target/<triple>/<profile>/`, not the packaged copy |
| `cargo flamegraph --locked` fails with `unexpected argument` | `cargo flamegraph` has no `--locked` | Run `cargo build --locked ...` first, then `cargo flamegraph` without it |
| `cargo flamegraph` fails on Linux | `perf_event_paranoid` too high | Set it to 1 or lower |
| `cargo flamegraph` fails on macOS | `xctrace` permission, or a `--freq` other than 997 | Grant the permission and drop `--freq`, or use `samply`; do not weaken SIP |
| Benchmark results swing by more than 10% between runs | Thermal or scheduler noise | Fix the power state, close background load, raise `sample_size` |
| Binary grew after a dependency bump | New monomorphizations or new codegen | `cargo bloat --crates` then `cargo llvm-lines` on the top crate |
| An Android profiling step fails | Device, manifest, or packaging setup | The common-mistakes table in [references/android-profiling.md](references/android-profiling.md) |

## Silent failures

None of these gives an error:

- A CI gate that cannot fail. Criterion prints "Performance has regressed." and still exits 0. A Gungraun run without the main-branch baseline prints `N/A` for every comparison and passes.
- Unsound parallel code. Never use a JNI environment, or an FFI handle that is not thread-safe, inside a `rayon` closure: rayon runs the closure on other threads.
- A lost FFI error path. `panic = "abort"` makes every `catch_unwind` inert, so an FFI entry point aborts the process and returns no error.
- Crashes that nobody can symbolicate. A second build with other `debug` or `strip` settings has a different build ID, so it cannot symbolicate the shipped binary.
- A library that faults on the device. Never put `-C target-cpu=native` in a config that cross-compiles: the library then assumes host CPU features and faults with an illegal instruction.

## 1. Host profiling

`samply` 0.13.1 samples on macOS and Linux and opens the result in the Firefox Profiler. It needs no root on macOS.

```bash
cargo install --locked samply
samply record ./target/profiling/myapp --workers 4
```

`cargo flamegraph` 0.6.14 records with `perf` on Linux and with `xctrace` on macOS. It has no `--locked` option, and it runs its own `cargo build` without one. Build with `--locked` first, then profile the same profile:

```bash
# x86_64 Linux only: rust-lld, the default linker there since Rust 1.90, needs this for perf stacks
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-Clink-arg=-Wl,--no-rosegment"

cargo build --locked --profile profiling --bin myapp
cargo flamegraph --profile profiling --bin myapp -- --workers 4
```

Set `--no-rosegment` only for the profiling build and the `cargo flamegraph` run, as above. In the repository config it also reaches shipped builds and puts read-only data in an executable segment. `CARGO_TARGET_<TRIPLE>_RUSTFLAGS` joins the config-file `[target]` rustflags; a plain `RUSTFLAGS` replaces every config-file entry. Read [references/cargo-flamegraph-setup.md](references/cargo-flamegraph-setup.md) when a first `cargo flamegraph` run fails or shows broken stacks, when you profile a test, example, or benchmark, or when you interpret a flamegraph pattern. Width is the share of samples; the left-to-right order is alphabetical, not time.

Heap profiles:

- Linux: `heaptrack ./target/profiling/myapp`, or `valgrind --tool=dhat ./target/profiling/myapp`.
- Any host, on stable: the `dhat` crate 0.3.3. Put both the `#[global_allocator] static ALLOC: dhat::Alloc = dhat::Alloc;` item and the `dhat::Profiler::new_heap()` call behind a `dhat-heap` feature, so neither reaches the shipped binary. Run `cargo run --locked --profile profiling --features dhat-heap -- <args>`, then open `dhat-heap.json` in the [DHAT viewer](https://nnethercote.github.io/dh_view/dh_view.html).
- A `dhat::assert_eq!` allocation-count test pins a win. The `rust-hot-path` skill has the test and its one-test-per-file isolation rule.

## 2. Android on-device profiling

Use `simpleperf` for a CPU profile of one app. Use Perfetto when you need the native profile next to scheduler, binder and app frame data. The app must be profileable (`<profileable android:shell="true" />` for a release build on Android 10 or later) or debuggable, and symbolization needs the unstripped `.so` under `target/<triple>/<ship-profile>/`, not the packaged copy. Read [references/android-profiling.md](references/android-profiling.md) when you record on a device: it has the call-graph mode and frame-pointer rules per ABI, the `simpleperf` and Perfetto commands, the native memory profiler, and a common-mistakes table.

## 3. iOS on-device profiling

Use Instruments through Product -> Profile in Xcode; there is no `simpleperf` on iOS. Read [references/ios-profiling.md](references/ios-profiling.md) when you profile a Rust library in an iOS app: it has the symbol setup (a Rust profile with line tables and no strip, and a dSYM), `os_signpost` interval markers around calls into Rust, and MetricKit field data.

## 4. Binary size (cargo-bloat)

Pass the profile that ships: `android-jni` in the `rust-android-build` skill, or `ios-release` in the `rust-ios-build` skill. Numbers from `release` do not match a size-optimized mobile profile.

```bash
# Per-crate breakdown; this maps to app bundle growth. Use aarch64-apple-ios for the iOS device slice.
cargo bloat --locked --profile <ship-profile> --target aarch64-linux-android --crates > before.txt
# apply the change, then run it again and compare
cargo bloat --locked --profile <ship-profile> --target aarch64-linux-android --crates > after.txt
diff before.txt after.txt

# Top 20 functions by size
cargo bloat --locked --profile <ship-profile> --target aarch64-linux-android -n 20
```

Each type that crosses an FFI boundary generates scaffolding code. If the FFI crate dominates `--crates`, reduce the enums and records that cross; see `uniffi-boundary` and `rust-jni`. Section 8 has the `strip` rule.

## 5. Monomorphization bloat (cargo-llvm-lines)

`cargo llvm-lines` counts LLVM IR lines per function. High IR volume costs both compile time and binary size.

```bash
cargo install --locked cargo-llvm-lines
cargo llvm-lines --locked --release -p my-crate | head -30
```

A high `Copies` count means the generic was instantiated many times. Keep the generic surface and move the body into one concrete inner function; this cut LLVM IR 4.6x in a measured case. To cut the number of copies as well, change the signature shape. Read the generic-signature section of [references/build-time-optimization.md](references/build-time-optimization.md) when `cargo llvm-lines` names a generic function: it has the thin-wrapper example and the measured costs.

## 6. Benchmarks

Use Criterion 0.8 by default. Use Gungraun (Valgrind instruction counts, Linux only) for a number that does not move with machine noise, or for a CI gate. Do not use `#[bench]`: it is nightly-only and fails on stable with E0658. Read [references/benchmarking.md](references/benchmarking.md) when you pick another harness (Divan, Hyperfine, Gungraun) or write or review benchmark code: it has the harness table, the manifest, benchmark structure, statistics, async benchmarks, the scaling probe with measured numbers, and the CI gate commands.

Declare `harness = false` for every benchmark target in the crate manifest, otherwise the built-in test harness intercepts the arguments.

```bash
cargo bench --locked --workspace --no-run      # compile every benchmark, measure nothing
cargo test --locked --workspace --benches      # run each Criterion benchmark once (smoke test)

# Run one suite, or one benchmark function
cargo bench --locked -p my-crate --bench decode
cargo bench --locked -p my-crate --bench decode -- decode_large

# Save a baseline, change the code, then compare against it
cargo bench --locked -p my-crate --bench decode -- --save-baseline before
cargo bench --locked -p my-crate --bench decode -- --baseline before
```

Name the bench target with `--bench` when you pass Criterion flags. Without it, the lib target's libtest harness also receives the flags and stops with `error: Unrecognized option: 'save-baseline'`.

Read the `change` interval in Criterion's output, not the midpoint. If the interval crosses zero, or `p > 0.05`, there is no measured change. Increase `sample_size` or `measurement_time`; do not lower the significance level. A low p-value is not proof either: wall-clock variance from memory layout (symbol order, environment size, stack alignment) is systematic within one build, so it repeats across samples and passes the test. Confirm a small wall-clock win with Gungraun instruction counts on Linux before you keep the change.

Prove that the benchmark measured something. `black_box` on the input and on the output does not prove that the work ran: LLVM can rewrite a reduction such as `(0..n).sum()` into a closed form, so the routine becomes O(1). Run the same routine at two input sizes 10x apart. A time ratio near 1.0 means the benchmark measured nothing. Do not require a ratio near 10: cache effects make it superlinear, and fixed per-iteration overhead makes it sublinear.

Gate benchmarks in CI:

- Every change: the compile or the smoke command above.
- Do not gate on Criterion (see Silent failures). A shared CI runner is also too noisy for a wall-clock gate.
- For an automatic gate, compare Gungraun instruction counts against a baseline from the main branch with `--callgrind-limits`. A regression over the limit exits with code 3. Both runs must read the same `target/gungraun` directory (see Silent failures).

## 7. Data-parallel work with rayon

The global `rayon` pool starts one thread per logical CPU, or `RAYON_NUM_THREADS`. A library must not build the global pool: the host, or an earlier `par_iter` call, can create it first, and `build_global` then returns an error. Build a library-owned pool once at init with `rayon::ThreadPoolBuilder::new().num_threads(threads).build()`, and run the parallel work inside `pool.install(|| ...)`.

- On a phone, let the host app choose the thread count and pass it across the FFI boundary at init. Return the build error to the caller; do not panic in init.
- Resolve every JNI or FFI value you need into owned data before the parallel section (see Silent failures).

## 8. Profiles and LTO

Cargo reads `[profile.*]` only from the workspace-root manifest. A table in a member crate or in a dependency is discarded, so a library crate cannot ship optimization settings to its consumers. Profile names other than `dev`, `release`, `test` and `bench` are your own convention.

```toml
[profile.release]
lto = "thin"
codegen-units = 1
debug = "line-tables-only"
strip = "none"             # strip the shipped copy in packaging (llvm-strip, Gradle, Xcode); archive this one
panic = "abort"

[profile.mobile-release]   # this is what ships in the app bundle
inherits = "release"
opt-level = "z"            # size-optimized; measure against 3
lto = "fat"
panic = "unwind"           # required when the FFI boundary catches panics
strip = "none"             # keep symbols here; the packaging step strips its own copy
debug = "line-tables-only"
```

Add the `profiling` profile from the top of this skill next to these. When the workspace already has a platform ship profile (`android-jni` from `rust-android-build`, `ios-release` from `rust-ios-build`), tune that profile and do not add `mobile-release` next to it.

Four settings need a deliberate answer:

- `panic`. Use `"abort"` for the smallest binary and no unwinding overhead. Use `"unwind"` when an FFI entry point must return an error instead of aborting (see Silent failures). The `rust-panic-safety` skill owns the boundary policy and the panic-message privacy rule.
- `strip`. Use `"symbols"` only when the artifact never needs crash symbolication. Otherwise keep `"none"` with `debug = "line-tables-only"`, strip the shipped copy in the packaging step, and archive the unstripped Cargo output it came from. The `rust-debugging` skill has the archive check.
- `lto`. `"thin"` gives gains similar to `"fat"` in much less link time. For a true no-LTO baseline use `lto = "off"`: `lto = false`, the default, still runs thin-local LTO.
- `opt-level`. `"s"` and `"z"` turn off the loop vectorizer; measure the hot loop at `3` (the `rust-hot-path` skill has the mechanism and the probe).

Read [references/build-configuration.md](references/build-configuration.md) when you change a build flag for speed or size: it has the LTO and strip tables, where Cargo reads each setting from, rustflags precedence, `target-cpu`, profile-guided optimization, and the global allocator.

## 9. Build time

Diagnose first:

```bash
cargo build --locked --timings           # writes target/cargo-timings/cargo-timing.html
cargo build --locked --release --timings
```

Read the timeline for long sequential chains, crates over 10 s, and proc-macro crates that block everything downstream.

Read [references/build-time-optimization.md](references/build-time-optimization.md) when `--timings` shows where the time goes: it has sccache, the cross-compilation target matrix, workspace splitting, linker choice, and the generic signature shape.

## Related skills

Other skills are named at their point of use above; each applies when it is installed. Two more apply here: `cargo-workflows` for workspace layout, feature flags, and profile plumbing, and `rust-observability` for tracing spans and structured timing in production.
