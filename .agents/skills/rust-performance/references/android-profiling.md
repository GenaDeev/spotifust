# Android Native Profiling

Profiling Rust `cdylib` and `staticlib` code inside an Android app. [SKILL.md](../SKILL.md)
section 2 holds the routing rule. Crash symbolication with `ndk-stack` and `llvm-addr2line`
belongs to the `rust-debugging` skill, and HWASan builds belong to the `rust-sanitizers-miri`
skill, when they are installed.

Contents:

1. Prerequisites
2. Call-graph mode and frame pointers
3. Record a CPU profile with simpleperf
4. Record a system trace with Perfetto
5. Native memory profiler
6. Panics and memory errors found while profiling
7. Common mistakes

## 1. Prerequisites

| Requirement | Why it matters | Where to set it |
|-------------|----------------|-----------------|
| The app is profileable or debuggable | `simpleperf` and Perfetto cannot sample a release app otherwise | A debug build; or `<profileable android:shell="true" />` in `<application>` on Android 10 or later; or a rooted device |
| An unstripped `.so` kept on the host | The packaged copy is stripped, so symbolization needs this one | `target/<triple>/<ship-profile>/`, from a profile with line tables and no `strip` |
| Debug symbols kept in a debug APK | Android Studio's profiler and debugger see raw addresses otherwise | The Android Gradle `keepDebugSymbols` packaging option for your `.so` |

`<ship-profile>` is the Android ship profile: `android-jni` in the `rust-android-build` skill.

## 2. Call-graph mode and frame pointers

`simpleperf` records call graphs in two ways. The simpleperf documentation compares them:

| Mode | Flag | Needs | Behavior |
|------|------|-------|----------|
| DWARF | `-g` (same as `--call-graph dwarf`) | Unwind info (`.eh_frame` or `.debug_frame`) | Works on ARM and ARM64; higher overhead, usually 4000 Hz or less |
| Frame pointer | `--call-graph fp` | Frame pointers | Lower overhead; works well for native ARM64 code, not for 32-bit ARM |

Start with DWARF. Use `fp` on arm64 when the DWARF overhead distorts the profile.

For `fp` mode, `aarch64-linux-android` already keeps non-leaf frame pointers by default since
Rust 1.89, and the standard library ships with frame pointers since Rust 1.79.
`-C force-frame-pointers=yes` adds leaf functions and covers the x86 and x86_64 ABIs. It is a
profiling addition: the `rust-android-build` skill does not set it. If `.cargo/config.toml` already
has a `[target.'cfg(target_os = "android")']` table, append `"-C", "force-frame-pointers=yes"` to
its `rustflags` array. A second table with the same header is a TOML error, and Cargo then stops
with `could not load Cargo configuration`. Otherwise add this table:

```toml
[target.'cfg(target_os = "android")']
rustflags = ["-C", "force-frame-pointers=yes"]
```

Cargo joins it with every other matching `[target.<triple>]` and `[target.'cfg(...)']` table.

A `RUSTFLAGS` environment variable replaces this table and every other config-file rustflags
entry. Keep the flag in the config file.

## 3. Record a CPU profile with simpleperf

The NDK ships `simpleperf` and its scripts at `$ANDROID_NDK_HOME/simpleperf/`. `app_profiler.py`
starts the recording for one app, pulls `perf.data`, and collects the matching native libraries
from `-lib` into `binary_cache/`:

```bash
# Run from the directory that should receive perf.data and binary_cache/
python3 "$ANDROID_NDK_HOME/simpleperf/app_profiler.py" \
  -p com.example.app \
  -r "-e task-clock:u -f 1000 --duration 30 -g" \
  -lib target/aarch64-linux-android/<ship-profile>

# Flamegraph in flamegraph.html. Without -o, inferno writes report.html, and the next command
# overwrites it.
"$ANDROID_NDK_HOME/simpleperf/inferno.sh" -sc --record_file perf.data -o flamegraph.html

# Interactive HTML report
python3 "$ANDROID_NDK_HOME/simpleperf/report_html.py"
```

Use the app while it records, or the profile holds no samples from your code.

## 4. Record a system trace with Perfetto

Use Perfetto when you need the native profile next to scheduler, binder, and app frame data. The
Perfetto CPU-profiling guide lists Android 15 or later and a profileable or debuggable app (or a
userdebug or eng build of Android). It recommends a sampling frequency below 200 Hz per CPU.

```bash
adb shell perfetto -c - --txt -o /data/misc/perfetto-traces/trace <<'EOF'
duration_ms: 10000
buffers { size_kb: 65536 }
data_sources { config {
    name: "linux.perf"
    perf_event_config {
        timebase { counter: SW_CPU_CLOCK frequency: 100 }
        callstack_sampling {
            scope { target_cmdline: "com.example.app" }
            kernel_frames: true
        }
    }
}}
data_sources { config {
    name: "linux.process_stats"
    process_stats_config { scan_all_processes_on_start: true }
}}
EOF

adb pull /data/misc/perfetto-traces/trace .
# Open at https://ui.perfetto.dev
```

`scope.target_cmdline` limits unwinding to samples taken while your app runs. Without it, Perfetto
tries to unwind every sampled process. Pass the config on stdin (`-c -`): before Android 12, SELinux blocks a config file path
on a non-rooted device.

## 5. Native memory profiler

1. Open the **Profiler** tab in Android Studio.
2. Select the app process.
3. Click **Record native allocations**.
4. The profiler shows native memory next to the Java heap.

Stack traces in the native profiler need unstripped symbols. Debug builds have them. Release
builds need the unstripped `.so` from the host.

## 6. Panics and memory errors found while profiling

- A panic during a profiling session: the `rust-panic-safety` skill owns the privacy-safe panic
  hook. With `panic = "abort"`, Android copies the panic message into the tombstone "Abort
  message", so a panic message is a log line.
- Crash or corruption during a session: run the same scenario under HWASan. HWASan runs only on
  64-bit Arm. Android 14 and later start it through `wrap.sh` for a debuggable app; Android 10 to
  13 need a HWASan system image. The NDK marks ASan unsupported as of 2023. The build and device
  steps are in the `rust-sanitizers-miri` skill.

## 7. Common mistakes

| Mistake | Fix |
|---------|-----|
| `simpleperf` or Perfetto reports permission denied, or records no app samples | Make the app debuggable, or add `<profileable android:shell="true" />` (Android 10+), or use a rooted device |
| Flamegraph shows hex addresses or `<unknown>` for Rust frames | Pass the directory of the unstripped `.so` to `-lib`, not the packaged copy |
| Stacks are truncated in `--call-graph fp` mode | Check `-C force-frame-pointers=yes` in the config-file rustflags for that ABI; or record with `-g` |
| Profile a `dev` or opt-level 1 build | The numbers do not transfer. Profile the ship profile with line tables |
| Perfetto trace is dominated by other processes | Add `scope { target_cmdline: "<package>" }` and keep the frequency at 100 Hz |
| HWASan reports nothing on an x86_64 emulator | HWASan runs only on 64-bit Arm; use an arm64 device |
| Profile numbers differ widely between runs | Thermal throttling. Cool the device, stop background sync, and repeat the run |
