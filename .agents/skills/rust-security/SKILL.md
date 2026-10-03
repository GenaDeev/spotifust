---
name: rust-security
description: Use when running cargo-audit or cargo-deny, editing deny.toml, triaging a RUSTSEC advisory or CVE in a dependency, vetting a new or updated crate for typosquat, malicious crate, or compromised-release risk, or hardening a Rust parser that reads untrusted files, archives, or binary formats. Not for authentication, secret storage, cryptographic design, or TLS policy (TLS belongs to rust-networking). Triggers on "cargo audit", "cargo deny", "RUSTSEC", "supply chain", "yanked", "path traversal", "decompression bomb".
license: BSD-3-Clause
---

# Rust Dependency and Parser Security

| Request | Route |
|---|---|
| Advisories, `deny.toml`, dependency vetting, registry and source policy | This skill. |
| Untrusted file, archive, backup, or binary-format parsing | This skill. |
| Publishing, Trusted Publishing, binary provenance, SBOMs | The `rust-crate-release` skill. |
| Unsafe-code soundness, Miri, sanitizers | The `rust-unsafe` or `rust-sanitizers-miri` skill. |
| TLS transport policy | The `rust-networking` skill. |
| Authentication, secret storage, cryptographic protocol design, key lifecycle | Not this skill. Follow the domain policy and its security review. |

Other skills named in this file apply when they are installed.

## Review gate

Block a change that does any of these:

- Adds a package to `Cargo.lock` without an identity check and the source triage of
  section 5.
- Adds an `=` requirement as a vetting device.
- Adds an `[advisories].ignore` entry for a `malicious` advisory, or one without an `id`,
  a `reason`, and a tracking issue.
- Adds a `[bans].skip` entry without an exact version and a causal `reason`.
- Widens the license allowlist to clear one dependency, with no recorded license review.
- Allocates from an untrusted length field without a cap.
- Extracts an archive entry without its own traversal check.
- Panics on malformed input across an FFI boundary.

Do not compile an unvetted package. Until section 5 step 3 passes for every new
`Cargo.lock` package, do not run `cargo build`, `check`, `test`, `clippy`, `doc`, or `run`,
and keep rust-analyzer off the workspace. Build scripts and proc macros run at compile time.

## Read the cargo-deny pin first

