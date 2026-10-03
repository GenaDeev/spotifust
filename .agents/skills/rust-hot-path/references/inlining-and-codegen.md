# Inlining and codegen inspection

Measured inlining thresholds, the compile-time price of `#[inline]`, the vectorization and
bounds-check probes, and the commands that show what the compiler did. It serves the Inlining
and Bounds checks sections of [SKILL.md](../SKILL.md), which give the four attribute forms and
the rule that every attribute needs a number.

All figures come from rustc 1.97.0 unless the text names 1.98.1. The host is
`aarch64-apple-darwin` unless the text names another target.

Sections: where the inliner stops and how to probe it; the compile-time price of `#[inline]`;
what each attribute becomes; `#[cold]` and `cold_path`; IR, symbol, and assembly checks;
proving the win; the hot and cold split, and outlining; special-casing small sizes;
vectorization and SIMD; bounds-check probes; triage.

## Where the inliner actually stops

A crate boundary is the barrier. A codegen-unit boundary inside one crate is not.

| Callee | Build | Symbol left in the caller crate |
| --- | --- | --- |
| 8-line `pub fn`, no attribute | `lto = false`, `codegen-units = 16` | None. Inlined |
| 60-line `pub fn`, no attribute | `lto = false`, `codegen-units = 16` | `declare noundef i32 @_RNvCs..._3dep5plain` plus a call |
| 60-line `pub fn`, `#[inline]` | `lto = false`, `codegen-units = 16` | None. Inlined |
| 60-line `pub fn`, no attribute | `lto = true` | None. Inlined |
| 60-line generic `pub fn`, no attribute | `lto = false`, `codegen-units = 16` | None. Inlined |
| 60-line private `fn` in another module of the same crate | `lto = false`, `codegen-units = 16` | None. Inlined |

The 60-line row reproduces on 1.98.1 with a body of `x = x + k` and `x = x ^ k` lines on a
`u32`: `--emit=mir` counts 121 statements plus the return, over the budget of rule 1, and `nm`
shows the symbol. The first 8 of those lines count 17 plus the return, under the budget.

Read the table as four rules.

1. A small non-generic `pub fn` crosses a crate boundary with no attribute when rustc infers it
   cross-crate-inlinable. At the 1.98.1 tag
   (`compiler/rustc_mir_transform/src/cross_crate_inline.rs`) the function's optimized MIR must
   hold no call (intrinsics excepted), no drop of a non-trivial value, no unwind cleanup or
   resume, and at most 100 statements plus terminators. `StorageLive`, `StorageDead`, and `Nop`
   do not count. The budget is `-Z cross-crate-inline-threshold`, default 100. Count MIR, not
   source lines: each `x = x + k` line lowers to two MIR statements. The MIR probe below gives
   the count.
2. `#[inline]` is the switch that ships the MIR of a large non-generic function to downstream
   crates. Nothing else does, short of LTO.
3. `#[inline]` on a generic function is redundant for this purpose. The downstream crate
   monomorphizes the body itself, so it already holds the MIR.
4. `lto = true` inlines everything the budget allows, regardless of the attribute. Do not add
   `#[inline]` to a crate that only ever builds under fat LTO.

The last row matters when a fix looks like a no-op. Splitting a hot function into two modules of
the same crate does not add a barrier. Moving it into its own crate does.

Rule 1 depends on the build. rustc skips the inference in incremental builds and at
`opt-level = 0`. The MIR inliner, which removes small calls such as `u32::wrapping_mul` before
the check, runs only at `opt-level` 2 and 3 in a non-incremental build. Measured on 1.98.1: a
leaf built from operators, `(x ^ 0x5555) >> 3`, crossed at opt-level 1, 2, 3, `"s"`, and
`"z"`. A leaf that calls `x.wrapping_mul(31)` crossed only at 2 and 3.

### Probe one function

A body that qualifies is instantiated per caller, so it leaves no symbol in its own rlib. That
makes the defining crate alone enough to answer the question:

```bash
cargo build --release -p dep
nm -g target/release/libdep.rlib | grep '_R'
```

No line for the function: the body crosses the boundary already, and `#[inline]` buys nothing.
A line for the function: the body does not qualify, and only `#[inline]` or LTO gets it across.
Run the probe again after each edit to the body. `target/` is the default target directory;
use yours when `CARGO_TARGET_DIR` or `build.target-dir` moves it.

When the symbol stays, count the function's MIR in the same profile to see why:

```bash
touch dep/src/lib.rs
cargo rustc --release -p dep -- --emit=mir=dep.mir
```

