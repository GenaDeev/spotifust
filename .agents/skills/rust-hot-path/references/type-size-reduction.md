# Type size reduction

Deep material for the "Type size" section of [SKILL.md](../SKILL.md): how to read a
`-Zprint-type-sizes` block, what a wrapper costs per field, the exact clippy boundaries, and how
to guard a size once you measured it.

Sections: read a `-Zprint-type-sizes` block; merge fields under one wrapper; shrink the index
type; freeze a `Vec` into `Box<[T]>`, and the cost of the conversion; probe the copy boundary;
the clippy gates; guard the size; order of attack.

All figures below were measured on rustc 1.97.0. Layout is an unspecified implementation detail.
Re-measure on your toolchain before you depend on a number.

## Read one `-Zprint-type-sizes` block

Within one compilation unit the blocks print in descending total size. A `cargo build` is many
such units. Every dependency and every build script emits its own sorted run, so the concatenated
stream is not globally sorted, and its first block is not the largest type in the build. Grep the
stream for your own type names, or re-sort it with `top-type-sizes`. Measured on a small crate
whose only dependency is `serde_json`: 1618 blocks, 8 descending runs, the largest type of the
build (17664 bytes) at block 444, and the crate's own largest type at block 1613.

This is the block for `enum Node { Leaf, Pair(u32, u32), Big([u8; 64]) }`:

```text
print-type-size type: `Node`: 68 bytes, alignment: 4 bytes
print-type-size     discriminant: 1 bytes
print-type-size     variant `Big`: 64 bytes
print-type-size         field `.0`: 64 bytes
print-type-size     variant `Pair`: 11 bytes
print-type-size         padding: 3 bytes
print-type-size         field `.0`: 4 bytes, alignment: 4 bytes
print-type-size         field `.1`: 4 bytes
print-type-size     variant `Leaf`: 0 bytes
print-type-size     end padding: 3 bytes
```

Read it line by line:

- The header gives the total and the alignment. 68 bytes is what every `Node` value costs.
- `discriminant: 1 bytes` is the tag. It sits at offset 0 and it is charged once.
- Variants print largest first. `Big` decides the total; `Leaf` and `Pair` pay for it.
- `padding: 3 bytes` inside `Pair` is the gap between the 1-byte tag and a field of alignment 4.
  The `alignment: 4 bytes` note on the next line is the reason it exists.
- `end padding: 3 bytes` rounds 65 bytes (64 plus the tag) up to a multiple of the alignment.

Fields print in **layout** order, not declaration order. There is no offset column, so field
reordering is visible only as the listing order. `struct Header { id: u64, kind: u8, flags: u32,
name: Box<str> }` prints as:

```text
print-type-size type: `Header`: 32 bytes, alignment: 8 bytes
print-type-size     field `.name`: 16 bytes
print-type-size     field `.id`: 8 bytes
print-type-size     field `.flags`: 4 bytes
print-type-size     field `.kind`: 1 bytes
print-type-size     end padding: 3 bytes
```

rustc moved `name` to the front and `kind` to the back. No interior `padding` line appears, so
editing the declaration wins nothing. That is the normal result under the default repr.

### What the dump covers, and what it omits

Produce the dump from a cold build. A warm cache emits nothing for the units rustc skips. Use
this form to survey the whole crate graph; the `cargo rustc` form in SKILL.md scopes the dump to
one crate and keeps the build cache.

```bash
cargo clean
RUSTFLAGS=-Zprint-type-sizes cargo +nightly build -j1 > type-sizes.txt 2>&1
```

`-j1` keeps the per-crate runs from interleaving. Measured on a one-type crate: the first build
prints its block, a build after an edit prints it again, and a build with no edit prints nothing.

- It covers every type laid out during codegen. Unrelated std and dependency types appear in the
  same stream, so grep for your own type names.
- An uninstantiated generic never appears. `Gen<T>` is absent; each monomorphization such as
  `Gen<u32>` is present. A type missing from the dump is usually a generic that this crate never
  instantiates. Measure it from a crate that does.
- On stable the flag is rejected. rustc 1.98.1 prints
  ``error: the option `Z` is only accepted on the nightly compiler``.