Before you write a `cargo deny` command or a `deny.toml` key, read the `cargo-deny`
version that CI pins. The CLI and the config schema changed in recent releases. The
commands in this skill need 0.20 or later, where `--config` is a root option. On 0.20,
`check --config` fails with `unexpected argument '--config' found`. On 0.19, root
`--config` fails with the same message. Before 0.19.1, an offline check never fails for a
stale database, and on 0.19.0 the `unsound` scope reports the wrong set. Read the version
table in [references/deny-policy.md](references/deny-policy.md#version-boundaries) when the
pin is older than 0.20, or when you add a key or a flag.

Use `--locked` in every command that resolves dependencies. Without it, a check can pass
against a lockfile that CI never builds.

Run cargo-deny from the workspace root. If you must pass `--manifest-path
<workspace>/Cargo.toml`, also pass `--config <workspace>/deny.toml`. A relative `--config`
path resolves against the current directory. When no file is there, cargo-deny prints only
`[WARN] config path '...' doesn't exist, falling back to default config`, runs the default
policy instead of yours, and can exit 0. Treat that warning as a failure.

## 1. cargo-audit

`cargo-audit` compares `Cargo.lock` with the RustSec advisory database. It fetches the
database on every run unless you pass `--no-fetch`. There is no `cargo audit fetch`
subcommand. It reports a vulnerability as an error. It reports an informational advisory
(`unmaintained`, `unsound`, `notice`) and a yanked version as a warning. It does not check
licenses or sources.

```bash
cargo install cargo-audit --locked
cargo audit                       # exit 1 on a vulnerability
cargo audit --deny warnings       # also fail on informational advisories and yanked versions
cargo audit --file path/to/Cargo.lock
cargo audit --json | jq -r '.vulnerabilities.list[]
  | "\(.advisory.id) \(.package.name) \(.package.version) \(.advisory.categories) unaffected=\(.versions.unaffected)"'
cargo audit bin target/release/<binary>
```

`cargo audit bin` audits a shipped binary. The result is complete only for a binary built
with `cargo auditable build`. For other binaries it recovers part of the dependency list
from panic messages.

Read [references/incidents.md](references/incidents.md) when you parse the terminal
report of `cargo audit`. It has a sample finding block.

## 2. cargo-deny

`cargo-deny` is the blocking gate. It checks advisories, licenses, bans (duplicate
versions, wildcard requirements, named crates), and sources. Use `cargo-audit` for a fast
advisory-only look.

```bash
cargo deny --config deny.toml --locked check               # all four checks, cargo-deny >= 0.20
cargo deny --config deny.toml --locked check advisories    # or licenses, bans, sources
cargo deny --config deny.toml --locked --offline check advisories    # no database fetch
```

A green check proves only that no known advisory, license, ban, or source rule matches the
lockfile. It does not catch a malicious crate that has no advisory yet. The gate in
section 5 covers that.

Read [references/deny-policy.md](references/deny-policy.md) when you write or change
`deny.toml` or the CI job. It holds the annotated policy, the license allowlist, the
`ignore` and `skip` entry formats, and the CI job steps.

In CI, run `cargo deny --config deny.toml --locked check` as its own job, on pull requests,
on pushes to the default branch, and on a schedule. New advisories land against unchanged
lockfiles.

| Failing check or message | First action |
|---|---|
| `advisories` | Read the RUSTSEC ID and follow section 4. |
| `licenses` | Find the crate with `cargo tree --locked -i <crate>`. Do not widen the allowlist without a license review. |
| `bans` | Run `cargo tree --locked -d`. Unify the versions. Add a `skip` entry only with a causal `reason`. |
| `sources` | A dependency came from a registry or git remote that policy does not allow. Check for an accidental `[patch]`, a deliberate `[patch]` whose remote is missing from `allow-git`, a leaked `path` override, or a new transitive git dependency. |
| `unexpected argument '--config' found` | The command and the pinned cargo-deny version disagree. See "Read the cargo-deny pin first". |

Reproduce a CI failure locally with the same command and the same pinned version. Fix the
root cause. Do not extend an `ignore` or `skip` list only to make the job green.

## 3. Advisory kinds

| Kind | How the advisory marks it | Response |
|---|---|---|
| Vulnerability | No `informational` field | Upgrade. If no fix exists, see section 4. |
| `unsound` | `informational = "unsound"` | Find out whether your code reaches the unsound API. |
| `unmaintained` | `informational = "unmaintained"` | Plan a replacement. A time-boxed ignore is acceptable. |
| `notice` | `informational = "notice"` | Read it and decide. |
| Malicious crate | `categories = ["malicious"]`, `patched = []` | Act now. Follow "Malicious advisory" below. |

`yanked` is not an advisory kind. It is a registry state that both tools read from the
index. Move off a yanked version, and set `yanked = "deny"`.

## 4. Respond to an advisory

```bash
cargo tree --locked -i <crate>@<version>   # which dependency pulls it in
cargo update -p <crate> --dry-run          # is a semver-compatible fix available?
cargo update -p <crate>
cargo deny --config deny.toml --locked check advisories
```

Prefer an upgrade. If the fix is outside the range that a parent crate allows, upgrade the
parent. Then prefer a `[patch.crates-io]` entry that points at a fixed `rev`. In the same
change, add the fork URL to `[sources].allow-git`, with a reason and a tracking link in a
comment. Otherwise `unknown-git = "deny"` fails the `sources` check. Add an ignore entry
only when no fix exists. Never ignore a `malicious` advisory: its code already ran, and the
ignore hides the crate from every later check.

Every `[advisories].ignore` entry must carry:

- `id`: the RUSTSEC ID.
- `reason`: why the advisory is safe to ignore in this workspace. "Not exploitable" alone
  is not a reason.
- A tracking issue link and a re-evaluation date in a trailing comment.

Set `unused-ignored-advisory = "deny"` (cargo-deny 0.19.0 and later), so an ignore entry
fails the check after the advisory leaves the graph. Remove the entry in the same change
that upgrades the dependency. Review the whole ignore list on every dependency bump.

The default time box is below. Replace it when your team has its own policy.

| Severity | Time before the ignore becomes blocking |
|---|---|
| Low or informational (unmaintained, no runtime exploit) | 90 days |
| Medium (the exploit needs conditions your build does not meet) | 30 days |
| High or critical | No ignore. Upgrade, patch, or remove the dependency within 7 days. |

An unmaintained proc-macro crate is a valid low-severity ignore, because it runs only at
compile time. An advisory in a crate that reads untrusted input at runtime is not.

### Malicious advisory

Assume that the code ran on every machine that built or tested the workspace with the
crate, or opened it in an IDE that runs rust-analyzer. Build scripts and proc macros run at
compile time, and rust-analyzer runs both when it loads the workspace. A runtime payload
can run under `cargo test`.

The `cargo audit` terminal report does not show the category. For a malicious release it
prints `Solution: No fixed upgrade is available!`. Read the category and the `unaffected`
ranges with the `--json` filter in section 1.

1. Take the malicious code out of the graph in one change.
   - If the advisory lists `unaffected` versions, a legitimate crate had a compromised
     release. Roll back to the newest unaffected version:
     `cargo update -p <crate>@<bad-version> --precise <unaffected-version>`. If the
     `Cargo.toml` requirement excludes that version, lower the requirement first.
   - If it lists no `unaffected` versions, remove the dependency.
   - Confirm that every malicious package left `Cargo.lock`, the injected dependency
     included (for example `proc-macro1`). `grep -n 'name = "<package>"' Cargo.lock` must
     print nothing.
2. Delete the local copies of each malicious package, because the next build can run them
   again. Run `cargo clean` in every workspace that built the crate.

   ```bash
   reg="${CARGO_HOME:-$HOME/.cargo}/registry"
   find "$reg/cache" -name '<crate>-<version>.crate' -delete
   find "$reg/src" -mindepth 2 -maxdepth 2 -type d -name '<crate>-<version>' -exec rm -rf {} +
   cargo clean
   ```

3. Report to the owner which machines and CI runners built, tested, or opened the
   workspace. Recommend that they rotate every credential those machines could read:
   registry tokens, SSH keys, cloud credentials, and wallet keys. Rotation is an external
   action. Do not do it yourself.

## 5. Vet a new or changed dependency

Until step 3 passes, the no-compile rule under the review gate holds. rust-analyzer runs
build scripts and proc macros when `Cargo.toml` changes. Close the IDE, or set
`rust-analyzer.cargo.buildScripts.enable` and `rust-analyzer.procMacro.enable` to `false`.
`cargo metadata`, `cargo fetch`, `cargo tree`, and `cargo deny` do not run crate code.
One exception: a scheduled latest-dependencies job builds unvetted packages by design. Run it
only on a disposable hosted runner with no secrets, read-only permissions, and no cache save,
and vet its lock diff with this gate before you adopt the update (the `cargo-workflows` skill,
when it is installed, has the job).

A payload can hide in runtime code, in a new transitive dependency, or in the build script
of a dependency that a compromised patch release adds. Read
[references/incidents.md](references/incidents.md) when a finding resembles a known attack
or when you explain a rejection. It lists recent crates.io malware cases.

Since 2026-02, crates.io files a RustSec advisory with category `malicious` for every crate
it removes for malware. `cargo audit` and the cargo-deny `advisories` check report these as
vulnerabilities.

Apply this gate to every package name that is new in the `Cargo.lock` diff, transitive
packages included. A `cargo update` is a vetting event too: a patch release that adds a
dependency adds a new package name.

1. **Create the candidate lockfile.** After you edit `Cargo.toml`, resolve once without
   `--locked`. After `cargo update`, the lockfile is already written. Then read the diff:

   ```bash
   cargo metadata --format-version 1 > /dev/null
   git diff -- Cargo.lock
   ```

   Every new `[[package]]` name is a package to vet. Do not use `cargo update -p <crate>`
   for a crate that is not in the old lockfile. Cargo cannot select it yet. Do not run a
   `--locked` check before this step, because the old lockfile does not hold the new graph.
2. **Check the identity.** Compare the name character by character with the intended
   crate. Look for an added word or character (`faster_log` for `fast_log`), a `-rs` or
   `-rust` suffix, or a plural. crates.io treats `-` and `_` as the same name, so a swap
   alone cannot squat. Read the first-publish date, the download count, and the repository
   URL, and check the owners on the crate page:

   ```bash
   curl -s -A '<tool> (<contact>)' https://crates.io/api/v1/crates/<crate> \
     | jq -r '.crate | "\(.created_at) \(.downloads) \(.repository)"'
   ```

   A crate first published less than 7 days ago (a default threshold; set your own) under
   a name close to a popular crate is a red flag. Reject it, or read every file of it in
   step 3.
3. **Read the published source, not the repository.** The `.crate` can differ from the Git
   tree. Run `cargo fetch --locked`, then find the unpacked source of each new package:

   ```bash
   cargo metadata --locked --format-version 1 \
     | jq -r '.packages[] | select(.name == "<crate>" and .version == "<version>") | .manifest_path'
   ```

   Triage every new package. Grep the directory of that `manifest_path` for code that
   reaches outside the process, and read each match:

   ```bash
   grep -rnE 'Command::new|TcpStream|UdpSocket|reqwest|ureq|hyper|env::var|home_dir|include_bytes!|[A-Za-z0-9+/=]{100,}' <dir>
   ```

   List the packages that run code at compile time:

   ```bash
   cargo metadata --locked --format-version 1 | jq -r '.packages[]
     | select(any(.targets[].kind[]; . == "custom-build" or . == "proc-macro"))
     | "\(.name) \(.version)"'
   ```

   Read the build script of each new package in full, with every file it includes. Read
   every file of a package that is a proc macro, has a grep match that its purpose does not
   explain, was first published or released less than 7 days ago, or has few downloads or
   a new owner. Look for network access, `std::process::Command`, environment reads,
   home-directory and key-file paths, embedded blobs, and file writes outside `OUT_DIR`. A
   pure utility crate that opens a socket is a red flag.
4. **Write a caret requirement at the reviewed version** (`name = "1.2.3"`). The committed
   `Cargo.lock` holds the exact version. Do not use an `=1.2.3` requirement as a vetting
   device. It stops `cargo update -p` from taking a security fix. In a published library it
   makes downstream resolution failures more likely: every other requirement on that crate
   must accept exactly that version, and a yank of that version breaks resolution. Use `=`
   only for tightly coupled pairs, such as a crate and its companion proc-macro crate.
5. **Run the policy on the candidate graph.** Run
   `cargo deny --config deny.toml --locked check`. Reject the dependency if it fails.
6. **Justify the dependency.** Check whether the standard library or a crate already in
   the graph does the job. Every new crate widens the attack surface.

For a version bump of a package that is already in the lockfile, check the release date:

```bash
curl -s -A '<tool> (<contact>)' https://crates.io/api/v1/crates/<crate>/<version> | jq -r .version.created_at
```

Read the changes of a release that is less than 7 days old or that adds a build script or a
proc-macro dependency. Run `diff -ru <old-dir> <new-dir>` on the two unpacked directories
under `$CARGO_HOME/registry/src`.

## 6. Supply chain hardening

```bash
cargo fetch --locked
cargo tree --locked -d             # duplicate versions
cargo tree --locked -i <crate>     # why is this crate here
cargo machete                      # unused dependencies
cargo install --locked cargo-vet
cargo vet init                     # once per repository
cargo vet
```

`cargo machete` is a heuristic. It reports false positives for renamed dependencies and
for dependencies that only a macro uses. Confirm each finding before you remove it.

- Commit `Cargo.lock` for every package, libraries included. It does not constrain the
  consumers of a library. The `cargo-workflows` skill owns the lockfile policy.
- Allow only crates.io, unless a git dependency has a written reason and a `rev` pin. A
  branch pin is unpinned code execution. Enforce this with `unknown-registry = "deny"` and
  `required-git-spec = "rev"`.
- Use Cargo 1.96.1 or later in every job that fetches from a third-party registry or over
  SSH git. Older Cargo has CVE-2026-5222 (registry token leak), CVE-2026-5223 (symlink
  override in third-party crate tarballs), and libssh2 CVEs. An MSRV job on an old
  toolchain is the usual exposure.

## 7. Untrusted-input parser hardening

Code that parses a file, a byte buffer, or a network frame from outside the process must
treat the input as adversarial. This applies to archives, protobuf, XML, JSON, SQLite
files, images, and custom binary containers.

- **Cap a length field before you allocate.** Check it against a documented maximum and
  against the remaining input before `Vec::with_capacity` or `vec![0u8; n]`.
- **Use checked arithmetic** (`checked_add`, `checked_mul`, `try_into`) for every offset
  and length. A release build wraps silently.
- **Limit recursion depth** with an explicit counter.
- **Reject non-finite floats** at the parse boundary.
- **Reject path traversal.** Accept only `Component::Normal` UTF-8 components. Create
  entries relative to an open staging-root handle, with no-follow semantics. Do not
  canonicalize a destination that does not exist yet. A library `extract` or `unpack_in`
  helper is not a traversal gate.
- **Disable XML entity expansion.**
- **Authenticate before you mutate.** Verify the digest or MAC of a container before you
  apply any of its content to live state.
- **Reject unknown schema versions and unknown fields.**
- **Return typed errors across an FFI boundary.** Do not panic on malformed input.
- **Keep std `RandomState` for hash-map keys that an outside caller controls**, such as
  HTTP headers, JSON object keys, and archive entry names. `FxHasher`, `FnvHasher`,
  `nohash`, and `BuildHasherDefault<DefaultHasher>` have no random secret, so an attacker
  can build a collision set offline.

Read [references/untrusted-input.md](references/untrusted-input.md) when you write or
review a parser, an archive extractor, a streamed backup format, a parser behind an FFI
boundary, or a hasher choice for a map. It has the format threat table, the checklists,
the hasher rules, and the tests that must exist.

## Related skills

- `cargo-workflows`: lockfile policy, workspaces, and feature flags.
- `rust-panic-safety`: panic containment at an FFI boundary.
- `rust-test-tools`: fuzz and property tests for parsers.
- `rust-hot-path`: the performance side of the hasher choice.
- `uniffi-boundary` and `ffi-error-progress-cancel`: validation and typed errors across a
  binding boundary.