Find `fn <name>(` in `dep.mir` and read its `bb` blocks. A terminator of the form
`_2 = helper(move _1) -> [return: bb1, ...]` is a call, and a call to anything but an intrinsic
disqualifies the function. Otherwise count every line in the blocks except `StorageLive`,
`StorageDead`, and `nop`, terminators included. More than 100 disqualifies it. rustc writes
`dep.mir` in the workspace root, and it writes nothing when the crate is fresh, so keep the
`touch`.

Run the probe in the profile you ship. Measured on 1.98.1:

| Build | What the probe shows |
| --- | --- |
| `--release` with the default `incremental = false` | The true answer |
| `CARGO_INCREMENTAL=1`, or the `dev` profile | A symbol for every function, so every helper reads as too large |
| A release profile with LTO, on macOS | Xcode `nm` fails with `Unknown attribute kind`: the rlib holds bitcode from a newer LLVM |

The last case needs no answer, because LTO ignores the boundary (rule 4). Probe it with
`--config 'profile.release.lto=false'`.

## The compile-time price of `#[inline]`

`#[inline]` makes every downstream crate compile the body again. It does not make the defining
crate slower: the attribute switches the function to per-caller instantiation, so the defining
crate stops emitting it. A 60-line `#[inline] pub fn` left no symbol in its own rlib at
all.

Measured on a dependency with 40 `pub fn` of 60 lines each, and one downstream crate that
calls all 40. rustc invoked directly, minimum of 9 runs, `aarch64-apple-darwin`:

| Build | Defining crate | Downstream rebuild |
| --- | --- | --- |
| No attribute | 0.12 s | 0.14 s |
| `#[inline]` on all 40 | 0.10 s | 0.16 s |

That is roughly 1.1x downstream at 40 functions, and 1.6x at 200 functions, where the same
rebuild moved from 0.16 s to 0.26 s. Every downstream crate pays again, so a `#[inline]` in a
utility crate is billed to the whole workspace. Add the attribute to the functions a profile
names, and to nothing else.

## What each attribute becomes

| Rust | LLVM effect | Notes |
| --- | --- | --- |
| `#[inline]` | `inlinehint` on the definition | Raises the budget. Not a command |
| `#[inline(always)]` | Forced inlining | The function usually leaves no symbol |
| `#[inline(never)]` | `noinline` on the definition | Survives as its own symbol |
| `#[cold]` | `cold` on the definition | Also produces call-site branch weights |

**No attribute is transitive.** A caller's attribute never reaches its callees. Verified with
`#[inline(always)] fn f()` that calls `#[inline(never)] fn g()`: `f` disappeared into `main` and
`g` stayed as `_RNvCs..._5trans1g`. If a measurement shows an `#[inline(always)]` changed
nothing, look at the callees first.

## `#[cold]` and the branch weight

`#[cold]` lowers to a function attribute on the definition, not to a call-site attribute:

```text
attributes #1 = { cold mustprogress nofree norecurse nosync nounwind willreturn memory(none) ... }
```

Note what is not in that set: `#[cold]` does not imply `noinline`. A small cold callee is still
inlined into its caller, so `#[cold]` on its own does not shrink the hot function. Pair it with
`#[inline(never)]` when the point is to get the body out of the caller.

What `#[cold]` does buy is the call-site weight. LLVM derives it at each call site and lays the
cold edge out of line:

```text
!2 = !{!"branch_weights", !"expected", i32 1, i32 2000}
```

So one attribute on the definition biases every caller.

### `cold_path` marks one branch

`core::hint::cold_path()` (Rust 1.95) marks a branch cold without a separate function. Call it
as the first statement of the rare arm:

```rust
pub fn parse_digits(bytes: &[u8]) -> Result<u32, usize> {
    let mut total = 0u32;
    for (i, &c) in bytes.iter().enumerate() {
        if !c.is_ascii_digit() {
            core::hint::cold_path();
            return Err(i);
        }
        total = total.wrapping_mul(10).wrapping_add(u32::from(c - b'0'));
    }
    Ok(total)
}
```

Measured on 1.98.1 at `-O`, the branch into that arm carries the weight:

```text
!5 = !{!"branch_weights", i32 4000000, i32 4001}
```

The arm's code stays in the function. Use `cold_path` when only the branch layout matters. Use
`#[cold]` with `#[inline(never)]` when the rare code must leave the hot function.

## Show what the compiler did

### LLVM IR is the cheapest check

```bash
touch src/main.rs
cargo rustc --release -p app -- --emit=llvm-ir=app.ll -C codegen-units=1

# Did the dependency's code survive as a call?
grep -nE '^(declare|define).*_3dep' app.ll
```

