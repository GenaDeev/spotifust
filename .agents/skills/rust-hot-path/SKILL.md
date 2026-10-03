---
name: rust-hot-path
description: Use when changing Rust code at a hot spot that a profile or benchmark already names, for example cutting allocations (with_capacity, reserve_exact, clone_from, workhorse buffers, SmallVec, ThinVec, swap_remove), shrinking a type (print-type-sizes, memcpy, large_enum_variant), choosing a hasher (FxHashMap, HashDoS), removing a bounds check, placing inline(always) or cold attributes or a cold path hint, or buffering I/O (BufWriter). Not for choosing or running a profiler; use `rust-performance`. Triggers on "reduce allocations", "too many allocations", "this type is too big", "which hasher".
license: BSD-3-Clause
---

# Rust hot path

What to change in the code after a profile names the hot spot. The `rust-performance` skill
produces the profile and owns benchmark setup; this skill turns the profile into a diff. Apply a
rule here only when a measurement points at it: optimized code is harder to read, so each change
must pay for the readability it spends.

Figures were measured on rustc 1.97.0, aarch64 and x86_64, unless a line names another version.
Std growth policy and layout are unspecified implementation details. Re-measure them on your
toolchain before you depend on an exact figure.

## Route the profile to a section

| The profile shows | Change | Section |
| --- | --- | --- |
| `SipHasher`, `hashbrown` hot | Hasher choice | [Lookups](#lookups) |
| `write`, `read` syscalls dominate | Missing buffering | [I/O](#io) |
| `malloc`, `free`, `__rust_alloc` hot | Allocation rate | [Allocation rate](#allocation-rate) |
| `memcpy` hot with no obvious copy | A type crossed the inline-copy boundary | [Type size](#type-size) |
| `memmove` under `Vec::remove` or `Vec::insert` | Each call shifts the tail. Use `swap_remove` when order does not matter, `VecDeque` for a queue, one `retain` for many removals | — |
| `core::panicking::panic_bounds_check` in the disassembly | Bounds checks the compiler could not remove | [Bounds checks](#bounds-checks) |
| A scalar loop where you expected vector code | Bounds checks, or `opt-level` 1, `"s"`, or `"z"` | [Bounds checks](#bounds-checks) |
| Function entry and exit costs, many small calls | Inlining | [Inlining](#inlining) |
| Nothing stands out, the work itself is the cost | Algorithm or data structure. Stop here | — |

The last row is the common one. A better algorithm beats every rule below. Reach for this
skill after you accept the algorithm.

## Done when

- The change names a profile, a benchmark delta, or a measured distribution. No number, no merge.
- The metric is stated: allocations per operation, bytes, ns per item, or instructions.
- The measurement came from the shipped profile: non-incremental, at its real `opt-level`, not
  from `dev`.
- A hasher swap states where the keys come from.
- An `unsafe` shortcut such as `get_unchecked` carries a SAFETY comment and a benchmark that
  justifies it.
- A new inline attribute carries a before and after number, because the inliner's budget is
  global and forcing one function in can push a neighbour out. Reject one in review that has none.
- The win has a guard: a `const` size assert, a `dhat` count, or a Criterion baseline.
- Every deliberate special case carries a comment that names its measurement, or the next reader
  simplifies it away. Name the share and the workload, as in
  `// 99% of calls carry 0 or 1 elements (measured 2026-08, ingest benchmark)`.

## Verify and pin the win

Each check proves one claim, and each has a trap that prints a clean result for the wrong reason.

```bash
# A bounds check is gone: the count of checks left in the probe file.
rustc -O --emit asm --crate-type=lib probe.rs -o out.s && grep -c 'panic_bounds_check' out.s

# The layout of a type: discriminant, variants largest first, fields, padding. Nightly only.
# Scoped to one crate; the RUSTFLAGS form dumps every dependency and invalidates the cache.
touch src/lib.rs && cargo +nightly rustc --release -q -- -Zprint-type-sizes

# A dependency function crosses the crate boundary with no attribute: no line names it.
cargo build --release -p dep && nm -g target/release/libdep.rlib | grep '_R'
```

- Mark every probe function `#[inline(never)]`. Without it a small `pub fn` with no caller is
  not emitted at all: the file comes out 54 bytes long, `grep -c` prints 0, and that reads
  exactly like a removed check.
- Keep the `touch`. The dump is a compile-time side effect, not an artifact, so a second run of
  an unchanged crate prints nothing, and that reads exactly like a clean result. Pipe the dump
  through `top-type-sizes` on a real crate to compact it.
- Run the `nm` probe in a non-incremental release build. Incremental and `dev` builds skip the
  inference, so every helper reads as "not inlinable".

An optimization that nothing guards is removed by the next refactor. Pin a type size at compile
time: a mismatch fails the build with E0080. Gate it on one architecture, because sizes differ
per target. `size_of` is in the prelude since Rust 1.80; write `std::mem::size_of` for an older
MSRV.

```rust
pub struct Header { id: u64, flags: u32 }

#[cfg(target_arch = "aarch64")]
const _: () = assert!(std::mem::size_of::<Header>() == 16);
```

Pin an allocation count with a `dhat` test: one heap test per integration test file, each with
its own `#[global_allocator]`, because dhat panics when two profilers run at once and libtest
runs tests in parallel. Read [references/allocation-reduction.md](references/allocation-reduction.md)
when you write the `dhat` test: it holds the test file, the `--test-threads=1` fallback, and
the testing-mode rule. Keep that allocator out of `src/`, because in a library it
replaces the allocator of every binary that links it. Pin a time with a Criterion baseline
through the `rust-performance` skill.

## Lookups

The default hasher is SipHash 1-3. It resists collision flooding and it is slow for short
keys. Replacing it is the largest single win available on a hash-heavy workload, and it
removes a security property.

| Keys come from | Hasher | Why |
| --- | --- | --- |
| Anything a caller outside the process controls: headers, query strings, JSON keys, archive entry names | Keep std `RandomState` | The random per-process seed is the HashDoS defence |
| Untrusted keys, and hashing is measured hot | `ahash::RandomState` | Fast and still randomly seeded per process |
| Internal keys: interned symbols, node indices, enum tags | `rustc_hash::FxHashMap` | Fastest measured, unseeded |
| Counters and dense integer ids | `nohash_hasher::IntMap` | Identity hash. Wrong for ids with constant low bits |

On 1M inserts plus lookups, `FxHasher` was 4.7-5.0x faster than SipHash on `u64` keys and
2.2-2.4x on 18-byte string keys. `fnv` is not the middle option its reputation suggests: it
was slower than `FxHasher` on integers and only about 1.2x faster than SipHash on strings.

`FxHashMap` is a type alias for `HashMap` with a different hasher, and std supplies `new` and
`with_capacity` only for `RandomState`, so both fail with E0599. Write `FxHashMap::default()`
or `HashMap::with_capacity_and_hasher(n, FxBuildHasher)`.

Enforce one choice across a workspace with `disallowed-types` in `clippy.toml`. Banning
`std::collections::HashMap` by path does not flag `FxHashMap`, even though the alias is
that type: clippy matches the written path.

Read [references/hashing-and-io.md](references/hashing-and-io.md) when you need the full hasher
measurements, the two-process seeding check, the collision failure mode, the `nohash-hasher`
key rules, or the byte-wise `ByteHash` derives.

## I/O

**File writes are unbuffered.** A `writeln!` to a `File` costs at least one `write` syscall,
and more when the template interpolates. A `BufWriter` took 300,000 lines from about a second
to under ten milliseconds on one local filesystem. The absolute times move with the
filesystem, so re-measure them on yours.

**Locking stdout does not help on its own.** `Stdout` is a `LineWriter`, so it issues one
syscall per newline, lock or no lock. Over 300,000 `println!` calls a `BufWriter` around the
lock cut the time 25x to a terminal and about 50x redirected to a file.

```rust
use std::io::{BufWriter, Write};

fn dump(lines: &[&str]) -> std::io::Result<()> {
    let mut out = BufWriter::new(std::io::stdout().lock());
    for line in lines {
        writeln!(out, "{line}")?;
    }
    out.flush()          // dropping a BufWriter discards this error
}
```

Always end a `BufWriter` with an explicit `flush()?`, because a dropped one loses its final
write error and the file ends truncated with nothing in the log. `into_inner()` is not a
substitute: it drains the buffer without flushing the inner writer.

The default buffer is 8 KiB. Use `with_capacity` when one logical record is larger, or a
single record costs several syscalls.

**`BufRead::lines` allocates one `String` per line.** Read with `read_line` into one `String`
that you `clear` each iteration: 201 allocations became 2 on a 200-line file. `read_line` keeps
the line terminator. Remove it with `strip_suffix`, never with `trim_end()`, which also deletes
the trailing spaces and tabs that `lines()` keeps; the loss surfaces later as a parser fault.
Read [references/hashing-and-io.md](references/hashing-and-io.md) when you rewrite a reader: it
holds the byte-exact `strip_eol` helper and the `read_until` trade.

## Allocation rate

**`Vec` does not double from 4.** The first non-zero capacity depends on the element size,
and each growth then doubles. Measured on rustc 1.97.0, it is 8 for 1-byte elements (`Vec<u8>`,
`String`), 4 for 2 to 1024 bytes (`Vec<u32>`, `Vec<u64>`), and 1 above 1024 bytes. Twenty
`push` calls on a `Vec<u32>` therefore cost four allocations and end at capacity 32, with
twelve slots of waste. One `Vec::with_capacity(20)` costs one allocation and no waste.

**`reserve` and `reserve_exact` differ only on a non-empty vector.** `reserve` applies the
amortized policy on top of your request. `reserve_exact` does not.

```rust
let mut a: Vec<u32> = vec![1, 2, 3];
a.reserve(1);
assert_eq!(a.capacity(), 6);        // max(requested, 2 * capacity)

let mut b: Vec<u32> = vec![1, 2, 3];
b.reserve_exact(1);
assert_eq!(b.capacity(), 4);        // exactly len + additional
```

On an empty vector both give the exact request, so a test that starts from `Vec::new()`
shows them as identical and hides a 50% overshoot in the real path. Use `reserve_exact`
when you know the final length and you are memory-bound. Use `try_reserve` when the size
comes from untrusted input; the `rust-panic-safety` skill covers it.

**`clone_from` reuses the destination buffer.** `a = b.clone()` allocates every time.
`a.clone_from(&b)` writes into the buffer `a` already holds.

```rust
let mut dst: Vec<u32> = Vec::with_capacity(99);
let src: Vec<u32> = vec![1, 2, 3];
dst.clone_from(&src);
assert_eq!(dst.capacity(), 99);     // the 99-element buffer survives
```

Copying a three-element `Vec<u32>` onto a reused destination 1000 times cost 1001
allocations through `clone()` and 2 through `clone_from`. The two forms look identical in
review, so the lint is the practical defence: `assigning_clones` is in clippy's `pedantic`
group and is off under a plain `cargo clippy`. `Vec::clone()` also drops the reserve: a
vector of capacity 100 and length 3 clones to capacity 3, so a clone never hands a pre-warmed
buffer to another owner.

**Keep one workhorse buffer.** Declare the buffer outside the loop and `clear` it at the top of
each iteration. `clear` keeps the allocation; assigning a fresh collection throws it away. Do
not call `shrink_to_fit` in such a loop: it reallocates every time, which turns a
zero-allocation loop back into one allocation per iteration. It is a footprint tool, never a
speed tool.

**Make `collect` exact.** A hand-written iterator's default `size_hint` is `(0, None)`, so every
downstream `collect` and `extend` runs the growth ladder: 10,000 items cost 13 allocations
without a hint and 1 with an exact one. The `rust-iterator-impl` skill holds the impl and the
`ExactSizeIterator` contract. An inexact adaptor has the same effect: `(0..1000)` collects at
capacity 1000, and after a `filter` its 500 survivors land at capacity 512 through the whole
ladder. When you know the output length, use `with_capacity` plus `extend`.

**Format into one reused buffer.** `format!` allocates a `String` per call. `write!` into one
cleared buffer. For one integer, call `n.format_into(&mut buf)` on one `core::fmt::NumBuffer`
per integer type (Rust 1.98), declared outside the loop.

Read [references/allocation-reduction.md](references/allocation-reduction.md) when the
allocation is a `HashMap` reserve, a `collect` chain, a zero-filled buffer, an inline-capacity
type such as `SmallVec` or `ArrayVec`, an `Rc`/`Arc` refactor, a formatting loop, or an eager
`ok_or` argument.

## Type size

A type that is instantiated often is worth shrinking. Two thresholds matter.

**The inline-copy boundary.** Above it, a move or copy becomes a `memcpy` call. It is
target-dependent, and the widely quoted 128 bytes is the x86_64 number only.

| Target | Copies inline up to | First size that calls `memcpy` |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | 128 bytes | 129 bytes |
| `aarch64-apple-darwin` | 256 bytes | 257 bytes |

Measure at `-O`. At `opt-level = 0` the boundary drops to 32 bytes on both targets, so a
debug build tells you nothing about the shipped one.

**Cache lines.** A type scanned in a loop wants to fit one 64-byte line. This threshold is
far below the copy boundary and is the one that usually decides a scan-heavy workload.

**Box the outsized variant.** An enum is as large as its largest variant. Boxing that variant
shrinks every value, including the small ones.

```rust
type Payload = [u8; 100];

enum Inline { Ping, Seq(i32), Body(i32, Payload) }
enum Boxed { Ping, Seq(i32), Body(Box<(i32, Payload)>) }

const _: () = assert!(std::mem::size_of::<Inline>() == 108);
const _: () = assert!(std::mem::size_of::<Boxed>() == 16);
```

The trade is one heap allocation whenever the boxed variant is built. It wins when that
variant is rare. Boxing a hot variant trades a size win for an allocation on the common
path and loses. Clippy's `large_enum_variant` suggests this fix, but only when the largest
and second-largest variants differ by more than 200 bytes. It never looks at the total.

**Drop the words you do not use.** `Vec<T>` is 3 words, `Box<[T]>` is 2 once the length is
final, and `ThinVec<T>` (`thin-vec` crate) is 1 for a vector that is often empty in a hot type.

**Keep the default repr.** `#[repr(C)]` turns off field reordering: `{ a: u8, b: u64, c: u8 }`
is 16 bytes by default and 24 under `#[repr(C)]`. Apply it only where a layout contract needs
it: an FFI type (the `rust-unsafe` skill holds the contract) or a byte-view derive such as
`ByteHash`.

Read [references/type-size-reduction.md](references/type-size-reduction.md) when you read a
`-Zprint-type-sizes` block, freeze a `Vec` into `Box<[T]>` (a `collect` into it is not a
cheaper path), merge wrapped fields, narrow indices, probe the copy boundary on your target, or
tune the clippy size thresholds.

## Bounds checks

An index expression is checked unless the compiler can prove the index is in range. The
check is cheap, and the branch it adds is what blocks vectorization.

Three safe shapes remove it, in order of preference:

1. Iterate. `v.iter().copied().sum()` has no index to check.
2. Reslice first: `let s = &v[..n];`, then index `s` with `0..n`. The length and the loop
   bound are now the same value.
3. `assert!(n <= v.len())` once, ahead of the loop.

Measured on 1.97.0 aarch64 at `-O`, all three removed the check, and the naive
`for i in 0..n { t += v[i]; }` kept it. Confirm each change with the assembly probe in
[Verify and pin the win](#verify-and-pin-the-win).

For a constant chunk size, split with `as_chunks::<N>()` (Rust 1.88): each full chunk is a
`[T; N]`, so constant indexes into it need no check. Clippy 1.98 warns on `chunks_exact(N)`
with a constant `N` by default (`chunks_exact_to_as_chunks`) when the crate's `rust-version` is
1.88 or later, or unset, so a `-D warnings` gate fails on the older form.

Vectorization also needs the right optimization level. rustc runs the loop vectorizer only at
`opt-level` 2 and 3, and the SLP vectorizer only at 3, so under `"s"` or `"z"` no rewrite of the
scalar loop turns the vectorizer back on. Raise the hot crate alone with
`[profile.release.package.<name>] opt-level = 3`, and also the crate that instantiates its
generic code. If the crate must stay at `"s"` or `"z"`, write the loop with `core::arch`
intrinsics: they do not depend on the vectorizer, and the `rust-unsafe` skill covers the calls
that need `unsafe`.

Reach for `get_unchecked` only when all these shapes, `as_chunks` included, fail and a
benchmark justifies it. It is `unsafe` and it needs a SAFETY comment that proves the bound; the
`rust-unsafe` skill holds the rules. Clippy's `missing_asserts_for_indexing` finds the sites
mechanically, from the `restriction` group.

Read [references/inlining-and-codegen.md](references/inlining-and-codegen.md) when the MSRV is
below 1.88 or the chunk size is known only at run time, or when you need the probe functions,
the vector-instruction counts, or a `core::arch` example.

## Inlining

Four forms, and they are not a strength dial:

| Form | Meaning | Reach for it |
| --- | --- | --- |
| none | The compiler decides | Almost always |
| `#[inline]` | Raises willingness, and permits cross-crate inlining | A large function a profile shows called across a crate boundary |
| `#[inline(always)]` | Effectively a command | One hot call site, after a measurement |
| `#[inline(never)]` | Suppresses it | The cold half of a hot and cold split |

**Do not sprinkle `#[inline]` on small public helpers.** rustc already ships some bodies to
downstream crates with no attribute and no LTO: a non-generic function whose optimized MIR
holds no call (intrinsics excepted), no drop of a non-trivial value, no unwind cleanup, and at
most 100 MIR statements. One remaining call disqualifies it at any size. rustc skips this
inference in incremental builds and at `opt-level = 0`, and the MIR inliner that removes small
calls runs only at `opt-level` 2 and 3. Add the attribute when the profile names the function,
because every downstream crate compiles the body again.

**An inline attribute ships one body, not its callees.** Across a crate boundary, `#[inline]`
on `f` ships only `f`. A call to `g` survives inside the inlined `f` unless `g` also qualifies,
carries its own attribute, or the build uses LTO. That is the usual reason an
`#[inline(always)]` on a dependency function measures as no change.

**Mark the rare path `#[cold]`, and pair it with `#[inline(never)]`.** `#[cold]` lowers to
the LLVM `cold` function attribute, which biases branch layout and register allocation away
from that edge. It does not imply `noinline`: measured on 1.97.0, a small `#[cold]` callee
was inlined into its caller whole, and no `cold` attribute survived in the IR at all. The
second attribute is what gets the body out of the hot function.

```rust
#[cold]
#[inline(never)]
fn report_corrupt(offset: usize) -> std::io::Error {
    std::io::Error::other(format!("corrupt record at {offset}"))
}
```

For a rare branch that is not its own function, call `core::hint::cold_path()` as the first
statement of that branch (Rust 1.95). On 1.98.1 it puts `branch_weights` on the branch in the
IR. The code stays inline.

Read [references/inlining-and-codegen.md](references/inlining-and-codegen.md) when you split a
hot call site from cold ones, outline a rare path, count a function's MIR statements, price the
compile time of `#[inline]`, or read the IR to see what was inlined.

## Related skills

Load these by name when they are installed.

| Skill | For |
| --- | --- |
| `rust-lints` | These rules as lints, and the `clippy.toml` thresholds |
| `rust-discipline` | Signature shape: `&str` and `&[T]` against `&String` and `&Vec<T>`, SemVer hazards |
| `rust-security` | Why a hasher swap is a security decision |
