# iOS Native Profiling

Profiling a Rust static library inside an iOS app. [SKILL.md](../SKILL.md) section 3 holds the
routing rule. Packaging (XCFramework, SwiftPM) belongs to the `rust-ios-build` skill, when it is
installed.

## Instruments

```text
Product -> Profile (Cmd-I) in Xcode
Time Profiler   -> CPU flamegraphs and call trees
Allocations     -> heap growth and allocation counts
Leaks           -> retain cycles
```

For Rust symbols in Instruments:

- Build the Rust static library with a profile that keeps line tables and does not strip, such
  as `ios-release` in the `rust-ios-build` skill.
- The Profile action builds the Release configuration by default. Keep its Debug Information
  Format at "DWARF with dSYM File".

## os_signpost markers

Mark the boundaries of major native stages so Instruments shows named intervals in the Points of
Interest track. That track shows only signposts logged with the `.pointsOfInterest` category; a
custom category appears under the os_signpost instrument instead. Emit the signposts from the
platform side around each call into the Rust library:

```swift
import os.signpost

let log = OSLog(subsystem: "com.example.app", category: .pointsOfInterest)
let id = OSSignpostID(log: log)

os_signpost(.begin, log: log, name: "HeavyStage", signpostID: id)
// call into the Rust library
os_signpost(.end, log: log, name: "HeavyStage", signpostID: id)
```

Emit signposts from Rust only through a platform shim behind a build feature or `cfg`. Keep the
shim out of the Android and host builds.

## MetricKit

MetricKit delivers aggregated `MXMetricPayload` reports (CPU time, hang rate, memory) from real
devices to an `MXMetricManagerSubscriber`. Use it to confirm that a local win holds on shipped
devices.