rustc writes `app.ll` relative to the directory cargo runs it in, the workspace root. Keep
`-C codegen-units=1`: with more than one unit rustc prints `ignoring emit path because multiple
.ll files were produced` and leaves one file per unit in the build directory, a Cargo-internal
path that moves with `build.build-dir`. One unit does not change the cross-crate answer,
because a codegen-unit boundary is not an inlining barrier.

Read the result with three rules:

| In the caller's IR | Meaning |
| --- | --- |
| No mention of the symbol | The body was inlined |
| `declare ... @<symbol>` plus a `call` | Not inlined. The call crosses the boundary |
| `define internal` or `define hidden ... @<symbol>` | The body was imported or monomorphized here, and a call may still remain. Run `nm` on the linked binary to see whether it survived |

The `touch` is not optional. `cargo rustc` runs the compiler only when the crate is stale. A
second run on an unchanged crate prints `Finished` in 0.00 s and writes no `.ll` at all, which
reads exactly like a clean result.

### Symbols in the linked binary

Faster than IR when you only need a yes or no answer.

```bash
cargo build --release
nm target/release/app | grep '_R'
```

A function that was inlined everywhere leaves no symbol. In the 60-line test above, `nm`
printed exactly one line for the dependency, and it was the function with no attribute.

**Expect `_R`, not `_ZN`.** v0 mangling is the default since 1.97. Mach-O adds one leading
underscore, so the same symbol reads `__RNv...` on macOS and `_RNv...` on ELF; grep `_R` to
match both. The `rust-debugging` skill covers mangling and demanglers.

### Assembly for one function

```bash
cargo install --locked cargo-show-asm
cargo asm --release --lib bounds::sum_sliced      # a function in the library target
cargo asm --release --bin my_bin some::function   # a function in a binary target
```

The crate is `cargo-show-asm`, and the subcommand is `cargo asm`. The names differ, which is the
usual mistake. The older `cargo-asm` crate is a separate unmaintained project.

The target is a flag, never a positional. The two positionals are a name filter and an index, so
`cargo asm --release my_bin bounds::sum_sliced` aborts with `Error: Multiple targets found` in
any crate that has both a library and a binary. Run the command with no function path first; it
lists the matching symbols and you copy one back.

Use Compiler Explorer (godbolt.org) for an isolated snippet. Use `cargo asm` when the answer
depends on the real crate graph and the real profile, which is every cross-crate question.

### Cachegrind tells you the same thing from a run

Linux only. Valgrind has no aarch64-macOS target, so this rule is documented and not measured
here.

```bash
valgrind --tool=cachegrind --cache-sim=no --branch-sim=no ./target/release/app
cg_annotate --auto=yes cachegrind.out.<pid> > annotated.txt
```

In the annotated source, a function was inlined into its callers if and only if its first and
last lines carry no event counts. Body lines carry counts either way, so read the braces, not
the body.

## Prove the change is a win

Benchmark setup, Criterion baselines, and CI regression gates live in the `rust-performance`
skill. One point is specific to inline attributes: they move code layout, and a layout shift
moves wall-clock time by an amount unrelated to the change. That shift repeats on every run of
the same build, so Criterion can report it as significant. Confirm a small wall-clock win with
instruction counts (Gungraun on Linux) before you keep the attribute.

## Split hot from cold, and outline the rare path

When one call site of a large function is hot, keep the body in an `#[inline(always)]` function,
and give the cold call sites an `#[inline(never)]` wrapper around it. They then pay no code
bloat.

Outlining is the reverse form. Inlining pulls a callee in; outlining pushes a rare path out, so
the hot function gets small enough for the inliner to accept it.

```rust
pub struct Cache {
    entries: Vec<(u32, u32)>,
}

impl Cache {
    // Rare: a miss also re-sorts the table. Out of line, and out of the
    // hot function's inlining budget.
    #[cold]
    #[inline(never)]
    fn insert_slow(&mut self, key: u32) -> u32 {
        let value = key.wrapping_mul(2654435761);
        self.entries.push((key, value));
        self.entries.sort_unstable_by_key(|e| e.0);
        value
    }

    pub fn get(&mut self, key: u32) -> u32 {
        match self.entries.binary_search_by_key(&key, |e| e.0) {
            Ok(i) => self.entries[i].1,
            Err(_) => self.insert_slow(key),
        }
    }
}
```

## Special-case the sizes that dominate

When small inputs dominate, handle 0, 1 and 2 elements ahead of the general loop. The general
loop then never pays for its own setup on the common call.

