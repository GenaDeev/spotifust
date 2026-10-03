# Host Profiler Setup: cargo flamegraph and samply

Setup and invocations for `cargo flamegraph` 0.6.14 and `samply` 0.13.1 on Linux and macOS.
[SKILL.md](../SKILL.md) defines the `profiling` profile that every command here uses. Criterion
authoring is in [benchmarking.md](benchmarking.md).

Contents: Linux prerequisites (`--no-rosegment`, DWARF or frame pointers), macOS prerequisites,
Installation, Invocations, Check the result, Read a flamegraph.

## Linux prerequisites

```bash
# Install perf
sudo apt-get install linux-tools-common linux-tools-$(uname -r)  # Debian/Ubuntu
sudo dnf install perf                                            # Fedora
sudo pacman -S perf                                              # Arch

# Allow perf and samply for unprivileged users (choose one)
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid                        # until reboot
echo 'kernel.perf_event_paranoid = 1' | sudo tee /etc/sysctl.d/99-perf.conf  # permanent
sudo sysctl -p /etc/sysctl.d/99-perf.conf
```

Do not change the process-global `kernel.kptr_restrict` setting as a routine prerequisite.
User-space Rust stacks do not need kernel symbols. When a specific kernel profile needs them,
follow the host security policy and restore the setting after the bounded diagnostic session.

### `--no-rosegment` for lld

Rust 1.90 made `rust-lld` the default linker for `x86_64-unknown-linux-gnu` only. Other Linux
targets still use the system linker unless you configure lld or mold. With lld or mold, `perf`
cannot build correct stacks unless the linker gets `--no-rosegment`. The flag puts read-only data
in the executable segment, so keep it out of the builds that ship. Set it for the profiling build
and the `cargo flamegraph` run only:

```bash
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-Clink-arg=-Wl,--no-rosegment"
cargo build --locked --profile profiling --bin myapp
cargo flamegraph --profile profiling --bin myapp
```

Set the variable for both commands. `cargo flamegraph` runs its own `cargo build`, and a build
without the flag replaces the binary. For another Linux target that links with lld or mold, use
its own variable, for example `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS`.

`CARGO_TARGET_<TRIPLE>_RUSTFLAGS` is the environment form of `[target.<triple>] rustflags`. Cargo
joins it with the config-file `[target.*]` entries (checked on cargo 1.98.1). Like any target
entry, it makes Cargo drop `[build] rustflags`. A `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS`
variable replaces every target entry, this one included, so the stacks break again.
[build-configuration.md](build-configuration.md) section 1 shows the precedence.

### Unwinding: DWARF or frame pointers

`cargo flamegraph` runs `perf record --call-graph dwarf,64000`. DWARF mode unwinds from
`.eh_frame`, which rustc emits by default; the `profiling` profile adds the function names, inline
frames, and file:line. DWARF mode does not need frame pointers.

DWARF mode copies only 64000 bytes of user stack per sample. A deeper stack (deep recursion,
large stack frames) stops at the same depth in every sample. Frame-pointer mode has no such limit.
Add `-Cforce-frame-pointers=yes` to the same variable, rebuild, and pass a custom `perf record`
command with `--cmd`:

```bash
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-Clink-arg=-Wl,--no-rosegment -Cforce-frame-pointers=yes"
cargo build --locked --profile profiling --bin myapp
cargo flamegraph --profile profiling --cmd "record -F 997 --call-graph fp" --bin myapp
```

`--cmd` replaces the whole `perf record` argument list, and `cargo flamegraph` rejects it together
with `--freq`, so put `-F` inside it. The standard library ships with frame pointers since Rust
1.79, and `aarch64-unknown-linux-gnu` keeps non-leaf frame pointers by default since Rust 1.89.
The flag adds frame pointers to your own code on every target.

## macOS prerequisites

`cargo flamegraph` uses `xctrace` on macOS. Grant the profiling permission when macOS prompts. Do
not weaken System Integrity Protection for profiling. `cargo flamegraph` rejects `--cmd`, and any
`--freq` other than the default 997, on macOS, because it drives `xctrace` at its fixed rate.

