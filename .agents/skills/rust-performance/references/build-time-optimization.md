# Build-Time Optimization

Reference for cutting Rust compile time in a multi-crate workspace, especially one that cross-compiles to several mobile targets. The Cargo book chapter [Optimizing Build Performance](https://doc.rust-lang.org/cargo/guide/build-performance.html) covers the general settings.

Contents: 1. Diagnose first; 2. sccache; 3. Cross-compilation target matrix; 4. Workspace splitting; 5. Linkers; 6. Other quick wins; 7. Generic signature shape and the thin wrapper.

## 1. Diagnose first

Start with `cargo build --locked --timings` and `cargo llvm-lines`, as SKILL.md sections 5 and 9
show. Attack the crates that the timeline shows over 10 s or on a long sequential chain. Check
the generic signature shape in section 7 before you split crates.

---

## 2. sccache

sccache caches `rustc` output. It matters most when the same crate is compiled many times, which is exactly what a multi-target mobile matrix does.

```bash
# Install
cargo install --locked sccache   # or: brew install sccache

# Enable for Rust builds, in .cargo/config.toml or in the environment
export RUSTC_WRAPPER=sccache

# Check the hit rate. Expect over 80% on a rebuild.
sccache --show-stats
```

In GitHub Actions, the cache persists between runs only with the GitHub Actions backend. Set both
variables through `GITHUB_ENV`; an `env` block on the setup step applies only to that step. The
action's README enables the cache only on non-release runs, so a release build compiles from source:

```yaml
- uses: mozilla-actions/sccache-action@fc920bf0ec8de6ee65d409111f7ec508035751ba # v0.0.11
  if: github.event_name != 'release' && github.event_name != 'workflow_dispatch'
- if: github.event_name != 'release' && github.event_name != 'workflow_dispatch'
  run: |
    echo "SCCACHE_GHA_ENABLED=true" >> "$GITHUB_ENV"
    echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"
```

Without `SCCACHE_GHA_ENABLED=true`, sccache writes a local disk cache that the next job never sees.

The target triple and code-generation options are part of the cache key.
Compilations for Android, iOS, and the host do not share one cached object merely
because a crate is pure Rust. `sccache` still helps when the same target and
compiler inputs repeat across local or CI builds. Set `RUSTC_WRAPPER=sccache`
for every target build, then measure the hit rate per target instead of assuming
cross-target reuse.

---

## 3. Cross-compilation target matrix

Android has four common target mappings. A product builds the subset in its
declared shipping ABI matrix:

```text
aarch64-linux-android
armv7-linux-androideabi
i686-linux-android
x86_64-linux-android
```

That 4x multiplier is usually the single biggest build-time factor in a mobile Rust project.

### Build only what you need during development

```bash
# Local iteration: arm64 only, which covers most devices and ARM emulators
cargo build --locked --target aarch64-linux-android

# Release: the whole matrix
for target in aarch64-linux-android armv7-linux-androideabi \
              i686-linux-android x86_64-linux-android; do
  cargo build --locked --release --target "$target"
done
```

### Parallelize the matrix in CI

```yaml
strategy:
  matrix:
    target:
      - aarch64-linux-android
      - armv7-linux-androideabi
      - i686-linux-android
      - x86_64-linux-android
# Each job builds one target, so wall-clock time is one target build.
```

### iOS targets

```bash
# Device
cargo build --locked --release --target aarch64-apple-ios

# Simulator on an Apple Silicon host
cargo build --locked --release --target aarch64-apple-ios-sim
```

`x86_64-apple-ios` (the simulator on an Intel host) is optional. Build it only when a supported
consumer still needs it. Packaging the slices into an XCFramework belongs to the `rust-ios-build`
skill, when it is installed.

Binding generation for an FFI layer runs once per target and costs almost nothing next to
compilation. See `uniffi-packaging-versioning` and `rust-android-build`.

---

## 4. Workspace splitting for parallelism

```bash
cargo tree --locked | head -30
cargo tree --locked --depth 1
cargo tree --locked --prefix depth

cargo build --locked --timings   # the timeline shows how much parallelism you get
```

Rules that actually help:

- Break circular dependencies first. Nothing else parallelizes until they are gone.
- Put proc-macros in their own crate. A proc-macro crate blocks every dependent crate.
- Keep frequently-changed code isolated, so a small edit invalidates a small part of the cache.
- Keep leaf crates such as shared error types and domain types small and stable. Everything depends on them, so every edit to them rebuilds everything.

See `rust-crate-architecture` for the layering rules behind this.

---

## 5. Linkers

### Host builds

`rust-lld` is the default linker for `x86_64-unknown-linux-gnu` since Rust 1.90. Do not configure `-C link-arg=-fuse-ld=lld` there. It is dead config on 1.97.0. Verified in a `rust:1.97-slim` container with no flags and no `.cargo/config.toml`: `readelf -p .comment` on the built binary reports `Linker: LLD 22.1.6`, and adding the flag changes nothing.

mold is the remaining upgrade on Linux:

```toml
# mold, the fastest option. Linux ELF only.
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

The [mold README](https://github.com/rui314/mold) reports mold 4.9x faster than LLVM lld and 1.9x faster than wild at the median of its August 2026 benchmarks. Measure the link time of your own project before you switch. mold is Linux ELF only. With lld or mold, add `-Wl,--no-rosegment` before you profile with `perf`; see [cargo-flamegraph-setup.md](cargo-flamegraph-setup.md).

`wild-linker` 0.10.0 is a newer incremental Linux linker. It is less mature than mold, so treat it as an experiment. Note the name: the crates.io crate called `wild` is an unrelated Windows glob-expansion library.

### Platform rules

- **macOS**: the default Apple linker needs no alternative. `mold` is Linux ELF only.
- **Android**: link through the NDK clang driver, which uses the NDK's own `lld`. The `rust-android-build` skill sets `CARGO_TARGET_<TRIPLE>_LINKER`. Do not replace that driver with mold or a host linker.
- **iOS**: use the Xcode-provided Apple linker through the standard Cargo iOS target configuration. Do not override.

---

## 6. Other quick wins

```toml
[profile.dev]
debug = "line-tables-only"     # faster than full debug info, still gives backtraces with line numbers
```

`debug = "line-tables-only"` removes variable and type info, so a debugger shows no local
variables. Set `debug = true` again before a debugger session; see `rust-debugging`.

Do not copy `split-debuginfo = "unpacked"` into a shared profile to speed up macOS links. It is
already the Cargo default on macOS, and on Linux it moves debug info into `.dwo` files next to the
objects.

```bash
# Sometimes faster for full rebuilds and for CI, where the incremental cache is cold
CARGO_INCREMENTAL=0 cargo build --locked
```

Pass `--locked` in every scripted build. It stops a dependency resolution from silently changing
what you measured.

---

## 7. Generic signature shape

An `impl Trait` parameter taken **by value** is the most expensive generic signature in a build. The callee owns the argument, so it must forward `&mut arg` to the next generic call. With a delegating `impl<T: Trait + ?Sized> Trait for &mut T` in scope that forwarding type-checks, and each call level instantiates the callee one `&mut` deeper. Behind `&mut impl Trait` the callee already holds a reference and forwards it unchanged, so every level instantiates at the root type.

```rust
trait Sink { fn put(&mut self, b: u8); }
impl<T: Sink + ?Sized> Sink for &mut T { fn put(&mut self, b: u8) { (**self).put(b) } }

// By value: `leaf` is instantiated once per call depth.
#[inline(never)] fn leaf(mut x: impl Sink) { x.put(1); }
#[inline(never)] fn mid(mut x: impl Sink) { leaf(&mut x); }
#[inline(never)] fn top(mut x: impl Sink) { leaf(&mut x); mid(&mut x); }

// Behind `&mut`: every level is instantiated at the root type.
#[inline(never)] fn leaf_ref(x: &mut impl Sink) { x.put(1); }
#[inline(never)] fn mid_ref(x: &mut impl Sink) { leaf_ref(x); }
#[inline(never)] fn top_ref(x: &mut impl Sink) { leaf_ref(x); mid_ref(x); }
```

Measured on rustc 1.97.0, aarch64-apple-darwin, `-C opt-level=0`, with one root type `Buf`:

| Form | Monomorphized symbols for the three functions |
| --- | --- |
| by value | 4: `top::<&mut Buf>`, `mid::<&mut &mut Buf>`, `leaf::<&mut &mut Buf>`, `leaf::<&mut &mut &mut Buf>` |
| behind `&mut` | 3: `top_ref::<Buf>`, `mid_ref::<Buf>`, `leaf_ref::<Buf>` |

`leaf` appears twice in the by-value column because two call paths reach it at two different depths. Count the copies with `nm target/release/libmycrate.rlib | rustfilt | grep '::leaf'`, after `cargo install --locked rustfilt`.

### Measured build cost of the three shapes

One generated crate in three copies that differ only in one trait method signature. Each copy has 50 nested struct levels, where `T<i>` holds a `Vec<T<i+1>>` and a `Vec<T<i+2>>`, so many distinct paths reach the same level at different depths. Each copy has one `Ser` impl per level and 300 `pub` entry functions, 603 lines in total.

```text
fn ser(&self, out: impl io::Write)      -> io::Result<()>;   // by value
fn ser(&self, out: &mut impl io::Write) -> io::Result<()>;   // behind &mut
fn ser(&self, out: &mut dyn io::Write)  -> io::Result<()>;   // behind &mut dyn
```

```bash
# One timed run. Repeat it three times per variant and take the median.
MEASURE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/rust-build-shape.XXXXXX")"
case "$MEASURE_DIR" in
  "${TMPDIR:-/tmp}"/rust-build-shape.*) ;;
  *) echo "unexpected measurement path: $MEASURE_DIR" >&2; exit 2 ;;
