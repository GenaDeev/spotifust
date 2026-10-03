# Benchmarking Reference

Harness choice, Criterion authoring, the check that a benchmark measured real work, and CI gates.
[SKILL.md](../SKILL.md) section 6 holds the rules and the commands for one local run.

Contents:

1. Pick the harness
2. Manifest setup
3. Benchmark structure
4. Statistical configuration and the verdict
5. Wall time, CPU time, and async benchmarks
6. Prove the benchmark measured something
7. Gate benchmarks in CI
8. Compare two branches locally

## 1. Pick the harness

| Harness | Measures | Reach for it when |
|---------|----------|-------------------|
| Criterion 0.8 | Wall clock, in process | The default. Baselines, statistics, HTML reports |
| Divan 0.1.21 | Wall clock, in process | You want a lighter in-process harness with less code per benchmark |
| Hyperfine 1.20.0 | Wall clock of a whole process | The unit of work is one CLI invocation, not one function |
| Gungraun 0.19.4 | Valgrind instruction counts | You need a number that does not move with machine noise, or a CI gate. Linux only (Valgrind); not macOS or Windows |

Two traps:

- Gungraun is the rename of `iai-callgrind`. `iai-callgrind` is still published separately at
  0.16.1, so name the crate you mean. Gungraun also needs a `gungraun-runner` binary of the same
  version on `PATH`.
- Rust's built-in `#[bench]` attribute is nightly-only. On stable it fails with E0658, and adding
  `#![feature(test)]` then fails with E0554:

```rust,compile_fail,E0658
#[bench] // error[E0658]: use of unstable library feature `test`
fn bench_decode() {}
```

## 2. Manifest setup

```toml
[dev-dependencies]
criterion = { version = "0.8", features = ["html_reports"] }

[[bench]]
name = "decode"
harness = false   # required, or the built-in test harness eats the arguments
```

The `html_reports` feature writes the report to `target/criterion/report/index.html`.

Criterion 0.8 needs Rust 1.86 or later. It dropped the async-std integration. On Unix and Windows
targets it shifts the stack by a different `alloca` offset for each sample. This removes part of
the memory-layout bias that SKILL.md section 6 describes; symbol order and heap layout stay fixed.
`alloca` is an unconditional dependency with a C build step, not a feature you can turn off. The
benchmark API in this file is the same in 0.7 and 0.8. Keep the version that the workspace already
pins unless you are upgrading on purpose.

## 3. Benchmark structure

### Throughput

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;   // criterion::black_box is deprecated; use the std one
use std::time::Duration;

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    // Measurement window and sample count
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    for (label, bytes) in [("small", 8_192usize), ("medium", 65_536), ("large", 262_144)] {
        let data = vec![0u8; bytes]; // replace with a real fixture

        // Report bytes/sec next to the time
        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("decode", label),
            &data,
            |b, data| b.iter(|| decode(black_box(data))),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
```

Use `Throughput::Bytes` for byte streams and `Throughput::Elements` for item counts, such as one
rendered frame or one processed record per iteration.

### Expensive setup

Build the fixture once, outside the measured closure. Only the work under test belongs inside
`iter`.

```rust
fn bench_render(c: &mut Criterion) {
    let scene = load_fixture_scene();   // setup, not measured

    let mut group = c.benchmark_group("render");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(50);

    for px in [2048u32, 3000, 4500] {
        group.throughput(Throughput::Elements(1));  // one output per iteration
        group.bench_with_input(
            BenchmarkId::new("raster", px),
            &px,
            |b, &px| b.iter(|| render_to_buffer(black_box(&scene), black_box(px))),
        );
    }
    group.finish();
}
```

If the setup must run per iteration, use `iter_batched` so the setup cost stays out of the
measurement.

## 4. Statistical configuration and the verdict

Set the statistics for a whole target through the `criterion_group!` config form. It replaces
the plain `criterion_group!(benches, bench_throughput);` line shown above. The builder methods on
`Criterion` take `self` by value, so chain them on a fresh `Criterion::default()`:

```rust
criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))  // how long to measure
        .sample_size(200)                           // number of samples
        .warm_up_time(Duration::from_secs(3))       // warm-up before measurement
        .noise_threshold(0.05)                      // 5% noise threshold
        .significance_level(0.05)                   // p-value threshold
        .confidence_level(0.95);                    // confidence interval width
    targets = bench_throughput
}
criterion_main!(benches);
```

The same methods exist on `BenchmarkGroup`, where they take `&mut self`. Use the group form to
configure one group only, as the examples above do.

Criterion prints the verdict with a p-value. SKILL.md section 6 states how to read it:

```text
decode/medium           time:   [12.345 µs 12.456 µs 12.567 µs]
                        change: [-5.2312% -4.8956% -4.5600%] (p = 0.00 < 0.05)
                        Performance has improved.