Measure the distribution first. Guessing it is how a special case ends up slower than the loop
it replaced. Count the arms in a debug build or behind a feature:

```rust
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(debug_assertions)]
static ARMS: [AtomicU64; 4] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

pub fn total(values: &[u32]) -> u32 {
    #[cfg(debug_assertions)]
    ARMS[values.len().min(3)].fetch_add(1, Ordering::Relaxed);
    values.iter().sum()
}
```

The gate is the point. Without it the `atomicrmw` survives `-O`, and the counter costs one
atomic read-modify-write on every call in release.

Run the real workload, print the four counts, and keep the special case only for the arms that
carry the traffic. Write the measured share into a comment next to the match, as SKILL.md
requires.

## When inlining is not the answer

A loop that the compiler will not vectorize does not get faster from an attribute.

Check the optimization level first. SKILL.md *Bounds checks* states the opt-level rule. Its
source is `compiler/rustc_codegen_ssa/src/back/write.rs` at the 1.98.1 tag. Measured on 1.98.1
with a `zip` loop that adds two `u32` slices: 5 NEON vector instructions at 2 and 3, and 0 at 1,
`"s"`, and `"z"`.

```bash
rustc -C opt-level=3 --emit asm --crate-type=lib vec.rs -o vec.s
grep -cE '\.4s|\.16b|\.2d' vec.s      # aarch64 NEON arrangement suffixes
```

```bash
rustc --target x86_64-unknown-linux-gnu -C opt-level=3 --emit asm --crate-type=lib vec.rs -o vec.s
grep -cE '^\s+v?padd[bwdq]' vec.s   # x86_64 packed integer adds; match the op to the element type
```

Do not count `%xmm` registers: they also move plain copies. Measured on 1.98.1: 2 `paddd` at 2
and 3, 0 at 1, `"s"`, and `"z"`.

A count of 0 at 1, `"s"`, or `"z"` is the profile, not the code.

When the loop stays scalar at `opt-level` 3 as well, `core::arch` is the next step. It is stable
and works in `no_std`. Measured on 1.98.1, a `vaddq_u32` function gave 1 NEON instruction at 1,
2, 3, `"s"`, and `"z"`. Baseline features of the target need no nightly and no
`#[target_feature]`. Verified on stable 1.97.0, aarch64:

```rust
#[cfg(target_arch = "aarch64")]
pub fn splat_seven() -> [u8; 16] {
    let mut out = [0u8; 16];
    // SAFETY: both intrinsics are baseline NEON on every aarch64 target,
    // and out holds the 16 bytes vst1q_u8 writes.
    unsafe {
        let v = core::arch::aarch64::vdupq_n_u8(7);
        core::arch::aarch64::vst1q_u8(out.as_mut_ptr(), v);
    }
    out
}
```

Two traps sit on the path past that point.

**A non-baseline feature needs the attribute at every level.** Enabling the feature in the build
configuration does not remove the requirement. The compiler says so (x86_64 shown; `dotprod` on
aarch64 fails the same way, measured on 1.98.1):

```rust,compile_fail,E0133
#[target_feature(enable = "avx2")]
pub fn dot(a: u32) -> u32 {
    a
}

// error[E0133]: call to function `dot` with `#[target_feature]` is unsafe
//   and requires unsafe block
// With -C target-feature=+avx2 the error adds:
//   = note: the avx2 target feature being enabled in the build configuration
//     does not remove the requirement to list it in `#[target_feature]`
pub fn caller(a: u32) -> u32 {
    dot(a)
}
```

Mark the caller with the same `#[target_feature]`, or call it from an `unsafe` block that a
runtime `is_x86_feature_detected!` guard (`is_aarch64_feature_detected!` on aarch64) protects.

**Portable SIMD is still nightly.** `std::simd` needs a feature gate, and the gate fails on
stable with E0554:

```rust,ignore
#![feature(portable_simd)]      // error[E0554]: `#![feature]` may not be used
                                // on the stable release channel
use std::simd::u8x16;
```

Write the intrinsics per architecture behind `#[cfg(target_arch = ...)]`, and keep a plain scalar
fallback for every other target.

## Bounds checks: the probe evidence

SKILL.md *Verify and pin the win* gives the assembly probe and the `#[inline(never)]` rule. The
evidence follows. A small non-generic `pub fn` with no caller is never emitted at `-O`, because it
qualifies for cross-crate inlining (rule 1 above) and is instantiated per caller instead. Without
the attribute, the `naive` probe below produces a 54-byte file that holds two directives, and
`grep -c` prints 0. With `#[inline(never)]` the same source emits the body and `grep -c` prints
1.

