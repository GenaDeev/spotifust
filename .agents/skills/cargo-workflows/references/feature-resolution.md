# Feature Resolution Pitfalls

Read this file when a crate compiles code or features it did not ask for, when a build passes with
`-p <crate>` but fails with `--workspace`, or when a workspace dependency rejects or ignores
`default-features = false`.

Contents:

- Test supported feature products
- Pitfall: feature unification enables features in `no_std` crates
- Pitfall: an inherited dependency cannot turn default features off

## Test supported feature products

Features are additive and unify for each package in the resolved graph. They are not exclusive
runtime switches. Define the combinations the project supports, then test those combinations
directly:

```bash
cargo test --locked --workspace
cargo test --locked -p <crate> --no-default-features
cargo test --locked -p <crate> --no-default-features --features <feature>
# Run only when the features are additive and the full combination is supported.
cargo test --locked -p <crate> --all-features
# When cargo-hack is installed: one run per feature, plus the default and the
# no-default runs. --exclude-all-features drops the extra all-features run.
cargo hack check --locked -p <crate> --each-feature --exclude-all-features
```

Do not use `--all-features` as a universal quality gate when two backends are intentionally
exclusive. Either make the features additive, or test each supported backend as a separate lane
and reject the invalid combination with a clear `compile_error!`.

`[target.'cfg(feature = "...")'.dependencies]` does not select dependencies by feature. Cargo
resolves features after it selects target dependency tables. Use optional dependencies plus
`[features]`, then put target selection in a real target table.

Reference: [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification),
[platform-specific dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies).

## Pitfall: feature unification enables features in `no_std` crates

When several packages use one dependency, Cargo builds it once with the union of their features.
Resolver 2 and later keep three edges apart: dev-dependencies (unless the command builds tests or
examples), build-dependencies and proc-macros, and target tables for targets not being built.
Normal-dependency features still unify across every package that one command builds.

The package selection therefore changes the build. Measured on Rust 1.98.1: member `b` enabled
feature `extra` on a shared dependency. `cargo check -p app` passed, and
`cargo check --workspace` compiled the same dependency with `extra` and failed.

The hazard: one crate enables `std` or `alloc` on a shared dependency, and a crate designed as
`no_std` silently gains heap allocation or `std`-only code. The same mechanism changes behaviour,
not only compilation: `serde_json`'s `preserve_order` feature changes the key order for every
crate in the build. The `rust-serde` skill has the serde_json details.

Detection:

```bash
# Show which packages activate which features on a shared dependency
cargo tree --locked -e features -i <shared-dep>

# Prove that a no_std crate builds without std: use a target that has no std.
rustup target add thumbv7em-none-eabihf
cargo check --locked -p <no-std-crate> --no-default-features --target thumbv7em-none-eabihf
```

A host-target `cargo check` cannot prove `no_std`, because `std` is always present on the host.

Fix: turn default features off where the dependency is declared. For an inherited dependency,
that is the `[workspace.dependencies]` entry; the next section shows why. Let each member add the
features it needs. Move a dependency that only tests need to `[dev-dependencies]`. Check the
`no_std` crate on its own with `-p`, and check the workspace build too.

Reference: [Cargo feature resolver version 2](https://doc.rust-lang.org/cargo/reference/features.html#feature-resolver-version-2).

## Pitfall: an inherited dependency cannot turn default features off

An inherited dependency (`foo = { workspace = true }`) accepts only `optional` and `features`
beside `workspace`. `features` adds to the features of the `[workspace.dependencies]` entry. When
the workspace entry keeps default features on, a member cannot turn them off:

| Member edition | Result on Cargo 1.98.1 (measured) |
|----------------|-----------------------------------|
| 2024 | Hard error: `` `default-features = false` cannot override workspace's `default-features` `` |
| 2021 and older | Warning: `` `default-features` is ignored for foo, since `default-features` was not specified for `workspace.dependencies.foo` ``. The default features stay on. |

The 2021 case is the dangerous one. A target table such as
`[target.'cfg(target_arch = "wasm32")'.dependencies] foo = { workspace = true, default-features = false }`
still builds the default features for that target. The link or compile step then fails on code
the target cannot support, and the warning scrolls past.

Fix: put `default-features = false` in the `[workspace.dependencies]` entry. Let each member, or
each target table, add the features it needs with `features = [...]`.

Detection:

```bash
cargo tree --locked --target <triple> -e features -i <dep>
```

Compare the output with the host resolution. A default feature that appears on a target where
the member disabled it confirms the problem.

Cargo 1.99, due 2026-10-01 and not stable as of Rust 1.98.1, lets an edition-2024 member override
an inherited `default-features`. Do not rely on it until the pinned toolchain is 1.99 or later and
the member is on edition 2024.

Reference: [inheriting a dependency from a workspace](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#inheriting-a-dependency-from-a-workspace);
Cargo issue #11779, closed as expected behaviour.
