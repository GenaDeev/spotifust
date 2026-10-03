# Build configuration for runtime speed

Reference for [SKILL.md](../SKILL.md). It holds the build knobs that change how fast the shipped
binary runs: where Cargo reads settings from, the LTO and strip tables, `opt-level`, `target-cpu`,
profile-guided optimization, and the global allocator. SKILL.md section 8 holds the profile block
and the `panic`, `strip`, and `lto` rules. Compile-time tuning is in
[build-time-optimization.md](build-time-optimization.md).

Contents: 1. Where the settings are read; 2. LTO, codegen units, and strip; 3. opt-level;
4. target-cpu; 5. Profile-guided optimization; 6. The global allocator; 7. Order of work.

Every figure below was measured on rustc 1.97.0, host `aarch64-apple-darwin` (`apple-m4`), unless the
text names another target.

---

## 1. Where the settings are read

Get this wrong and every later measurement is noise.

### `[profile.*]` is read only from the workspace root

A profile table in a workspace member, or in a dependency, is discarded. For a workspace member, Cargo
prints one warning and builds anyway. For a dependency, Cargo prints nothing at all. A path dependency
that asks for `opt-level = 1` still received `-C opt-level=3`, with no diagnostic on the build output.
The workspace-member warning reads:

```text
warning: profiles for the non root package will be ignored, specify profiles at the workspace root:
package:   /path/to/workspace/member/Cargo.toml
workspace: /path/to/workspace/Cargo.toml
```

A library crate cannot ship optimization settings to its consumers. Put the settings in the workspace
root manifest and document them for downstream users.

### `RUSTFLAGS` in the environment replaces the config file

Cargo does not merge across rustflags levels. One level wins and the others are dropped whole. Verified
with `cargo build --release -v` against this `.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "target-cpu=native"]
```

| Command | Flags rustc received |
| --- | --- |
| `cargo build --release` | `-C target-cpu=native` |
| `RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release` | `-C force-frame-pointers=yes` only |

`[target.<triple>] rustflags` replaces `[build] rustflags` the same way. With both tables present, a
build received only the `[target.aarch64-apple-darwin]` entry, with and without an explicit `--target`.

Precedence, highest first: the `CARGO_ENCODED_RUSTFLAGS` environment variable, then `RUSTFLAGS`,
then the `[target.*]` tables, then `[build] rustflags`. The Cargo reference calls these four
mutually exclusive sources: the first one present is used and the rest are dropped.
`CARGO_ENCODED_RUSTFLAGS` also dropped a `[target.<triple>]` entry when re-checked on 1.98.1.

Inside the `[target.*]` level, entries do join. Cargo concatenates the matching `[target.<triple>]`
table and every matching `[target.'cfg(...)']` table. With the three tables below, rustc received
`-C target-cpu=native` and `-C force-frame-pointers=yes` together, and lost `-C debug-assertions=yes`:

```toml
[build]
rustflags = ["-C", "debug-assertions=yes"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "target-cpu=native"]

[target.'cfg(target_os = "macos")']
rustflags = ["-C", "force-frame-pointers=yes"]
```

Setting `RUSTFLAGS` on that same config dropped both target tables.

This bites the profiling workflow. The Linux profiling setup passes `-Wl,--no-rosegment` through
`CARGO_TARGET_<TRIPLE>_RUSTFLAGS`, the environment form of a `[target.<triple>]` entry. Cargo joins
it with the config-file target tables (checked on cargo 1.98.1). A `RUSTFLAGS="..."` prefix drops
it, and `perf` stacks break with no error. A `[build] rustflags` entry disappears as soon as any
matching `[target.*]` entry exists, from a table or from that variable. Keep every rustflags entry
at one level.

### `lto = false` is not `lto = "off"`

Thin-local LTO is the implicit default for any build with `opt-level > 0`. `lto = false` stops only the
cross-crate part. Verified with `cargo build --release -v`:

| `Cargo.toml` | Flags rustc received |
| --- | --- |
| `lto = false` | `-C embed-bitcode=no` |
| `lto = "off"` | `-C embed-bitcode=no -C lto=off` |

Anyone who benchmarks "LTO on against LTO off" by flipping `false` and `"thin"` compares thin-local LTO
against thin LTO. Use `lto = "off"` for the true baseline.

---

## 2. LTO, codegen units, and strip

