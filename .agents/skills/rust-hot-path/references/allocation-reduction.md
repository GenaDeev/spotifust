# Allocation reduction

Allocation patterns that [SKILL.md](../SKILL.md) names but does not develop. SKILL.md holds the
`Vec` growth ladder, `reserve_exact`, `clone_from`, and the workhorse-buffer rule. Read those
first; this file does not repeat them.

Sections: symptom routing; `HashMap` capacity; `collect` exactness; zero-filled buffers;
inline capacity (`SmallVec`, `ArrayVec`); inline strings; measuring the length distribution;
`Rc` and `Arc`; `format!` and `format_into` in a loop; eager arguments; `shrink_to_fit`; pinning
an allocation count with `dhat`; lints.

All figures were measured on rustc 1.97.0, aarch64-apple-darwin, release profile, unless the
text names 1.98.1.

## Route a symptom to a fix

| Symptom | Cause | Fix |
| --- | --- | --- |
| A map reallocates several times under a known key count | `HashMap::new` plus inserts | [HashMap capacity](#hashmapwith_capacity-never-returns-the-number-you-ask-for) |
| `collect` lands on a power of two, not on the length | The chain lost its exact size | [collect exactness](#collect-allocates-once-only-when-the-length-is-exact) |
| A zero-filled buffer pays a separate zeroing pass | `resize(n, 0)` instead of `vec![0; n]` | [Zero-filled buffers](#zero-filled-buffers-keep-the-calloc) |
| One allocation per call for a collection that is almost always tiny | The field is `Vec` or `String` | [Inline capacity](#inline-capacity-smallvec-and-arrayvec) |
| The allocation count rose after an `Rc` or `Arc` refactor | The wrapper heap-allocates a value that was inline | [Reference counting](#rc-and-arc-are-not-an-allocation-fix) |
| One allocation per iteration of a formatting loop | `format!` returns an owned `String` | [format! in a loop](#format-in-a-loop-allocates-once-per-call) |
| An allocation happens on the path that discards the value | `ok_or`, `unwrap_or`, `map_or` | [Eager arguments](#eager-arguments-allocate-on-the-path-that-discards-them) |
| A raw pointer taken before a `Vec` call dangles after it | `shrink_to_fit` reallocated the buffer | [shrink_to_fit](#shrink_to_fit-reallocates) |

## `HashMap::with_capacity` never returns the number you ask for

hashbrown rounds the bucket count up to a power of two and keeps a 7/8 load factor. The
returned capacity is therefore a different number from the request.

| Requested | `capacity()` | Buckets |
| --- | --- | --- |
| 1 | 3 | 4 |
| 3 | 3 | 4 |
| 7 | 7 | 8 |
| 8 | 14 | 16 |
| 20 | 28 | 32 |
| 100 | 112 | 128 |
| 1000 | 1792 | 2048 |

Ask for the count you expect. Never read the returned number back and treat it as your own
figure; at a request of 1000 it overstates the reservation by 79%.

`HashMap::new` allocates nothing until the first insert, so the reserve is free to skip and
expensive to omit. Twenty inserts of `u32` keys cost 4 allocations and 572 bytes from
`HashMap::new`, and 1 allocation and 296 bytes from `HashMap::with_capacity(20)`.

```rust
use std::collections::HashMap;

let mut index: HashMap<u32, u32> = HashMap::with_capacity(20);
for key in 0..20u32 {
    index.insert(key, key * 2);
}
assert_eq!(index.capacity(), 28);   // 32 buckets at a 7/8 load factor
```

`HashSet` uses the same table and behaves the same way. A non-default hasher does not change
the rounding; see the `Lookups` section of SKILL.md for the hasher decision.

## `collect` allocates once only when the length is exact

`collect` reserves the iterator's exact size when the source reports one, and falls back to
`size_hint`'s lower bound when it does not. The fallback runs the whole growth ladder.

| Chain over 1000 items | Length | Capacity | Allocations |
| --- | --- | --- | --- |
| `(0..1000u32)` | 1000 | 1000 | 1 |
| `(0..1000u32).map(...)` | 1000 | 1000 | 1 |
| `(0..1000u32).rev()` | 1000 | 1000 | 1 |
| `slice.iter().copied()` | 1000 | 1000 | 1 |
| `(0..1000u32).filter(...)` | 500 | 512 | ladder |
| `(0..1000u32).take_while(...)` | 500 | 512 | ladder |
| `(0..10u32).flat_map(...)` | 45 | 64 | ladder |
| `"hello world".chars()` | 11 | 16 | ladder |

`filter` reports `(0, Some(1000))`. The lower bound is 0 because a predicate can reject
everything, so the reserve is 0 and the first push starts at capacity 4. `chars` reports
`(3, Some(11))` for an 11-byte string: the lower bound is `len.div_ceil(4)`, the byte length
divided by 4 and rounded up, because 4 is the maximum width of one UTF-8 code point.
`flat_map`, `take_while`, `skip_while`, `scan`, `map_while`, and every adaptor placed after one
of them lose exactness too. `scan` over a 1000-item range keeps all 1000 elements and still
lands at capacity 1024.

These adaptors keep exactness and need no fix: `map`, `rev`, `copied`, `cloned`, `enumerate`,
`skip`, `take`, `step_by`, `chain`, and `zip`.

When you know the output length, state it and use `extend`. `extend` reserves the lower bound
and then follows the ladder, so a `with_capacity` large enough for the result keeps one
allocation.

```rust
fn evens(limit: u32) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::with_capacity(limit as usize / 2);
    out.extend((0..limit).filter(|value| value % 2 == 0));
    out
}
// limit 1000: length 500, capacity 500, one allocation.
```

For a hand-written iterator, implement `size_hint` instead. SKILL.md states the rule, and the
`rust-iterator-impl` skill holds the impl.

Do not `collect` only to iterate again. Return `impl Iterator<Item = T>` instead of `Vec<T>`. On
edition 2024 this needs no lifetime bound, because RPIT captures in-scope lifetimes by default.
The lint is `needless_collect`, in clippy's `nursery` group.

## Zero-filled buffers: keep the `calloc`

`vec![0u8; n]` lowers to `__rust_alloc_zeroed`. That is one `calloc`: the zeroing moves into the
allocator, which skips it for an allocation large enough to be served by fresh kernel pages. Every
other spelling keeps the plain allocation and adds an explicit `_bzero` pass over the whole
buffer. Against `with_capacity` plus `resize` on aarch64-apple-darwin, that is 1.4x at 64 bytes
and 13x at 1 MiB. Near 4 KiB the two forms tie, because `calloc` still zeroes in user space at
that size.

| Form | Lowering at `-O` | Zeroing pass in the generated code |
| --- | --- | --- |
| `vec![0u8; n]` | `__rust_alloc_zeroed` | 0 |
| `Vec::with_capacity(n)` + `extend(repeat_n(0u8, n))` | `__rust_alloc_zeroed` | 0 |
| `Vec::new()` + `resize(n, 0)` | `do_reserve_and_handle` + `_bzero` | 1 |
| `Vec::with_capacity(n)` + `resize(n, 0)` | `__rust_alloc` + `_bzero` | 1 |

The last row is the trap. Adding `with_capacity` to a `resize` call looks like the careful
version and it still loses the `calloc`. Write `vec![0u8; n]`.

Clippy catches both `resize` forms with `slow_vector_initialization` ("slow zero-filling
initialization"). That lint warns by default, so a plain `cargo clippy` already reports it.
The rule holds only for a zero fill. `vec![0xFFu8; n]` has no `calloc` to lose.

## Inline capacity: SmallVec and ArrayVec

An inline-capacity type stores the first N elements in the struct itself. It trades bytes for
allocations, and the trade is only worth making when the collection is almost always short.

Sizes on 64-bit, smallvec 1.16.1 and arrayvec 0.7.8:

| Type | Bytes | Note |
| --- | --- | --- |
| `Vec<u32>`, `String` | 24 | The baseline |
| `SmallVec<[u8; 1]>` | 24 | Free: below the 24-byte floor |
| `SmallVec<[u8; 8]>` | 24 | Free |
| `SmallVec<[u32; 1]>` | 24 | Free |
| `SmallVec<[u32; 2]>` | 24 | Free |
| `SmallVec<[u8; 16]>` | 32 | |
| `SmallVec<[u32; 4]>` | 32 | |
| `SmallVec<[u32; 32]>` | 144 | Six times `Vec` |
| `ArrayVec<u32, 4>` | 20 | Never spills, never allocates |

smallvec 1.x holds a `usize` beside a niche-packed enum, so no one-line size rule holds. Up to 8
inline bytes stay free. Above that the width steps by 8 and stops tracking the array: `[u8; 9]`,
`[u8; 12]`, `[u8; 17]`, `[u16; 5]`, `[u32; 3]`, and `[u32; 5]` all measure 32 bytes, the same as
the two 32-byte rows above. Assert the width of the N you pick.

```rust
use smallvec::SmallVec;

const _: () = assert!(std::mem::size_of::<SmallVec<[u32; 4]>>() == 32);
```

A spilled `SmallVec` does not grow from N. It rejoins the normal `Vec` ladder.

```rust
use smallvec::SmallVec;

let mut path: SmallVec<[u32; 4]> = SmallVec::new();
path.extend([1, 2, 3, 4]);
assert!(!path.spilled());
assert_eq!(path.capacity(), 4);

path.push(5);
assert!(path.spilled());
assert_eq!(path.capacity(), 8);   // the Vec ladder, not 5
```

Pushing 20 elements into a `SmallVec<[u32; 4]>` cost 3 allocations and ended at capacity 32.
The same 20 pushes into a `Vec<u32>` cost 4 allocations and ended at capacity 32. One
allocation saved, and every operation on the type pays an inline-or-spilled branch.

**Raising N toward the 95th percentile is the direction that makes `SmallVec` lose.** A large
N grows every value, including the empty ones, and moves that cost into every copy of the
enclosing type. Pick N at the mode of the distribution, not at its tail.

`ArrayVec` has a hard bound and no heap path at all. Use it when the maximum is a real
invariant, and handle the full case explicitly.

```rust
use arrayvec::ArrayVec;

let mut fixed: ArrayVec<u32, 4> = ArrayVec::new();
assert!(fixed.try_push(1).is_ok());
fixed.extend([2, 3, 4]);
assert!(fixed.try_push(5).is_err());   // full: no spill, no allocation
```

smallvec 2.0 is a pre-release, 2.0.0-beta.1 as of 2026-09. Pin the full version: `cargo add
smallvec@2` fails with "could not be found in registry index". It changes the type form to
`SmallVec<T, N>` and packs the value into a union, so it inlines 8 more bytes at the same width.
Measured on beta.1 with rustc 1.98.1: `SmallVec<u32, 4>` is 24 bytes against 32 for
`SmallVec<[u32; 4]>` in 1.16.1, and a push past N still jumps to capacity 8. The width still
rounds by 8, so measure it there too.

## Inline strings

Three string types, all 24 bytes wide on 64-bit, so the swap costs no size in the enclosing
type:

| Type | Inlines up to | First heap length | Last release |
| --- | --- | --- | --- |
| `String` | 0 bytes | 1 | std |
| `CompactString` (compact_str 0.10.0) | 24 bytes | 25 | current |
| `SmartString` (smartstring 1.0.1) | 23 bytes | 24 | 2022 |

Prefer `compact_str`. It inlines one more byte and it is maintained. Measured allocation
counts confirm both thresholds: at length 24, `CompactString` allocates 0 times and
`SmartString` allocates 2.

```rust
let key = compact_str::CompactString::from("content-length");
assert!(!key.is_heap_allocated());
```

24 bytes covers most header names, enum tags, file extensions, and short identifiers. It does
not cover paths or user text. Measure the length distribution before you swap the type.

## Measure the distribution before you pick a number

Do not guess N or a capacity. Add one temporary `eprintln!("{}", values.len());` at the
allocation site, run the real workload, and read the shape. Remove the line afterwards.

```bash
cargo run --release -q 2>len.txt >/dev/null
sort -n len.txt | uniq -c | sort -rn | head -20
```

Read the result this way:

| Shape | Choose |
| --- | --- |
| One length dominates and it is small | `SmallVec` or `ArrayVec` with N at that length |
| A wide spread with a known upper bound | `Vec::with_capacity` at the common case |
| A wide spread with no bound | Plain `Vec`. Reserve nothing |

## `Rc` and `Arc` are not an allocation fix

`Rc::clone` and `Arc::clone` increment a counter and allocate nothing. Every other clone of a
heap-owning type allocates: cloning a three-element `Vec<u32>` through `Deref` cost 1
allocation.

The reverse is the mistake worth naming. Wrapping a value in `Rc` or `Arc` moves it to the
heap. A value that is rarely shared therefore gains an allocation it did not have. Reach for
reference counting when the sharing is real, not to make a clone cheaper.

`Arc::make_mut` and `Rc::make_mut` give clone-on-write. Both take a sized `T: Clone`, and since
Rust 1.81 also unsized values such as `Arc<str>` and `Arc<[T]>`. The real bound is
`T: ?Sized + CloneToUninit`, an unstable trait that std implements for those types.

| Handles outstanding | `make_mut` behaviour | `T::clone` calls |
| --- | --- | --- |
| Sole strong, no weak | Mutates in place | 0 |
| Two strong | Clones the value | 1 |
| Sole strong, one or more weak | Moves the value to a new allocation | 0 |

```rust
use std::sync::Arc;

let mut shared: Arc<Vec<u32>> = Arc::new(vec![1, 2, 3]);
Arc::make_mut(&mut shared).push(4);       // sole owner: in place
let second = Arc::clone(&shared);
Arc::make_mut(&mut shared).push(5);       // second strong handle: clones
assert_eq!(second.len(), 4);
assert_eq!(shared.len(), 5);

let mut name: Arc<str> = Arc::from("id");
Arc::make_mut(&mut name).make_ascii_uppercase();   // unsized, since 1.81
```

The third row is a trap. `make_mut` skips the clone, and it silently disassociates every
`Weak`. Any observer that held one starts getting `None` from `upgrade`.

```rust
use std::sync::{Arc, Weak};

let mut owner: Arc<Vec<u32>> = Arc::new(vec![1, 2, 3]);
let observer: Weak<Vec<u32>> = Arc::downgrade(&owner);
assert!(observer.upgrade().is_some());

Arc::make_mut(&mut owner).push(4);
assert!(observer.upgrade().is_none());    // every Weak is dead now
```

Do not combine `make_mut` with a `Weak`-based cache or observer list.

## `format!` in a loop allocates once per call

`format!` builds a fresh `String` every call. `write!` appends into a buffer you own. Counted
with a counting global allocator on 1.97.0, 1000 iterations: 1000 allocations through
`format!`, 1 through the loop below.

```rust
use std::fmt::Write as _;

fn labels(ids: &[u32]) -> Result<usize, std::fmt::Error> {
    let mut buf = String::with_capacity(32);
    let mut written = 0;
    for id in ids {
        buf.clear();
        write!(buf, "item-{id}")?;
        written += buf.len();
    }
    Ok(written)
}
```

Two traps sit on this path. `write!` on a `String` needs `std::fmt::Write` in scope, and
without the import it fails with E0599, which is where people give up and restore `format!`.
And `write!` on a `String` returns `std::fmt::Result`, not `std::io::Result`, so the function
signature changes with the fix.

To hand a formatted value to a callee that takes `fmt::Arguments`, use `format_args!`. It
borrows its arguments and allocates nothing.

For one integer, skip `fmt` entirely. `format_into` (Rust 1.98) writes the digits into a stack
`NumBuffer` and returns a `&str` that borrows it: no allocation, no `write!`, no `itoa`
dependency. On 1.98 the type exists only at `core::fmt::NumBuffer`; `use std::fmt::NumBuffer`
fails with E0432. The buffer is generic over the integer type, so declare one per type: passing
a `NumBuffer<u32>` to `i64::format_into` fails with E0308 (1.98.1).

```rust,run
use core::fmt::NumBuffer;

fn main() {
    let mut buf = NumBuffer::new();
    let mut total = 0;
    for id in [7u32, 42, 1000] {
        total += id.format_into(&mut buf).len(); // a &str inside buf
    }
    assert_eq!(total, 1 + 2 + 4);
    assert_eq!((-15i64).format_into(&mut NumBuffer::new()), "-15");
}
```

The `&str` borrows the buffer, so copy it out before the next call reuses `buf`.

## Eager arguments allocate on the path that discards them

`ok_or`, `unwrap_or`, `map_or`, and `Result::or` take an argument that is already evaluated.
The computation therefore runs on every call, including the call that throws the result away.
Measured: `ok_or(expensive())` on a `Some` value cost 1 allocation, `ok_or_else(expensive)`
cost 0.

| Eager | Lazy |
| --- | --- |
| `opt.ok_or(e)` | `opt.ok_or_else(\|\| e)` |
| `opt.unwrap_or(v)` | `opt.unwrap_or_else(\|\| v)` |
| `opt.map_or(v, f)` | `opt.map_or_else(\|\| v, f)` |
| `res.or(other)` | `res.or_else(\|_\| other)` |

```rust
fn slow_default() -> String {
    String::from("computed default")
}

fn eager(value: Option<u32>) -> Result<u32, String> {
    value.ok_or(slow_default())      // allocates even when value is Some
}

fn lazy(value: Option<u32>) -> Result<u32, String> {
    value.ok_or_else(slow_default)   // allocates only when value is None
}
```

Convert only the calls whose argument does work. Wrapping a plain literal in a closure is the
over-correction, and clippy's `unnecessary_lazy_evaluations` reports it by default.

## `shrink_to_fit` reallocates

`shrink_to_fit` always reallocates when the capacity is above the length. Whether the buffer moves
is the allocator's choice: on aarch64-apple-darwin a `Vec<u32>` at length 3 keeps its pointer when
it shrinks from capacity 4, and moves when it shrinks from 8, 100, or 1000. Assume it moved.
`type-size-reduction.md` holds the same table for `into_boxed_slice`, which calls this method.

```rust
let mut values: Vec<u32> = Vec::with_capacity(100);
values.extend([1, 2, 3]);
let before = values.as_ptr();
values.shrink_to_fit();
assert_eq!(values.capacity(), 3);
// `before` is dead now: the realloc is free to move the data, and at this size it does.
let _ = before;
```

Two consequences. It makes any raw pointer held across the call unusable, which matters at an FFI
boundary. Take `as_ptr()` again after any call that can reallocate (`push`, `reserve`,
`shrink_to_fit`, `into_boxed_slice`). A pointer taken before that call dangles, even when the
address comes back unchanged. It also turns a zero-allocation reuse loop
into one allocation per iteration, which SKILL.md states under the workhorse buffer. Call it
once, when a long-lived structure reaches its final size, and never inside a loop.

## Pin an allocation count with `dhat`

The `dhat` crate runs on stable. SKILL.md *Verify and pin the win* states the placement rule:
one heap test per integration test file, and no `#[global_allocator]` in `src/`. One test per
file is needed because dhat panics when two profilers run at once, and libtest runs tests in
parallel. A file with one test runs it alone, and no harness thread adds to the count.

```rust
// tests/parse_allocs.rs: one heap test per file.
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn parse_allocates_once() {
    let _profiler = dhat::Profiler::builder().testing().build();
    let parsed = my_crate::parse("key=value");
    let stats = dhat::HeapStats::get();
    dhat::assert_eq!(stats.total_blocks, 1);
    std::hint::black_box(parsed);
}
```

If several heap tests must share one file, run that target with `-- --test-threads=1`.
`dhat::assert_eq!` panics outside testing mode, so reach it only under a testing profiler.

## Lints that mechanize this file

Groups verified on clippy 0.1.98.

| Lint | Group | On by default | Catches |
| --- | --- | --- | --- |
| `slow_vector_initialization` | `perf` | yes | `resize(n, 0)` that loses the `calloc` |
| `unnecessary_lazy_evaluations` | `style` | yes | A closure around a value that costs nothing |
| `assigning_clones` | `pedantic` | no | `a = b.clone()` where `a.clone_from(&b)` reuses the buffer |
| `redundant_clone` | `nursery` | no | A clone whose original is never used again |
| `or_fun_call` | `nursery` | no | `ok_or(expensive())` and the other eager forms |
| `needless_collect` | `nursery` | no | A `collect` that is only iterated again |

Only the first two run under a plain `cargo clippy`. Enable the other four per lint rather than
by group; `pedantic` and `nursery` bring hundreds of unrelated lints with them. The `rust-lints`
skill holds the workspace `[lints.clippy]` table.
