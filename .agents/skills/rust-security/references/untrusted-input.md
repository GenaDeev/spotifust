# Untrusted-Input Parser Hardening

Contents: [threat surface](#threat-surface-by-format-class), [core rules](#core-rules),
[archive extraction](#archive-extraction-checklist),
[streamed containers](#streamed-container-and-backup-checklist),
[XML and JSON](#xml-and-json-specifics), [SQLite](#sqlite-and-embedded-database-specifics),
[hash map keys](#untrusted-keys-in-a-hash-map), [FFI boundary](#ffi-boundary-hardening),
[tests](#tests-that-must-exist).

Use this reference when Rust code parses data that a user, a network peer, or
another application supplies. Supply chain scanning protects you from a
malicious dependency. It does nothing about a malicious file. Every parser in
the trust boundary must treat its input as adversarial.

## Threat surface by format class

| Format class | Typical risks |
|---|---|
| ZIP and other archives | Path traversal, duplicate entries, central- and local-header disagreement, decompression bombs, forged declared sizes, forged hashes, entry-count explosion |
| Protobuf and other tagged binary formats | Deep nesting, unbounded repeated fields, integer overflow in offset and length arithmetic, unknown-field smuggling |
| XML | Entity expansion (billion laughs), external entity resolution, unbounded attribute or element counts, deeply nested elements |
| JSON | Deep nesting, enormous arrays, duplicate keys, number precision loss, unknown fields accepted silently |
| SQLite and other embedded databases | SQL injection through a filename or an identifier, path traversal on attach or import, untrusted schema |
| Binary containers with a header | Malformed header, integer overflow in offset arithmetic, length fields that exceed the file, truncated payloads |
| Streamed or chunked containers | Malformed frame lengths and counts, duplicate or reordered chunks, authentication failure late in the stream, resource exhaustion before the terminal record, partial application of a failed transaction |
| Geometry and coordinate data | Non-finite coordinates, out-of-range coordinates, coordinate arrays large enough to exhaust memory, overflow in projection arithmetic |

## Core rules

### Never trust a length field before you allocate

Cap the declared length against a documented maximum before you allocate. This
is the single most common resource-exhaustion bug in binary parsers.

```rust
const MAX_CHUNK_BYTES: u32 = 64 * 1024 * 1024; // 64 MiB

fn read_chunk(declared_len: u32) -> Result<Vec<u8>, ParseError> {
    if declared_len > MAX_CHUNK_BYTES {
        return Err(ParseError::TooLarge {
            declared: declared_len,
            cap: MAX_CHUNK_BYTES,
        });
    }
    Ok(vec![0u8; declared_len as usize])
}
```

Cross-check the declared length against the real remaining input where the
format allows it. A length field that exceeds the file is a malformed file, not
a large allocation.

Use checked arithmetic for every offset and length computation on untrusted
values. `checked_add`, `checked_mul`, and `try_into` turn a silent wrap in
release mode into an error you can return.

### Limit recursion depth

Nested protobuf messages, nested XML elements, and nested JSON objects all let a
small input drive a deep call stack. Carry an explicit depth counter. Reject
input that passes the cap. Do not rely on the stack to fail safely.

```rust
const MAX_NESTING_DEPTH: usize = 8;

fn parse_node(input: &[u8], depth: usize) -> Result<Node, ParseError> {
    if depth > MAX_NESTING_DEPTH {
        return Err(ParseError::TooDeep { cap: MAX_NESTING_DEPTH });
    }
    // ... recurse with depth + 1
    todo!()
}
```

### Validate floating-point input

Validate every float from untrusted input before arithmetic that can produce
`NaN` or infinity. A `NaN` that reaches a comparison silently breaks ordering
invariants and can panic later in code that assumed a total order.

```rust
fn validate_coordinate(x: f64, y: f64) -> Result<(), ParseError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(ParseError::InvalidGeometry("non-finite coordinate"));
    }
    // Range check against the coordinate system in use, for example WGS84.
    if !(-180.0..=180.0).contains(&x) || !(-90.0..=90.0).contains(&y) {
        return Err(ParseError::InvalidGeometry("coordinate out of range"));
    }
    Ok(())
}
```

Validate at the parse boundary, not at the point of use. A value that enters
your domain types unvalidated will be used unvalidated somewhere.

### Reject unknown schema versions and unknown fields

Accept only the schema identities and versions your reader implements. Reject
unknown fields rather than ignoring them. A reader that silently accepts drift
accepts attacker-chosen fields, and it hides format-version mismatches until a
later stage fails in a worse place.

Do not add a legacy reader or a migration path as a hardening workaround. A
second reader is a second attack surface.

### Reject path traversal

Inspect the decoded entry path with `Path::components`. Accept only
`Component::Normal` values. Reject `Prefix`, `RootDir`, `ParentDir`, and
`CurDir`. Also reject non-UTF-8 components and platform-reserved names. Do this
in the parser before any filesystem access.

Do not join the entry to a root and then call `canonicalize`. A new target does
not exist yet. A separate check followed by create also has a symlink race.

Open the private staging root once as a directory handle. Walk and create every
component relative to that handle. Disable symlink and reparse-point following
on every operation. Use exclusive creation for the final file. On Linux, use
`openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`. Use equivalent
directory-relative operations on other platforms. Reject archive symlink and
hard-link entries. If the platform API cannot provide these semantics, do not
extract an untrusted archive there.

A library `extract` or `unpack_in` helper is not a traversal gate. RustSec records an
arbitrary file write in `zip` extraction (RUSTSEC-2025-0168) and a symlink-following
`chmod` in `tar` `unpack_in` (RUSTSEC-2026-0067). Iterate the entries, validate each one
yourself, and write it through the staging-root handle. Gate the archive crate versions
with `cargo deny --config deny.toml --locked check advisories`.

## Archive extraction checklist

Apply all of these before you expose an extracted path to any consumer:

- Validate the archive manifest identity and version first. Reject anything
  else.
- Compare the central directory against the local headers. Reject disagreement.
- Reject duplicate entry names.
- Accept only normal UTF-8 path components. Reject absolute paths, parent
  components, platform-reserved names, symlinks, and hard links.
- Create each entry relative to the staging-root handle. Do not follow links at
  any component, and create the final file exclusively.
- Cap the entry count.
- Cap both the compressed size and the decompressed size, per entry and for the
  whole archive. This is the decompression-bomb defense. A declared
  uncompressed size is attacker-controlled. Enforce the cap while you stream,
  not only against the declared value.
- Verify CRC32 and any stronger digest the format carries.
- Validate the declared kind or type of every entry against what the consumer
  expects. Do not let one asset kind stand in for another.
- Validate the complete reference graph before you hand anything to a consumer.
  An entry that references a missing or unexpected sibling is a malformed
  archive.

Keep the archive schema owned by one implementation. If a Rust crate owns the
format, do not let a second implementation in another language reinterpret the
same bytes. Two readers with different validation produce a parser-differential
bug. For example, `tar` before 0.4.45 ignored PAX size headers in some cases, so one
archive unpacked differently in other tools (RUSTSEC-2026-0068). `tokio-tar` has the same
class of PAX bug and no patched release (RUSTSEC-2025-0111).

## Streamed container and backup checklist

For formats that stream frames or chunks and end with a summary record:

- Accept only the current envelope version and application contract.
- Enforce a hard cap on the total envelope size.
- Enforce a fixed, ordered chunk size for payload data. Reject out-of-order or
  duplicate chunks.
- Authenticate every frame, and authenticate the terminal summary record.
- Verify the record counts, the byte counts, the content hashes, and the
  end-of-file marker against the summary.
- **Authenticate before you mutate.** Complete verification first. Build a
  restore or apply plan second. Change live state third, and only through a
  journal and a transaction protocol that can roll back.
- Never apply a partially verified stream. A failure at the last frame must
  leave live state untouched.

## XML and JSON specifics

- Configure the XML parser with entity expansion disabled and external entity
  resolution disabled. `quick-xml` does not expand entities by default. Keep it
  that way and assert it in a test.
- Do not route JSON through an XML parser to reuse a code path. Keep JSON on a
  bounded JSON parser.
- Cap the total input size before you hand a document to either parser.

## SQLite and embedded database specifics

- Never build SQL by string concatenation with any value that came from
  untrusted input. Use bound parameters.
- A filename is untrusted input. A table name or column name from a file is
  untrusted input, and bound parameters do not cover identifiers. Validate
  identifiers against an allowlist.
- Validate the path of any database file you attach or import. Apply the path
  traversal rules above.
- Treat the schema of an imported database as untrusted. Verify it before you
  query it.

## Untrusted keys in a hash map

Decide the hasher by key provenance, not by profile. Keep std `RandomState` (SipHash 1-3
with random keys drawn per thread and varied for each `RandomState`, the std HashDoS
defense) for keys that an outside caller controls: HTTP headers, query parameters, JSON
object keys, archive entry names, and protocol field names. `FxHasher`, `FnvHasher`,
`nohash`, and `BuildHasherDefault<DefaultHasher>` (std uses the keys `(0, 0)`) have no
random secret, so an attacker can build a collision set offline. Use them only for internal
keys. Use `ahash::RandomState` when the untrusted-key path is measured hot. It is seeded only
while its default `runtime-rng` feature is on. Confirm that with
`cargo tree --locked -e features -i ahash`. The `rust-hot-path` skill has the measurements,
the seeding check, and the collision failure mode.

## FFI boundary hardening

When a Rust parser sits behind an FFI boundary, the boundary is part of the
trust boundary.

- **Validate at the FFI entry point.** Do not assume the platform layer
  pre-validated paths, byte buffers, enum discriminants, or lengths. The
  platform layer usually assumes Rust did it.
- **Return typed errors. Do not panic.** A panic that unwinds out of an
  `extern "C"` function aborts the process on Rust 1.81 and later. On earlier
  versions it is undefined behavior. Both outcomes destroy the caller. Make the
  boundary function unable to panic, or catch the panic at the boundary, and map
  every internal error into the boundary error type. See the `rust-panic-safety`
  and `ffi-error-progress-cancel` skills.
- **Never expose raw pointers through a generated binding.** Handle-based
  object types from a binding generator are safe. A raw `*const u8` or
  `*mut u8` field in a boundary type is not.
- **Own the format in Rust.** If Rust parses the format, no binding consumer
  should reimplement the schema. Give the consumer validated, typed results.

## Tests that must exist

Malformed-input tests belong beside the parser, in the crate that owns it.
Cover at minimum:

- Empty input and truncated input at several offsets.
- A length field larger than the remaining input.
- A length field at and above the documented cap.
- Nesting at and above the depth cap.
- Duplicate entries and out-of-order entries.
- Path traversal sequences, absolute paths, and non-UTF-8 paths.
- Non-finite and out-of-range numeric values.
- A valid file with one flipped byte in the authenticated region.
- An oversized buffer at the FFI boundary.

Add fuzz targets for every parser that reads a container format. See the
`rust-test-tools` skill for fuzz and property-test setup, and the
`rust-sanitizers-miri` skill for running those inputs under sanitizers.