| Setting | Link time | Use when |
| --- | --- | --- |
| `lto = "off"` | Fastest | The true no-LTO baseline for an A/B |
| `lto = false` (the default) | Fast | Thin-local LTO inside one crate; no LTO at all with `codegen-units = 1` or `opt-level = 0` |
| `lto = "thin"` | Moderate | Most release builds; gains similar to `"fat"` in much less link time |
| `lto = "fat"` | Slowest | The last bit of speed or size |
| `codegen-units = 1` | Slower compile | Better optimization; measure it with and without LTO |

| Setting | What stays in the file | Use when |
| --- | --- | --- |
| `strip = "symbols"` | No symbol table, no debug info | An artifact that never needs crash symbolication |
| `strip = "debuginfo"`, the default when `debug` is off | Symbol table only | Function names in crash reports and profiles |
| `strip = "none"` + `debug = "line-tables-only"` | Symbols and file:line tables | Profiling, and offline symbolication with line numbers |
| `strip = "none"` + `debug = true` | Everything, including variable info | Debugger sessions |

---

## 3. opt-level

`rustc -C help` states the range: `optimization level (0-3, s, or z; default: 0)`.

Cargo requires bare integers for the numeric levels; only `s` and `z` are quoted. `opt-level = 3` is
accepted. `opt-level = "3"` fails the manifest parse:

```text
error: must be `0`, `1`, `2`, `3`, `s` or `z`, but found the string: "3"
```

### Measure size on `__text`, not on the file

A Mach-O binary is page-padded, so whole-file byte counts move in steps and can invert the answer.

Measured on one program (`HashMap` plus sort plus `format!`), built with `panic = "abort"`,
`strip = true`, `codegen-units = 1`, an isolated target directory, `aarch64-apple-darwin`:

| opt-level | `__text` bytes | File bytes |
| --- | --- | --- |
| 3 | 224,292 | 358,336 |
| 2 | 223,512 | 358,336 |
| 1 | 223,424 | 358,400 |
| `"s"` | 223,136 | 358,400 |
| `"z"` | 222,364 | 358,432 |

The two columns disagree. Here `"z"` holds the smallest code section and the largest file. The file
column moves 96 bytes of padding across the five levels; `__text` moves 1,928 bytes of real code. Read
`__text` with `size -m <binary>` on macOS, or use `cargo bloat`.

The whole spread is under 1 percent of the code section, and a size level can still cost throughput.
Build the ship profile at `3`, at `"s"` and at `"z"`, then pick from your own numbers.

### `s` and `z` disable loop vectorization

A plain accumulate loop over `&[u32]`, marked `#[inline(never)]`, `aarch64-apple-darwin`. The count
is the number of vector operand suffixes (`.4s`, `.2d`, `.16b`, `.8h`) in the emitted assembly:

| opt-level | Vector ops |
| --- | --- |
| 0 | 0 |
| 1 | 0 |
| 2 | 15 |
| 3 | 15 |
| `s` | 0 |
| `z` | 0 |

`"s"` and `"z"` turn off the loop vectorizer; measure the hot loop at `3` (the `rust-hot-path`
skill has the mechanism and the probe, including the `#[inline(never)]` rule).

---

## 4. target-cpu

`-C target-cpu` raises the instruction-set baseline the compiler may use.

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --locked --release
```

`native` resolves to the host processor. `rustc --print target-cpus` names it on the first line: on
this machine, `native - Select the CPU of the current host (currently apple-m4).`

### Prove that it did something

Diff the cfg set. The flagged run must gain `target_feature` lines:

```bash
diff <(rustc --print cfg) <(rustc --print cfg -C target-cpu=native)
```

On this host the flagged run gained `target_feature="bf16"`, `target_feature="bti"` and
`target_feature="i8mm"`. An empty diff means the flag did nothing; name the features explicitly with
`-C target-feature=+<name>` instead.

### It is not a harmless no-op under `--target`

Two measured failure modes, both silent:

| Command | Result |
| --- | --- |
| `rustc --print cfg --target aarch64-linux-android -C target-cpu=native` | 31 `target_feature` lines, against 1 without the flag. The host's `apple-m4` features are applied to the Android build. |
| `rustc --print cfg --target x86_64-unknown-linux-gnu -C target-cpu=native` | `'apple-m4' is not a recognized processor for this target (ignoring processor)`, and `target_feature="fxsr"` drops out of the baseline set |

The first row is the dangerous one. Cross-compiling to Android from an Apple Silicon host with
`target-cpu=native` produces a library that assumes CPU features the phone does not have. It compiles,
it links, it ships, and it faults with an illegal instruction on the device.

Rules:

- Use `target-cpu=native` only for a binary that runs on the machine that built it: local benchmarks,
  or a service on fixed self-hosted hardware.
- Never put `target-cpu=native` under `[build] rustflags` in a repository that cross-compiles.
- For a distributed artifact, name a baseline instead. On x86_64 the levels are `x86-64` (the default),
  `x86-64-v2`, `x86-64-v3` and `x86-64-v4`. List the choices with
  `rustc --print target-cpus --target <triple>`.
- Re-run the cfg diff after any container or toolchain change. It is a two-second check.

---

## 5. Profile-guided optimization

`-C profile-generate` and `-C profile-use` are stable. Both appear in `rustc -C help` on 1.97.0. The
rustc book chapter "Profile-guided Optimization" is the primary reference.

The model is two passes: build instrumented, run on representative input, rebuild with the merged
profile.

```bash
# 1. Instrumented build. --target keeps build scripts out of the profile; see below.
RUSTFLAGS="-Cprofile-generate=$PWD/pgo-data" \
  cargo build --locked --release --target aarch64-apple-darwin

