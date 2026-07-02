# Release Process

This document outlines the release process for Spotifust.

## Semantic Versioning
Spotifust strictly follows Semantic Versioning (SemVer):
* `0.x.y`: Development phase (API/UI changes are expected).
* `1.0.0`: First stable release.
* `MAJOR`: Breaking changes.
* `MINOR`: New features added in a backwards-compatible manner.
* `PATCH`: Bug fixes.

## Release Cadence
Releases are generally grouped by milestone. Security fixes trigger an immediate patch release.

## Version Numbering
Before tagging, update the `version` field in `Cargo.toml`.

## GitHub Releases
We use automated GitHub Actions to build binaries for Windows, macOS, and Linux.
When a new tag `vX.Y.Z` is pushed, a draft release is generated.

## Tagging
Create a tag and push:
```sh
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

## Changelog Generation
We use Release Drafter. It categorizes merged pull requests automatically based on their labels (`enhancement`, `bug`, etc.) to generate release notes.

## Prereleases
Any tag containing `alpha`, `beta`, or `rc` (e.g. `v1.0.0-beta.1`) will automatically be marked as a "pre-release" in GitHub.