`samply` needs no extra setup to launch a program. To attach to a running process on macOS, run
`samply setup` once, and again after each `samply` update.

## Installation

```bash
cargo install --locked flamegraph
cargo install --locked samply
```

`cargo flamegraph` folds stacks with `inferno`, which is pure Rust and needs no extra install.

## Invocations

`cargo flamegraph` has no `--locked` option, and it runs its own `cargo build` (or `cargo bench
--no-run` for `--bench`) without one. Run the matching `cargo build --locked` or `cargo bench
--locked --no-run` first, with the same profile, package, and target flags.

```bash
# A binary with arguments
cargo build --locked --profile profiling --bin myapp
cargo flamegraph --profile profiling --bin myapp -- --workers 4 --input data.bin

# One integration test binary. It profiles the harness and every test in it,
# so filter to one test after --.
cargo build --locked --profile profiling --test integration_tests
cargo flamegraph --profile profiling --test integration_tests -- test_name

# A Criterion benchmark. --bench puts Criterion in benchmark mode;
# --profile-time skips the statistics phase so it stays out of the profile.
cargo bench --locked --profile profiling --no-run -p my-bench-crate --bench decode
cargo flamegraph --profile profiling -p my-bench-crate --bench decode -- \
    --bench --profile-time 10 decode_large

# An example
cargo build --locked --profile profiling --example my_example
cargo flamegraph --profile profiling --example my_example

# A nested workspace, run from the repository root so that relative
# fixture paths in the program still resolve
cargo build --locked --manifest-path path/to/Cargo.toml --profile profiling --bin myapp
cargo flamegraph --manifest-path path/to/Cargo.toml --profile profiling --bin myapp -- \
    run --input fixtures/sample.bin

# Write to a chosen file, then open it
cargo flamegraph --profile profiling -o /tmp/fg.svg --bin myapp && open /tmp/fg.svg      # macOS
cargo flamegraph --profile profiling -o /tmp/fg.svg --bin myapp && xdg-open /tmp/fg.svg  # Linux

# samply on the same build
samply record ./target/profiling/myapp --workers 4
```

On Linux, `--freq <HZ>` changes the sample rate; 997 Hz is the default, a prime that avoids
aliasing with periodic work.

## Check the result

- Frames show Rust function names, not hex addresses. Hex addresses mean the build has no
  symbols: check that the command used `--profile profiling`, and that the profile sets
  `strip = "none"`.
- On Linux, `readelf -S target/profiling/myapp | grep -c debug_info` prints a non-zero count.
- Frames show demangled names, not `_R...` strings. Raw v0 names mean the tool is too old for
  the default mangling since Rust 1.97. `cargo flamegraph` 0.6.14 demangles v0 names itself, so
  reinstall it with `cargo install --locked flamegraph`. For direct `perf report` or
  `perf script` output, use Linux perf 6.16 or later (the perf in Ubuntu 24.04 and Debian 13 is
  too old), or pipe the text through `rustfilt`.

## Read a flamegraph

Width is the share of samples. The left-to-right order is alphabetical, not time. The same
patterns apply to a `simpleperf` flamegraph from an Android device.

| Pattern | Meaning | Action |
|---------|---------|--------|
| Wide plateau at the top | Leaf hotspot | Change that function; see the `rust-hot-path` skill |
| Wide frame with tall narrow towers above it | Hot dispatch | Reduce call overhead, inline, or devirtualize |
| Unexpected `alloc` / `dealloc` / `drop` frames | Allocation pressure | Confirm the sites with DHAT, then reuse buffers |
| Stacks cut off, or `[unknown]` under your frames | Unwinding failed | See the failure triage in SKILL.md |
| Raw `_R...` symbol names | The tool cannot demangle v0 symbols | See the failure triage in SKILL.md |

A differential flamegraph uses color: red marks growth, blue marks a reduction.