# 2. Run the representative workload. Each instrumented binary writes one .profraw
#    file and updates it in place on later runs.
./target/aarch64-apple-darwin/release/myapp --input real-workload.bin

# 3. Merge with the llvm-profdata that matches rustc's LLVM. rustup does not put it on PATH.
rustup component add llvm-tools
PROFDATA="$(rustc --print sysroot)/lib/rustlib/$(rustc --print host-tuple)/bin/llvm-profdata"
"$PROFDATA" merge -o merged.profdata pgo-data
"$PROFDATA" show merged.profdata    # "Total functions:" must be non-zero

# 4. Optimized build. The llvm-args flag warns for each function that has no profile data.
RUSTFLAGS="-Cprofile-use=$PWD/merged.profdata -Cllvm-args=-pgo-warn-missing-function" \
  cargo build --locked --release --target aarch64-apple-darwin
```

On 1.98.1, two runs of the instrumented binary left one `.profraw` file, `show` printed
`Total functions: 6` for a small program, and the flagged build warned `no profile data available
for function ...` for code the workload did not reach. The rustc book says an `llvm-profdata` from
a recent LLVM or Clang usually works too: `xcrun llvm-profdata` merged a 1.98.1 profile on macOS.
A `RUSTFLAGS` prefix replaces config-file rustflags for these builds (section 1).

### Two silent traps

**A `.profraw` passed to `-C profile-use` is a warning, not an error.** The build succeeds and applies
no PGO at all:

```text
warning: pgo-data/default_14157245456489944735_0.profraw: invalid instrumentation profile data (bad magic)
```

A missing file is a hard error, so only the un-merged case is silent. Grep the build output for
`bad magic` before you believe a PGO number. A clean build is not proof either: without
`-pgo-warn-missing-function`, LLVM says nothing when a function has no profile data.

**Without `--target`, the instrumented build also instruments build scripts.** Cargo passes `RUSTFLAGS`
to host artifacts when no target triple is given. Measured on a crate with a trivial `build.rs`: one
`.profraw` file appeared in the profile directory after the build and before the program ran at all,
and a second after the run. With `--target <host triple>` the count after the build was zero. Build
script data in the merge distorts the profile. Always pass `--target`.

### Where PGO does not reach

`cargo install` exposes no two-pass mechanism. `cargo install --help` offers only `--debug` and
`--profile <PROFILE-NAME>`. A binary distributed through crates.io and installed with `cargo install`
therefore cannot be PGO-optimized by its author. Ship a pre-built artifact if PGO matters.

`cargo-pgo` 0.3.0 wraps the sequence above and adds BOLT. It hides the two traps rather than removing
them, so reach for it only once the manual four steps work.

---

## 6. The global allocator

Rust uses the system allocator by default. Swapping it is one static item.

```rust,ignore
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