- `top-type-sizes` 0.2.1, released 2025-12-26, reformats the same output. Install it with
  `cargo install --locked top-type-sizes` and pipe the dump through it. Its value is sorting, compaction
  and filtering. It re-sorts the whole stream by size, which repairs the per-unit order of a
  multi-crate build, and it offers `--remove-wrappers`, `--hide-less` and `--limit`. `-r` flips
  the print direction.

### Triage the block

| Line in the dump | Cause | Fix |
| --- | --- | --- |
| `end padding: N` | The size rounds up to the type's alignment | Remove the most-aligned field, or accept the tail |
| `padding: N` between two fields | The tag or a preceding field leaves the next field misaligned | Under the default repr, nothing to do. Under `#[repr(C)]`, declare fields in descending alignment, within the limit below |
| One variant much larger than the rest | An enum is as large as its largest variant | Box that variant |
| Two variants both large | Boxing one wins nothing, and clippy stays silent | Box both, or split the enum into two types |
| The type is absent | An uninstantiated generic | Measure in the crate that instantiates it |
| A large `std` type you never named | Codegen lays out dependency types too | Ignore it, unless you own the call that instantiates it |

### The limit of a `#[repr(C)]` reorder

Descending alignment removes interior padding. It never removes tail padding, because the size
still rounds up to the alignment. The reorder therefore shrinks the type only when the interior
gap is larger than the tail gap it creates. Measured on aarch64-apple-darwin:

```rust
pub struct Native { pub a: u8, pub b: u64, pub c: u8 }             // rustc reorders: 16 bytes
#[repr(C)] pub struct Spread { pub a: u8, pub b: u64, pub c: u8 }  // 7 interior + 7 tail
#[repr(C)] pub struct Packed { pub b: u64, pub a: u8, pub c: u8 }  // 0 interior + 6 tail
#[repr(C)] pub struct Small  { pub a: u8, pub b: u32 }             // 3 interior
#[repr(C)] pub struct Wide   { pub b: u32, pub a: u8 }             // the same 3, now at the tail
const _: () = assert!(std::mem::size_of::<Native>() == 16);
const _: () = assert!(std::mem::size_of::<Spread>() == 24 && std::mem::size_of::<Packed>() == 16);
const _: () = assert!(std::mem::size_of::<Small>() == 8 && std::mem::size_of::<Wide>() == 8);
```

The first pair saves 8 bytes. The second pair saves nothing. So a reorder is worth a try for the
size, and it is never a way to reach zero padding. A type that must carry no padding at all, for
`zerocopy`'s `ByteHash` or `bytemuck`'s `NoUninit`, needs an explicit tail field or a narrower
alignment. See `hashing-and-io.md` for that bound.

## Merge fields under one wrapper

A synchronization wrapper charges a fixed header per field, not per byte. Measured on
aarch64-apple-darwin:

| Type | Size |
| --- | --- |
| `std::sync::Mutex<u32>` | 16 bytes |
| `std::sync::Mutex<(u32, u32)>` | 24 bytes |
| `RefCell<u32>` | 16 bytes |
| `RefCell<(u32, u32)>` | 16 bytes |

Three consequences:

- Two `Mutex<u32>` fields cost 32 bytes here. One `Mutex<(u32, u32)>` costs 24, and it takes one
  lock acquisition per paired access instead of two. The absolute numbers are platform-specific.
  `std::sync::Mutex` carries a 16-byte header on Darwin and an 8-byte one on a Linux futex target,
  where `Mutex<u32>` is 12 bytes and `Mutex<(u32, u32)>` is 16. The saving of one header per merged
  pair holds on both.
- Two `RefCell` fields cost 32 bytes. One `RefCell<(u32, u32)>` costs 16. The merge is nearly
  free, because the borrow flag is one `usize` whatever the payload is.
- Two `Arc<Mutex<T>>` fields cost two allocations and 16 bytes inline. One `Arc<Mutex<(T, T)>>`
  costs one allocation and 8 bytes inline, and it removes a lock-ordering deadlock class
  outright: there is no longer a pair to order.

```rust
use std::sync::{Arc, Mutex};

pub struct Counters {
    hits: Arc<Mutex<u64>>,
    misses: Arc<Mutex<u64>>,
}
```