```

## 5. Wall time, CPU time, and async benchmarks

Criterion measures wall time. For compute-bound work with no I/O this is the right metric. For
work that blocks on I/O, wall time reports the wait, not the cost.

Add the async harness only when the code under test is async. Do not wrap synchronous
compute-bound code in it.

```toml
[dev-dependencies]
criterion = { version = "0.8", features = ["async_tokio"] }
tokio = { version = "1", features = ["full"] }
```

```rust
fn bench_async(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("async_op", |b| {
        b.to_async(&rt).iter(|| async_operation(black_box(42)))
    });
}
```

## 6. Prove the benchmark measured something

SKILL.md section 6 states the rule and how to read the ratio. The evidence: LLVM rewrites
`(0..n).sum()` into the closed form `n * (n - 1) / 2`, and a `black_box` on each end does not
bring the loop back. Measured under `cargo +nightly bench`, `black_box` on both sides:
`closed_form_2m` 0.58 ns/iter and `closed_form_20m` 0.57 ns/iter. A 10x input moved the time by 1.02x. The same 10x change on a
pre-built `Vec<u64>` moved it 11.5x to 13.2x over four runs. A `black_box` inside the reduction,
as in `(0..black_box(n)).map(black_box).sum::<u64>()`, does emit the loop again, but then the
barrier is what you measure.

Put the two sizes in two benchmark functions of the same binary. The program below shows a folded
routine and a real one side by side:

```rust
use std::hint::black_box;
use std::time::Instant;

/// Seconds per call of `f`, averaged over `reps` calls.
fn per_call<T>(reps: u32, mut f: impl FnMut() -> T) -> f64 {
    let start = Instant::now();
    for _ in 0..reps { black_box(f()); }
    start.elapsed().as_secs_f64() / f64::from(reps)
}

fn main() {
    // Folded: LLVM rewrites `(0..n).sum()` into n * (n - 1) / 2.
    let folded = per_call(1_000_000, || black_box((0..black_box(20_000_000u64)).sum::<u64>()))
        / per_call(1_000_000, || black_box((0..black_box(2_000_000u64)).sum::<u64>()));

    // Real: the sum reads memory that the compiler cannot fold away.
    let small: Vec<u64> = (0..2_000_000).collect();
    let large: Vec<u64> = (0..20_000_000).collect();
    let real = per_call(50, || black_box(&large).iter().sum::<u64>())
        / per_call(50, || black_box(&small).iter().sum::<u64>());

    // Eighteen release runs: folded ratio 0.93 to 1.27, real ratio 11.8 to 14.7.
    println!("folded ratio = {folded:.2}, real ratio = {real:.2}");
}
```

The Criterion examples above pass every fixture through `black_box`. That guards against a
discarded result. It does not guard against a folded loop. Apply this scaling check to each of
them before you trust the number.

## 7. Gate benchmarks in CI

Three gates, from cheap to strict:

| Gate | Command | What a green result proves |
|------|---------|----------------------------|
| Compile | `cargo bench --locked --workspace --no-run` | The benchmark code compiles. Nothing about speed |
| Smoke run | `cargo test --locked --workspace --benches` | Each Criterion benchmark ran once without a panic. Criterion runs in test mode here and measures nothing |
| Regression | Gungraun limits against a main-branch baseline | Instruction counts did not grow past the limit, if the log compares against `main` with numbers, not `N/A` |

SKILL.md section 6 says why Criterion alone cannot gate a pull request. Compare Criterion
baselines on one dedicated runner with a fixed power state, and have a person read the `change`
interval.

For an automatic gate, use Gungraun on a Linux runner. Install `gungraun-runner` at the same
version as the `gungraun` library, or use the `gungraun/setup-gungraun` action pinned to a
commit SHA.

Gungraun stores results under `target/gungraun` in the workspace root, or under `GUNGRAUN_HOME`.
Separate CI jobs and runners do not share it, and SKILL.md section 6 says what a run without the
baseline reports. Run both in one job, checked out with the full history (`fetch-depth: 0`):

```bash
# The base commit of the pull request
git checkout --detach "$BASE_SHA"
cargo bench --locked --bench my_gungraun_bench -- --save-baseline=main

# The pull-request head. A regression over 5% in instructions exits with code 3.
git checkout --detach "$HEAD_SHA"
cargo bench --locked --bench my_gungraun_bench -- --baseline=main --callgrind-limits='ir=5%'
```

Or restore `target/gungraun` (or `GUNGRAUN_HOME`) from a cache or artifact that the main-branch
job saved. Read the log: each benchmark must print `Baselines: |main` and numeric comparisons.

Gungraun does not measure wall time. Its estimated cycles only correlate with wall time, so
confirm a large change with Criterion on real hardware.

## 8. Compare two branches locally

```bash
# Save a baseline on the base branch
cargo bench --locked -p my-crate --bench decode -- --save-baseline main-branch

# Switch branch and compare against it
git switch my-feature
cargo bench --locked -p my-crate --bench decode -- --baseline main-branch
```

Name the bench target with `--bench`; SKILL.md section 6 says why.
