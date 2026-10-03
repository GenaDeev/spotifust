# Advisory Reports and Supply-Chain Incidents

## A `cargo audit` finding

cargo-audit 0.22.2 prints one block per finding. This is the output for `smallvec` 1.6.0:

```text
Crate:     smallvec
Version:   1.6.0
Title:     Buffer overflow in SmallVec::insert_many
Date:      2021-01-08
ID:        RUSTSEC-2021-0003
URL:       https://rustsec.org/advisories/RUSTSEC-2021-0003
Severity:  9.8 (critical)
Solution:  Upgrade to >=0.6.14, <1.0.0 OR >=1.6.1

error: 1 vulnerability found!
```

## Recent crates.io malware

These incidents show where a payload hides:

- 2025-09, `faster_log` and `async_println`: copies of legitimate logging crates under
  similar names. The payload ran at runtime in a log-packing code path, not at build time.
  It searched files for Ethereum and Solana private keys.
- 2025-12, `finch-rust`: a copy of `finch`. The payload was in its new dependency
  `sha-rust`.
- 2026-08, `arrayref` 0.3.10, `internment` 0.8.7, `append-only-vec` 0.1.9: the owner's
  credentials were likely compromised. The new patch releases added a dependency on
  `proc-macro1`, whose build script downloaded a payload. The releases were live for less
  than two hours.

The RustSec feed of new advisories, `malicious` ones included, is
<https://rustsec.org/feed.xml>.