```rust
use std::sync::{Arc, Mutex};

pub struct Counters {
    // One allocation, one lock, no ordering rule between the two counts.
    counts: Arc<Mutex<(u64, u64)>>,
}
```

The counter-case decides it. A merge serializes threads that previously touched the two fields
independently. Merge when the fields are read or written together. Keep them apart when different
threads contend for them.

## Shrink the index type

Indices are `usize` by habit, and `usize` is 8 bytes on a 64-bit target. `u32` halves an
index-heavy struct. Measured: `(u32, u32, u32)` is 12 bytes, `(usize, usize, usize)` is 24.

Store the narrow type and widen at the use site.

```rust
pub struct Edge {
    from: u32,
    to: u32,
    weight: u32,
}

pub fn endpoint(nodes: &[u64], edge: &Edge) -> u64 {
    nodes[edge.from as usize]
}
```

`as usize` is a widening cast on a 64-bit target. The cost is a register move, and it is paid once
per access against 4 bytes saved per stored index. The ceiling is `u32::MAX` elements. Narrow on
store with `u32::try_from(index)` and handle the error, or `.expect("<collection> index fits u32:
<reason>")` for a real invariant. `index as u32` truncates silently and corrupts the edge. Write the
ceiling in a comment next to the field.

## Freeze a finished `Vec` into `Box<[T]>`

SKILL.md *Type size* gives the word counts of `Vec<T>`, `Box<[T]>`, and `ThinVec<T>`.
`collect::<Box<[T]>>()` is `collect::<Vec<_>>().into_boxed_slice()` in std, so it is not a
cheaper path. It allocates once only when std knows the exact length up front: a range, a
slice, a `Vec`, and `map` or `copied` over one. After `filter` or another inexact adaptor it runs
the growth ladder and then one shrinking `realloc`. Measured on 1.98.1, 500 filtered `u32` cost 1
allocation and 8 reallocations in both forms. When you know the final length, write
`Vec::with_capacity(n)`, `extend`, then `into_boxed_slice()`: capacity equals length, so the
conversion does not reallocate.

### The conversion is not always free

`Vec::into_boxed_slice` calls `shrink_to_fit` when capacity exceeds length. That issues a
`realloc`, and a `realloc` does not always move the buffer. Measured with a `Vec<u32>` at length 3
on aarch64-apple-darwin, against the system malloc:

| Capacity before the call | Data pointer |
| --- | --- |
| 3 | unchanged |
| 4 | unchanged |
| 8 | moved |
| 1000 | moved |

The result is allocator-specific, not only target-specific. The same probe under glibc reports an
unchanged pointer for every row above, on aarch64 and on x86_64 alike, because glibc trims the
chunk in place. State the cost as "may cost a full copy when capacity is meaningfully above
length", never as "always copies", and run the probe against your own allocator before you budget
for it.

The reverse direction has no such caveat. `<[T]>::into_vec` never reallocates: the pointer is
preserved and the capacity equals the length in every row above.

## Probe the copy boundary on your own target

The boundary is target-dependent and opt-level-dependent, so measure it rather than quote it:

```bash
cat > probe.rs <<'RS'
pub type Buf = [u8; 256];
#[inline(never)]
pub fn copy(src: &Buf) -> Buf { *src }
RS
rustc -O --edition 2024 --emit asm --crate-type=lib probe.rs -o probe.s
grep -c memcpy probe.s        # 0 = copied inline, 1 or more = a call to memcpy
```

Measured with this recipe on aarch64-apple-darwin: 256 prints 0 and 257 prints 1. Repeat the run
with `-C opt-level=0` and the boundary drops to 32 and 33. A debug build therefore tells you
nothing about the shipped one.

Raise the array size until `grep` prints 1. That size minus one is the largest type your target
moves without a library call.

### A zero `memcpy` count is not a zero copy count

Do not read `grep -c memcpy` of 0 as "this value-passing signature is free". Below the boundary
rustc lowers the move into inline `ldp`/`stp` pairs. The call leaves the assembly. The byte
traffic stays. Count instructions per function as well:

```bash
rustc --edition 2024 -C opt-level=3 --emit=asm --crate-type=lib probe.rs -o probe.s
awk '/^_[A-Za-z0-9_$.]*:$/{f=$0} /^[ \t]+[a-z]/{n[f]++} END{for(k in n) print n[k], k}' probe.s
```

Measured on aarch64-apple-darwin at `-C opt-level=3`, on a state struct of `[u8; N]` plus a
function pointer plus a `NonNull`, driven by a loop:

| State size | `fn step(s: &mut S)` loop | `s = step(s)` loop |
| --- | --- | --- |
| 216 bytes | 0 memcpy, 15 instructions | 0 memcpy, 51 instructions |
| 256 bytes | 0 memcpy, 15 instructions | 0 memcpy, 51 instructions |
| 1040 bytes | 0 memcpy, 15 instructions | 2 memcpy, 28 instructions |

At 256 bytes both forms report zero `memcpy` calls, and the by-value loop still runs 3.4x the
instructions. The `memcpy` call is the ceiling of the copy cost, not a switch that turns it on.
Use the `grep` only to find the boundary. Use the instruction count to decide a signature.

## The clippy gates and where they stop

| Lint | Default | Fires when | `clippy.toml` key |
| --- | --- | --- | --- |
| `large_enum_variant` | warn | Largest variant minus second-largest is strictly greater than 200 bytes | `enum-variant-size-threshold` |
| `result_large_err` | warn | The `Err` variant reaches 128 bytes | `large-error-threshold`, `large-error-ignored` |

Both boundaries measured exactly on 1.97.0:

- `enum D { A(u8), B([u8; 201]) }` is silent. `[u8; 202]` warns.
- `struct E([u8; 127])` used as `Result<(), E>` is silent. `[u8; 128]` warns.

`large_enum_variant` looks only at the difference between the two largest variants. It never looks
at the total, so this 301-byte enum passes a default `cargo clippy` with no warning:

```rust
pub enum Frame {
    Header([u8; 300]),
    Body([u8; 300]),
}
const _: () = assert!(std::mem::size_of::<Frame>() == 301);
```

Boxing one variant here wins nothing. Box both, or split `Frame` into two types and let the caller
hold the one it needs.

Lower both thresholds when the enum or the error travels through a hot path:

```toml
enum-variant-size-threshold = 64
large-error-threshold = 64
large-error-ignored = ["my_crate::LegacyError"]
```

Verified with those keys in a `clippy.toml`: an 80-byte variant and an 80-byte error both warn,
and a 200-byte error named in `large-error-ignored` stays silent while an identical unlisted type
warns. Name a type in the allowlist only when it is over the threshold. `std::io::Error` is 8
bytes on a 64-bit target, so it never fires, and an entry for it does nothing.

## Guard the size you measured

```rust
pub struct Node {
    next: Option<Box<Node>>,
    id: u32,
}

// 16 bytes measured on rustc 1.97.0, aarch64-apple-darwin.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Node>() == 16);
```

- `size_of` is in the prelude since Rust 1.80. Write `std::mem::size_of` when the MSRV is
  older. No external dependency is required.
- A mismatch fails the build with
  `error[E0080]: evaluation panicked: assertion failed: std::mem::size_of::<Node>() == 16`.
- Gate it. The same assertion without the `cfg` compiles on aarch64-apple-darwin and fails with
  E0080 when the identical file is cross-compiled to `i686-linux-android`, because the pointer is
  4 bytes there.
- Do not add the `static_assertions` crate for this. It is stuck at 1.1.0, released 2019-11-03,
  and the `const` form above needs no dependency at all.

Put the assert directly under the type, not in a test module. It must fail the build of the crate
that owns the layout, in every profile, including a release build that runs no tests.

## Order of attack

1. Dump with `-Zprint-type-sizes` from a clean build. Grep the stream for your own types, or pipe
   it through `top-type-sizes`.
2. Box the outsized variant, if the block shows one.
3. Merge fields that share a `Mutex`, a `RefCell`, or an `Arc`.
4. Narrow indices from `usize` to `u32`.
5. Freeze finished vectors to `Box<[T]>`, and check the capacity first.
6. Re-dump from a clean build, confirm the new number, then pin it with a gated `const` assert.