```rust,ignore
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

Current versions: `tikv-jemallocator` 0.7.0 (over `tikv-jemalloc-sys` 0.7.1, jemalloc 5.3.1) and
`mimalloc` 0.1.52 (over mimalloc 3.3.2). The older `jemallocator` crate is stale at 0.5.4; do not start
there. jemalloc covers Linux and macOS. mimalloc is portable.

The speed effect is platform-dependent, because it is a comparison against whatever system allocator
the platform ships. Measure it on each target. The cost is fixed and measurable up front.

Measured on the same `fn main` with `strip = true`, `panic = "abort"`, `codegen-units = 1`,
`aarch64-apple-darwin`. Build time is a cold release build with a fresh target directory:

| Allocator | Binary bytes | Delta | Cold build |
| --- | --- | --- | --- |
| System (default) | 341,344 | — | 0.3 s |
| `mimalloc` 0.1.52 | 443,840 | +102,496 | 3.1 s |
| `tikv-jemallocator` 0.7.0 | 605,104 | +263,760 | 40.5 s |

jemalloc builds a C library from source, which is where the 40 s goes. On a mobile target the binary
delta lands in the app bundle; check it with `cargo bloat --crates` against the ship profile.

### Confirm the allocator is actually linked

Both allocators announce themselves at run time, which is faster than reading a symbol table:

```bash
_RJEM_MALLOC_CONF=stats_print:true ./myapp    # jemalloc prints its statistics at exit
MIMALLOC_VERBOSE=1 ./myapp                    # mimalloc prints its options at startup
```

### jemalloc tuning: two different variables

`tikv-jemallocator` prefixes the jemalloc symbols, so the plain `MALLOC_CONF` name does not reach it.
The build-time and run-time knobs are separate variables, and mixing them up is the usual reason a
tuning attempt measures as no change.

| When | Variable | Verified behaviour |
| --- | --- | --- |
| Run time | `_RJEM_MALLOC_CONF` | `_RJEM_MALLOC_CONF=bogus:1 ./myapp` prints `<jemalloc>: Invalid conf pair: bogus:1` |
| Run time | `MALLOC_CONF` | Silently ignored |
| Build time | `JEMALLOC_SYS_WITH_MALLOC_CONF` | Forwarded by `build.rs` as jemalloc's `--with-malloc-conf=`. A binary built with `JEMALLOC_SYS_WITH_MALLOC_CONF=bogus:1` prints `<jemalloc>: Invalid conf pair: bogus:1` with no environment variable set at run time |
| Build time | `MALLOC_CONF` | Does nothing. The same build with `MALLOC_CONF=bogus:1` printed no jemalloc line |

The prefix is a crate feature. `tikv-jemallocator` 0.7.0 offers
`unprefixed_malloc_on_supported_platforms`, which moves the run-time name back to `MALLOC_CONF`. On
some platforms the feature does nothing. `tikv-jemalloc-sys` 0.7.1 lists `android`, `dragonfly` and
`apple` in `NO_UNPREFIXED_MALLOC_TARGETS` and turns the prefix back on for them, so on macOS and
Android the run-time name stays `_RJEM_MALLOC_CONF`. The build script reports this, but Cargo shows
build-script warnings only for path dependencies. Ask for them with `-vv`, which shows them for
every crate:

```bash
cargo build --locked --release -vv 2>&1 | grep -i unprefixed
```

Do not read Cargo's internal `build/*/output` files instead. Their location moves with
`build.build-dir`.

With the feature enabled, the `aarch64-apple-darwin` build script printed:

```text
cargo:warning="Unprefixed `malloc` requested on unsupported platform `aarch64-apple-darwin` => using prefixed `malloc`"
```

Transparent huge pages (`thp:always`) are Linux-only. On macOS the binary reports
`<jemalloc>: No THP support: thp:always` and continues. Do not carry a THP measurement from a Linux
server to a macOS or Android result.

---

## 7. Order of work

Each row costs more than the one above it. Stop when the metric is met.

| Step | Cost | Portability cost | Check that it worked |
| --- | --- | --- | --- |
| Confirm the settings reach rustc | Minutes | None | `cargo build --release -v`, read the rustc command line |
| `opt-level`, `lto`, `codegen-units` | Build time only | None | `cargo bloat`, Criterion baseline |
| `target-cpu` baseline (`x86-64-v3` and similar) | None at run time | Drops old CPUs | `diff` of `rustc --print cfg` |
| Swap the allocator | Binary size, build time | jemalloc: Linux and macOS only; mimalloc: none | `MIMALLOC_VERBOSE=1`, or jemalloc `stats_print` |
| PGO | A representative workload, plus CI plumbing | None | No `bad magic` warning; `llvm-profdata show` counts functions; hot functions draw no missing-profile warning |
| `target-cpu=native` | None | Binary runs on one machine class | `diff` of `rustc --print cfg` |

Every row needs a Criterion baseline before and after, per the rules of engagement in SKILL.md.
A build-configuration change is invisible in review, so an unmeasured one becomes a permanent unexplained setting.
