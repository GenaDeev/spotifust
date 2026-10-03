# Hashing and I/O

Reference for [SKILL.md](../SKILL.md). It holds the full hasher measurements, the HashDoS
mechanism, the byte-wise hash derives, the buffered I/O numbers, and the line-reading loop
behind the Lookups, Allocation rate, and I/O sections.

Sections: 1-7 hashers (cost, `ahash` on aarch64, the seeding check, the collision failure
mode, crate choice, workspace enforcement, rustc's own history); 8 byte-wise hashing; 9-11
buffered output and capacity; 12-13 line reading; 14 triage.

## 1. What each hasher costs

1M insert plus lookup, several invocations, measured on rustc 1.97.0, aarch64-apple-darwin:

| Hasher | `u64` keys | 18-byte string keys | Seeded per process |
| --- | --- | --- | --- |
| std SipHash 1-3 | 69.5-73.4 ms | 143-149 ms | Yes |
| `FxHasher` (`rustc-hash` 2.1.3) | 14.8-14.9 ms | 61.8-64.6 ms | No |
| `ahash` 0.8.12 | 24.9-25.9 ms | 75.3-79.9 ms | Yes |
| `fnv` | 31.7-32.2 ms | 117-123 ms | No |

- `FxHasher` is 4.7-5.0x faster than SipHash on `u64` keys, and 2.2-2.4x faster on 18-byte
  string keys. The win shrinks as the key grows, because a longer key spends more time in
  the compression loop of every hasher.
- `fnv` costs about 2.1x `FxHasher` on integers. On 18-byte strings it beats SipHash by only
  1.2x, 117-123 ms against 143-149 ms. `fnv` gives up the per-process seed and returns no
  margin that pays for the loss. Do not pick it.
- `ahash` is the only fast hasher here with a per-process seed. It costs about 1.7x
  `FxHasher` on integers and about 1.2x on strings. Pay that when the keys are untrusted.

These are microbenchmark figures. They bound the win. Section 7 is the end-to-end anchor.

## 2. `ahash` does not use AES on stable aarch64

`ahash` 0.8.12 gates its AES path on the `nightly-arm-aes` feature. That feature is not
default and it needs a nightly compiler. A stable aarch64 build therefore runs the
folded-multiply fallback, which is what the table in section 1 measured. Do not quote
AES-accelerated throughput for a stable ARM target.

`ahash` 0.8.12 enables `runtime-rng` and `getrandom` as default features, so
`ahash::RandomState` is seeded per process out of the box. Turning default features off for
a `no_std` build removes the seed, and with it the reason to choose `ahash` over `FxHasher`.

## 3. Verify the seeding claim yourself

Run one binary twice. A hasher that prints the same number in both runs has no per-process
seed.

```rust
use std::hash::{Hash, Hasher};
use rustc_hash::FxHasher;

fn main() {
    let mut h = FxHasher::default();
    42u64.hash(&mut h);
    println!("{}", h.finish());
}
```

Measured on rustc 1.97.0 across two separate processes:

| Hasher | Output for `42u64` |
| --- | --- |
| `FxHasher` | 12569757018929961129 in both runs |
| `FnvHasher` | Byte-identical in both runs |
| `ahash` | Different each run |
| std `RandomState` | Different each run |
| `nohash` | The key itself. It is the identity function |

std documents SipHash 1-3 plus its per-process random seed as the HashDoS defence. A swap
to `FxHasher` or `fnv` deletes that defence. A swap to `ahash` keeps it.

## 4. The failure mode is a long probe sequence, not a bucket chain

std's `HashMap` is hashbrown. It is open-addressed with SIMD group probing. It holds no
per-bucket chain, so collision flooding does not build an O(n) list. It builds long probe
sequences, and every insert and lookup walks them.

Measured on 1.97.0 aarch64: keys crafted only from the growth policy, multiples of 2^20, do
not hurt `rustc-hash` 2.1.3 at all. 50,000 inserts took 0.29 ms for `0..n` and 0.22 ms for
`n * 2^20`. At 1M keys the pair was 9.2 ms and 7.5 ms. `rustc-hash` 2.x rotates the hash in
`finish`, so the bits a naive key set zeroes never reach the bucket index. The stale `fxhash`
0.2.1 crate has no such finalizer, and it does collapse: the same 50,000 inserts go from
0.20 ms to 228 ms, a 1100x loss.

Cheap key patterns therefore no longer break the current crate. An attacker who knows the
`FxHasher` constant can still build a deliberate collision set, and that set has no cheap
defence. Gate the hasher on where the keys come from, never on how hot the map is. A map that
is hot and fed from the network keeps a seeded hasher.

## 5. Pick the crate and the key type carefully

| Goal | Crate | Note |
| --- | --- | --- |
| Fastest, unseeded | `rustc-hash` 2.1.3 | Types `FxHashMap`, `FxHashSet`, `FxBuildHasher`, `FxHasher` |
| Fastest, unseeded | not `fxhash` 0.2.1 | The name still resolves. The crate is stale |
| Fast and seeded | `ahash` 0.8.12 | Keep default features. See section 2 |
| Identity hash | `nohash-hasher` 0.2.0 | Types `IntMap<K, V>` and `IntSet<K>`. There is no `NoHashMap` |

`FxHashMap::new` and `FxHashMap::with_capacity` fail with E0599 (SKILL.md *Lookups*). These two
constructors build:

```rust
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::collections::HashMap;

let mut empty: FxHashMap<u32, u32> = FxHashMap::default();
let mut sized: HashMap<u32, u32, FxBuildHasher> =
    HashMap::with_capacity_and_hasher(64, FxBuildHasher);
empty.insert(1, 1);
sized.insert(2, 2);
```

`nohash-hasher` needs a marker impl on any newtype key. Without it the map has no `insert`
method at all:

```rust
use nohash_hasher::{IntMap, IsEnabled};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct NodeId(u32);

impl IsEnabled for NodeId {}          // remove this line and insert disappears

fn main() {
    let mut ranks: IntMap<NodeId, u32> = IntMap::default();
    ranks.insert(NodeId(1), 7);
}
```

The error names the missing bound:

```text
error[E0599]: the method `insert` exists for struct `HashMap<NodeId, u32,
BuildHasherDefault<NoHashHasher<NodeId>>>`, but its trait bounds were not satisfied
    = note: the following trait bounds were not satisfied:
            `NodeId: IsEnabled`
note: the trait `IsEnabled` must be implemented
```

Identity hashing wants keys that are dense in the low bits, not keys that are spread. A plain
counter is the best case: 1M sequential `u64` keys took 5.3 ms in `IntMap` against 15.9 ms in
`FxHashMap`. 1M random ids took 21.3 ms against 20.7 ms, which is no win at all. The failure
case is a strided key space whose low bits are constant: 1M keys at `i * 4096` took 3941 ms
in `IntMap` against 17.0 ms in `FxHashMap`, a 232x loss. Use `IntMap` for counters and dense
ids. Do not use it for pointers, page-aligned handles, or any id whose low bits are fixed.

## 6. Enforce one hasher across the workspace

```toml
# clippy.toml
disallowed-types = [
  { path = "std::collections::HashMap", reason = "use rustc_hash::FxHashMap" },
]
```

Verified on 1.97.0: one `std::collections::HashMap` use produced three
`clippy::disallowed_types` warnings, at the `use`, at the type annotation, and at the
`::new` call. The `FxHashMap` in the same file produced none. Clippy matches the written
path, not the resolved type, and `FxHashMap` is an alias of that same `HashMap`. That
mismatch is what makes the rule usable: it bans the default hasher and leaves the alias
alone. `disallowed_types` warns as soon as `clippy.toml` names a path, with no lint group
and no `#![warn]` attribute.

## 7. What rustc's own hasher history says

rustc changed its hasher twice. These figures are rustc-perf instruction counts and cycles,
not wall clock. Read them as the end-to-end anchor for section 1: a hasher swap moves a
whole compiler by single-digit percent, not by the 5x the microbenchmark shows.

| Change | Result | Source |
| --- | --- | --- |
| fnv -> fxhash | Up to 6% gain | rust-lang/rust#37229 |
| fxhash -> ahash | 1-2% more instructions, 1-4% more cycles | rust-lang/rust#69153, comment 589504301 |
| fxhash -> the default std hasher | Every benchmark regressed, by 3.9% to 84.7% | rust-lang/rust#69153, comment 589338446 |

Both comments sit on issue 69153 at `github.com/rust-lang/rust/issues/69153`. Add the suffix
`#issuecomment-589504301` or `#issuecomment-589338446` to reach one directly.

nnethercote states that the ahash trial got the fallback path, not AES, so that row is not
evidence about AES-accelerated `ahash`. The last row is what a symbol-heavy workload loses
by leaving the default hasher in place.

## 8. Hash a struct as bytes, and let the compiler reject the padding

Hashing field by field calls the hasher once per field. Hashing the struct as one byte slice
calls it once. The hand-rolled version transmutes the struct to `&[u8]`, so it silently
hashes uninitialized padding, and two equal values then hash differently. Two crates turn
that into a compile error. Both derives sit behind a non-default `derive` feature.

| Crate | Derive | Also requires | Padding gives |
| --- | --- | --- | --- |
| zerocopy 0.8 | `#[derive(ByteHash)]` | `Immutable`, `IntoBytes` | E0277 |
| bytemuck 1.25.2 | `#[derive(ByteHash)]` | `NoUninit`, which implies `Copy` | E0080 |

The two errors, verbatim:

```text
error[E0277]: `P` has 3 total byte(s) of padding
              ...types with padding cannot implement IntoBytes
error[E0080]: evaluation panicked: derive(NoUninit) was applied to a type with padding
```

Reordering the fields largest-first does not fix it. Under `#[repr(C)]` the same 3 bytes
move from the middle to the tail, because the struct still rounds up to its alignment:

```rust
#[repr(C)]
struct SmallFirst { a: u8, b: u32 }      // 3 bytes of padding after `a`

#[repr(C)]
struct LargeFirst { b: u32, a: u8 }      // the same 3 bytes, now at the tail

#[repr(C, packed)]
struct NoPadding { a: u8, b: u32 }

const _: () = assert!(std::mem::size_of::<SmallFirst>() == 8);
const _: () = assert!(std::mem::size_of::<LargeFirst>() == 8);
const _: () = assert!(std::mem::size_of::<NoPadding>() == 5);
```

Only two shapes build: `#[repr(C, packed)]`, or an explicit filler field that the
constructor initializes. Packing costs unaligned field access, so measure it first. Treat
the rejection as the feature. It is the reason to use the derive instead of the transmute.

## 9. Buffering is the whole I/O win

300,000 `writeln!` to a `std::fs::File` on a local APFS volume, measured on 1.97.0 aarch64:

| Writer | Time |
| --- | --- |
| Bare `File` | 1.08-1.15 s |
| `BufWriter<File>` | 5.9-6.5 ms |

That is 170-190x. The absolute numbers move with the filesystem. The ratio does not. An
unbuffered `writeln!` costs at least one `write` syscall, and more as soon as the template
interpolates. `write_fmt` calls `write` once per format fragment, so `writeln!(f, "line {i}")`
issues three syscalls per line, and `writeln!(f, "line")` issues one.

300,000 `println!`, same machine:

| Form | To a terminal | Redirected to a file |
| --- | --- | --- |
| `println!` | 128 ms | 356 ms |
| `println!` with `stdout().lock()` held | 133 ms | 377 ms |
| `BufWriter::new(stdout().lock())` | 5.1 ms | 7.0 ms |

Holding the lock gained nothing, and measured slightly worse in both columns. `Stdout` is
internally a `LineWriter`, so it issues one `write` syscall per newline whether or not you
hold the lock. Only block buffering removes the syscalls. Keep the lock for atomicity, never
as a speed change.

Since Rust 1.61 `StdoutLock` is `'static`. One binding is enough, and the old two-binding
dance is obsolete:

```rust
use std::io::Write;

fn heading(title: &str) -> std::io::Result<()> {
    let mut out = std::io::stdout().lock();
    writeln!(out, "== {title} ==")
}
```

## 10. A dropped `BufWriter` discards the final error

Verified with a writer whose `write` succeeds and whose `flush` fails:

| End the writer with | Result |
| --- | --- |
| `drop(w)` | Nothing. No panic, no message, no diagnostic |
| `w.flush()` | `Err("disk full")` |
| `w.into_inner()` | `Ok` |

```rust
use std::io::{BufWriter, Write};

struct FailingFlush;

impl Write for FailingFlush {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { Ok(buf.len()) }
    fn flush(&mut self) -> std::io::Result<()> { Err(std::io::Error::other("disk full")) }
}

fn reports_the_error() -> std::io::Result<()> {
    let mut w = BufWriter::new(FailingFlush);
    w.write_all(b"record")?;
    w.flush()                      // Err. Dropping w here reports nothing at all.
}
```

`into_inner` is not a substitute. It writes the buffered bytes to the inner writer and hands
that writer back. It never calls the inner writer's `flush`. A second buffer underneath, a
`LineWriter` or another `BufWriter`, still holds bytes after `into_inner` returns `Ok`. The
production symptom is a truncated tail with no error in the log. Any process that writes a
file and then exits needs the explicit `flush()?`.

## 11. Buffer capacity

`BufReader::new` and `BufWriter::new` both default to 8192 bytes on 1.97.0. std documents
the value as "currently 8 KiB, but may change", and it is 512 on `target_os = "espidf"`.
Never encode 8192 in a length calculation. Raise it when one logical record exceeds 8 KiB.
Below that size the default already turns many records into one syscall.

```rust
use std::io::BufWriter;

fn record_writer(f: std::fs::File) -> BufWriter<std::fs::File> {
    BufWriter::with_capacity(64 * 1024, f)      // one record reaches 64 KiB
}
```

## 12. `read_until` skips the UTF-8 check

`read_line` validates UTF-8. `read_until` returns the raw bytes.

| | `read_line(&mut String)` | `read_until(b'\n', &mut Vec<u8>)` |
| --- | --- | --- |
| Invalid bytes | `Err(InvalidData, "stream did not contain valid UTF-8")` | Returns the bytes |
| Delimiter | Kept in the buffer | Kept in the buffer |
| 1M lines of 20 bytes | 15.9-16.9 ms | 12.3-13.5 ms |

That is 1.2-1.3x. Quote a line width with any figure here, because the width decides the
answer: the gain is 1.6-1.7x at 8-byte lines and settles at 1.2-1.3x from 20 bytes up to 200.
The only extra work `read_line` does is one vectorised UTF-8 scan of the payload, and that
scan costs less per byte than the copy `read_until` already pays. At 300k lines the gap sits
inside the run-to-run noise, so do not quote a gain measured on a small file.

```rust
use std::io::{BufRead, BufReader, Read};

fn total_len<R: Read>(input: R) -> std::io::Result<usize> {
    let mut reader = BufReader::new(input);
    let mut record: Vec<u8> = Vec::with_capacity(128);
    let mut total = 0;
    loop {
        record.clear();
        if reader.read_until(b'\n', &mut record)? == 0 {
            break;
        }
        total += record.strip_suffix(b"\n").unwrap_or(&record).len();
    }
    Ok(total)
}
```

Take the swap when the payload is not text, or when a later stage validates it. Use the
`bstr` crate for byte-string ergonomics afterwards.

## 13. `lines()` allocates one `String` per line

`BufRead::lines` yields `io::Result<String>`, so it allocates once per line. `read_line`
appends into a `String` you own. Clear it each iteration and the whole file costs two
allocations. Counted with a counting global allocator on 1.97.0, a 200-line file: 201
allocations through `lines()`, 2 through the loop below.

```rust
use std::io::{BufRead, BufReader};

// `lines()` removes one trailing "\n", and one "\r" only when that "\r"
// precedes the "\n". It keeps every other trailing byte.
fn strip_eol(line: &str) -> &str {
    match line.strip_suffix('\n') {
        Some(s) => s.strip_suffix('\r').unwrap_or(s),
        None => line,
    }
}

fn count_lines(file: std::fs::File) -> std::io::Result<usize> {
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut n = 0;
    loop {
        line.clear();                        // keeps the buffer
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        n += strip_eol(&line).len();         // read_line keeps the terminator
    }
    Ok(n)
}
```

The swap also changes behaviour: `lines()` removes the terminator and `read_line` keeps it.
Remove it with `strip_suffix`, never with `trim_end()`. `trim_end()` removes all trailing
whitespace, so it also eats the spaces and tabs that `lines()` preserves. Measured on
`printf 'foo  \r\nbar\ttab \n'`:

| Line | `lines()` | `read_line` + `trim_end()` | `read_line` + `strip_eol` |
| --- | --- | --- | --- |
| 1 | `"foo  "` | `"foo"` | `"foo  "` |
| 2 | `"bar\ttab "` | `"bar\ttab"` | `"bar\ttab "` |

`trim_end()` is correct only when the caller does not care about trailing whitespace. On a
fixed-width record, a TSV with an empty last field, or a diff hunk, it deletes content, and
the failure reads as a parser fault far from the I/O rewrite that caused it.

Keep the `\r` strip inside the `Some` arm. An unconditional `s.strip_suffix('\r')` also
removes a trailing `\r` that no `\n` follows, which `lines()` keeps: on the bytes
`"a\nfoo\r"`, `lines()` yields `["a", "foo\r"]` and the unconditional form yields
`["a", "foo"]`. The `match` above is byte-identical to `lines()` on all 19531 strings of
length 0 to 6 over `{a, \r, \n, space, tab}`, measured on 1.97.0.

## 14. Triage

| Symptom | Cause | Fix |
| --- | --- | --- |
| `SipHasher13` hot, keys are internal | Default hasher on short keys | `FxHashMap`, section 1 |
| `SipHasher13` hot, keys come from outside the process | Default hasher, doing its job | `ahash::RandomState`, section 4 |
| A map is fast in benchmarks and slow on production traffic | Long probe sequences from colliding keys | Move that one map back to a seeded hasher |
| E0599 on `FxHashMap::new` or `::with_capacity` | std supplies both only for `RandomState` | `FxHashMap::default()`, or `HashMap::with_capacity_and_hasher(n, FxBuildHasher)` |
| E0599, `insert` exists but trait bounds not satisfied | A `nohash-hasher` key lacks `IsEnabled` | `impl IsEnabled for T {}`, section 5 |
| `IntMap` slower than `FxHashMap` | Strided ids with constant low bits under an identity hash | `FxHashMap`, section 5 |
| `clippy.toml` bans `HashMap` and nothing fires | The file uses the `FxHashMap` alias | Expected. Clippy matches the written path, section 6 |
| E0277 "cannot implement IntoBytes", or E0080 "applied to a type with padding" | The struct has padding | `#[repr(C, packed)]` or a filler field, section 8 |
| `write` syscalls dominate a file writer | No buffering | `BufWriter`, section 9 |
| `write` syscalls dominate stdout, lock already held | `Stdout` is a `LineWriter` | `BufWriter::new(stdout().lock())`, section 9 |
| Output truncated at the tail, no error anywhere | The `BufWriter` was dropped | Explicit `flush()?`, section 10 |
| One record costs several syscalls | The record exceeds the 8 KiB default | `with_capacity`, section 11 |
| `Err(InvalidData, "stream did not contain valid UTF-8")` | `read_line` on non-UTF-8 input | `read_until`, section 12 |
| One allocation per line in a reader | `lines()` yields an owned `String` | `read_line` into one cleared buffer, section 13 |
| A parser fails on trailing spaces after an I/O rewrite | `trim_end()` replaced what `lines()` did | `strip_suffix`, section 13 |