Measured on 1.97.0, aarch64-apple-darwin, each function compiled alone with
`#[inline(never)]`: `naive` printed 1, and the three shapes below printed 0.

```rust
// Keeps the check: the loop bound and the length are unrelated values.
#[inline(never)]
pub fn naive(v: &[u32], n: usize) -> u32 {
    let mut t = 0;
    for i in 0..n { t += v[i]; }
    t
}

// 1. Reslice first, so the length and the loop bound are the same value.
#[inline(never)]
pub fn resliced(v: &[u32], n: usize) -> u32 {
    let s = &v[..n];
    let mut t = 0;
    for i in 0..n { t += s[i]; }
    t
}

// 2. Assert the range once, ahead of the loop.
#[inline(never)]
pub fn asserted(v: &[u32], n: usize) -> u32 {
    assert!(n <= v.len());
    let mut t = 0;
    for i in 0..n { t += v[i]; }
    t
}

// 3. Iterate. Preferred: no index exists to check.
#[inline(never)]
pub fn iterated(v: &[u32]) -> u32 {
    v.iter().copied().sum()
}
```

SKILL.md *Bounds checks* gives `as_chunks::<N>()` as a fourth shape, with the Clippy rule. The
lint suggests `as_chunks::<N>().0`, and the `.1` half is the leftover that `remainder()` used to
return. Below a `rust-version` of 1.88 Clippy stays silent and `as_chunks` is not available, so
keep `chunks_exact`. For a size known only at run time, `chunks_exact` stays, and its leftover
keeps asymmetric names: `remainder()` on `chunks_exact`, `into_remainder()` on
`chunks_exact_mut`:

```rust,run
fn main() {
    let data = [1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let (chunks, rest) = data.as_chunks::<4>();
    let mut sum = 0;
    for c in chunks {
        sum += c[0] + c[1] + c[2] + c[3];
    }
    assert_eq!((sum, rest), (36, &[9, 10][..]));

    // A size known only at run time: chunks_exact stays.
    let n = std::hint::black_box(4);
    assert_eq!(data.chunks_exact(n).remainder(), &[9, 10]);
    let mut buf = [0u8; 10];
    let mut it = buf.chunks_exact_mut(n);
    for c in &mut it {
        c[0] = 1;
    }
    assert_eq!(it.into_remainder().len(), 2);
}
```

## Triage

| Symptom | Cause | Fix |
| --- | --- | --- |
| A bounds-check probe finds nothing, and the function is small | The `pub fn` was never emitted | Add `#[inline(never)]` to the probe |
| `#[inline(always)]` measured as no change | The attribute is not transitive; the callee still stands | Mark the callee too, and confirm with `nm` |
| A dependency function shows in a profile after a refactor | The code moved into its own crate, and the body does not qualify for cross-crate inlining | Add `#[inline]`, or turn on LTO |
| Every function in the rlib keeps a symbol | Incremental or `dev` build: rustc skips the inference | Probe with `--release` and `incremental = false` |
| A helper crosses at `opt-level = 3` and stops at `"s"` or `"z"` | The MIR inliner does not run at opt-level 1, `"s"`, or `"z"`, so a small call stays in the body | `#[inline]` on the helper, or `opt-level = 3` for that crate |
| `nm` prints `Unknown attribute kind` on an rlib | Xcode `nm` reads LTO bitcode from a newer LLVM | Probe with `--config 'profile.release.lto=false'` |
| Adding `#[inline]` made no difference at all | The build already uses `lto = true` | Remove the attribute and keep the compile time |
| `cargo rustc -- --emit=llvm-ir=app.ll` writes no `.ll` | The crate was fresh, so rustc never ran | `touch` a source file and repeat |
| `ignoring emit path because multiple .ll files were produced` | More than one codegen unit | Add `-C codegen-units=1` |
| A grep for `_ZN` in a profile finds nothing | v0 mangling is the default since 1.97.0 | Grep `_R`, and allow the extra Mach-O underscore |
| A loop has no vector instructions in the release build | `opt-level` 1, `"s"`, or `"z"` disables the loop vectorizer | `opt-level = 3` for that crate, or `core::arch` intrinsics |
| A benchmark shows a large, repeatable, unexplainable win | Wall clock moved with the code layout | Re-measure with instruction counts; see the `rust-performance` skill |
| `cargo asm` is not a command | The installed crate is `cargo-asm`, not `cargo-show-asm` | `cargo install --locked cargo-show-asm` |