esac
trap 'rm -rf -- "$MEASURE_DIR"' EXIT
RUSTC_WRAPPER= CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$MEASURE_DIR" \
  cargo build --release -q
```

| Signature | `cargo check` | `cargo build` | `cargo build --release` | `ser` symbols in the rlib | rlib bytes |
| --- | --- | --- | --- | --- | --- |
| `out: impl io::Write` | 0.18 s | 0.58 s | 10.7 s | 381 | 1200168 |
| `out: &mut impl io::Write` | 0.18 s | 0.24 s | 0.71 s | 28 | 539080 |
| `out: &mut dyn io::Write` | 0.18 s | 0.25 s | 0.67 s | 51 | 222376 |

Release figures are the median of three runs, each from a removed target directory, with `sccache` and incremental compilation off. The by-value form costs 15x the release build time of `&mut impl io::Write`, 2.2x its rlib size, and 5.4x the rlib size of the `&mut dyn io::Write` form.

Three rules follow:

- Take a generic writer, reader or sink parameter by `&mut impl Trait`. `&mut dyn Trait` saves almost nothing more on this shape and it costs a virtual call per write.
- `cargo check` is identical for all three, to the hundredth of a second. Type-checking is depth-independent, so a check-only CI gate reports none of this. Gate build time on `cargo build --release`.
- Confirm the mechanism from the deepest symbol. In the by-value rlib it is `<T48 as Ser>::ser::<&mut &mut ... &mut Vec<u8>>`, with 48 `&mut` levels.

### Thin wrapper: compile the body once

A high `Copies` count in `cargo llvm-lines` means the generic was instantiated many times. Keep the generic surface and move the body into a concrete inner function:

```rust
// Before: the whole body is monomorphized for every T.
fn send<T: AsRef<[u8]>>(data: T) {
    // ... large body ...
}
```

```rust
// After: a thin generic wrapper plus one concrete inner copy.
fn send<T: AsRef<[u8]>>(data: T) {
    fn inner(data: &[u8]) {
        // ... large body, compiled once ...
    }
    inner(data.as_ref())
}
```

Measured on rustc 1.97.0 at `-C opt-level=3`, one 15-line body reached through six argument types (`&str`, `&String`, `String`, `&Rc<str>`, `&Cow<'_, str>`, `&Box<str>`): the fully generic form emitted 917 LLVM IR lines over its 6 copies, and the thin-wrapper form emitted 53 lines over the same 6 copies plus 148 lines in the one `inner` copy, so 201 in total. That is 4.6x less IR for the same work.

Check the crates with the heaviest generic iterator chains and the widest trait-bound surfaces first. The wrapper shrinks each copy; it does not reduce the number of copies. To cut the copy count, change the signature shape as shown above.
