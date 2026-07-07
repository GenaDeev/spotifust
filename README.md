<div align="center">

<img src=".github/assets/banner.png" alt="Spotifust banner" width="100%" />

# 🎧 Spotifust

**A multi-platform, ultra-lightweight Spotify client built entirely from scratch in Rust.**

[![CI](https://img.shields.io/github/actions/workflow/status/GenaDeev/spotifust/ci.yml?branch=main&label=CI&style=flat-square)](https://github.com/GenaDeev/spotifust/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/GenaDeev/spotifust?style=flat-square)](https://github.com/GenaDeev/spotifust/releases)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg?style=flat-square)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-DEA584?logo=rust&logoColor=white&style=flat-square)](https://www.rust-lang.org/)
[![iced](https://img.shields.io/badge/GUI-iced%200.14-6574CD?logo=rust&logoColor=white&style=flat-square)](https://github.com/iced-rs/iced)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-informational?style=flat-square)

[![Last Commit](https://img.shields.io/github/last-commit/GenaDeev/spotifust?style=flat-square)](https://github.com/GenaDeev/spotifust/commits/main)
[![Repo Size](https://img.shields.io/github/repo-size/GenaDeev/spotifust?style=flat-square)](https://github.com/GenaDeev/spotifust)
[![Issues](https://img.shields.io/github/issues/GenaDeev/spotifust?style=flat-square)](https://github.com/GenaDeev/spotifust/issues)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](CONTRIBUTING.md)
[![Lines of Code](https://img.shields.io/endpoint?url=https%3A%2F%2Ftokei.kojix2.net%2Fbadge%2Fgithub%2FGenaDeev%2Fspotifust%2Flines&style=flat-square)](https://tokei.kojix2.net/github/kojix2/tokei-api)

> ⚡ Single process • 🦀 100% Rust • 🎵 Embedded librespot • 📦 No Electron • No Chromium • No Node.js

</div>

---

## 📌 What is this?

Spotifust started from a simple but slightly stubborn idea: **why does a music player need to load an entire browser inside it?** This project ditches heavy web engines (Electron/Chromium) and native OS wrappers (WinUI 3/WinRT) to deliver a single-process application with hardware-accelerated graphics and embedded audio streaming, straight from Rust.

No Node.js running behind the scenes, no full Chromium instance rendering four buttons. One binary, one process, and the GPU doing what it does best.

---

## ✨ Features

- 🎵 **Native Spotify playback** — Stream directly via embedded librespot, no browser engine
- 🖥️ **Cross-platform** — Windows (.msi), macOS (.dmg), and Linux (.tar.gz)
- ⚡ **Ultra-lightweight** — Target baseline under 25 MB RAM
- 🎨 **GPU-accelerated UI** — Powered by iced with tiny-skia rendering
- 🔐 **Secure auth** — PKCE OAuth flow, credentials stored in your OS keychain
- 🧩 **Modular architecture** — Clean MVU (Model-View-Update) following the Elm pattern
- 📦 **Zero runtime dependencies** — No Node.js, no JVM, no Python, no bundled browser

---

## 🛠️ Tech Stack

| Component | Technology | Description |
| :--- | :--- | :--- |
| **GUI Framework** | [`iced`](https://github.com/iced-rs/iced) v0.14 | Cross-platform GUI based on the Elm Architecture, focused on type-safety |
| **Renderer** | `tiny-skia` (via iced) | Software 2D rendering with optional GPU acceleration |
| **UI Layout** | `iced::widget::canvas` | Custom 2D canvas for draggable, resizable fluid cards |
| **Spotify Web API** | [`rspotify`](https://github.com/ramsayleung/rspotify) v0.16 | Async Spotify Web API wrapper for search, playlists, metadata |
| **Audio Streaming** | [`librespot`](https://github.com/librespot-org/librespot) v0.8 | Embedded engine for session management, DRM decryption, chunk fetching |
| **Audio Playback** | [`rodio`](https://github.com/RustAudio/rodio) v0.21 | Cross-platform audio output to system sound drivers |
| **Async Runtime** | [`tokio`](https://github.com/tokio-rs/tokio) v1.52 | Multi-threaded async event loop for I/O-bound operations |
| **Error Handling** | [`thiserror`](https://github.com/dtolnay/thiserror) v2 | Derive macro for central `AppError` enum with per-subsystem variants |
| **Credential Storage** | [`keyring`](https://github.com/hwchen/keyring-rs) v4 | OS-level secure credential store (Credential Manager / Keychain / Secret Service) |

---

## 🏗️ Architecture

Unlike traditional applications, Spotifust does not run separate sidecar processes. The entire ecosystem lives inside a single monolithic Rust binary:

```text
┌─────────────────────────────────────────────────────────┐
│                    Single Process                       │
│                                                         │
│  ┌─────────────┐   Message    ┌───────────────────┐     │
│  │  iced App   │◄────────────►│   Model (State)   │     │
│  │  View/Update│              └───────────────────┘     │
│  └──────┬──────┘                        ▲               │
│         │ Canvas                        │ mpsc           │
│  ┌──────▼──────┐              ┌─────────┴─────────┐     │
│  │  Card Layout│              │  tokio::spawn      │     │
│  │  Engine     │              │  ┌───────────────┐ │     │
│  └─────────────┘              │  │   librespot   │ │     │
│                               │  │   session     │ │     │
│                               │  └───────┬───────┘ │     │
│                               │          │ PCM     │     │
│                               │  ┌───────▼───────┐ │     │
│                               │  │  rodio sink   │ │     │
│                               │  └───────────────┘ │     │
│                               └────────────────────┘     │
└─────────────────────────────────────────────────────────┘
```

1. **The Elm Engine (Model-View-Update):** `iced` drives the state. The `Model` holds the application data, the `View` renders the canvas primitives, and the `Update` processes incoming asynchronous events smoothly.
2. **The Canvas Layout System:** Instead of standard flexbox-style UI containers, the main dashboard uses a low-level `Canvas` widget with a custom spatial data structure tracking bounding boxes for each modular card, handling hardware input events directly for dragging and resizing.
3. **In-Process Audio Core:** `librespot` is compiled directly as an internal module. It establishes direct TCP/TLS connections with Spotify's infrastructure, performs AES-128 DRM decryption internally, and feeds decoded PCM arrays directly into the system's hardware audio buffers via a bounded channel.

---

## 📂 Project Structure

```text
spotifust/
├── src/
│   ├── main.rs              # Entry point & bootstrap
│   ├── app.rs               # iced Application (MVU loop)
│   ├── error.rs             # Central AppError enum (thiserror)
│   ├── api/
│   │   ├── mod.rs
│   │   └── auth.rs          # PKCE OAuth flow & token management
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── engine.rs        # Playback control & track queue
│   │   ├── session.rs       # librespot session management
│   │   └── sink.rs          # rodio audio output sink
│   └── ui/
│       ├── mod.rs
│       ├── icons.rs          # SVG icon definitions
│       ├── login.rs          # Login screen view
│       ├── main_layout.rs    # Main dashboard canvas layout
│       └── theme.rs          # Color palette & styling
├── assets/                   # App icons & resources
├── installer/                # WiX MSI installer sources
├── docs/                     # Additional documentation
├── scripts/                  # Developer & CI scripts
│   ├── build.sh             # Unix packaging script
│   ├── build.ps1            # Windows packaging script
│   ├── test.sh              # Unix test runner
│   └── test.ps1             # Windows test runner
├── install.sh                # End-user Linux installation script
├── Cargo.toml
└── TODO.md                   # Development backlog & roadmap
```

---

## 🚀 Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) **1.85** or later (2024 edition)
- A **Spotify Premium** account (required: Spotify's streaming API doesn't allow full playback on free accounts)

### Build from source

```bash
git clone https://github.com/GenaDeev/spotifust.git
cd spotifust
cargo build --release
```

> [!TIP]
> Always build in `--release` mode. Debug builds with GPU rendering perform significantly worse and don't represent the real experience.

### Run

```bash
cargo run --release
```

On first launch, it'll ask for your Spotify Premium credentials to initialize the `librespot` session. Once authenticated, the session gets cached locally for future launches.

### Environment variables (optional)

If you're registering your own app in the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) to use `rspotify` with your own API credentials:

```bash
export SPOTIFY_CLIENT_ID="your_client_id"
```

> [!NOTE]
> Spotifust uses the Authorization Code Flow with PKCE — no client secret is required for the desktop app's own auth.

---

## 📦 Downloads

Pre-built binaries are available on the [Releases](https://github.com/GenaDeev/spotifust/releases) page with the following naming convention:

| Platform | File | Architecture |
| :--- | :--- | :--- |
| 🪟 Windows | `spotifust-windows-x86_64-{version}.msi` | x86_64 |
| 🍎 macOS | `spotifust-macos-aarch64-{version}.dmg` | Apple Silicon |
| 🍎 macOS | `spotifust-macos-x86_64-{version}.dmg` | Intel |
| 🐧 Linux | `spotifust-linux-x86_64-{version}.tar.gz` | x86_64 |
| 🐧 Linux | `spotifust-linux-x86_64-{version}.deb` | Debian/Ubuntu |

### Linux Installation

Download the `.tar.gz`, extract it, and run the included `./install.sh` script to install the app and register the `spotifust://` protocol handler automatically. Alternatively, install the `.deb` package directly on Debian-based systems.

### Building installers locally

- **Windows:** Run `.\scripts\build.ps1` in PowerShell. Requires the [WiX v4 Toolset](https://wixtoolset.org/) installed via `dotnet tool install --global wix`.
- **macOS:** Run `./scripts/build.sh`. Creates an `.app` bundle and packages it into a `.dmg`.
- **Linux:** Run `./scripts/build.sh`. Compresses the release binary into a `.tar.gz` archive.

---

## 🧪 Testing & CI

To ensure your code meets the quality standards of the project, we provide unified test scripts. They format the code, run clippy, run tests, and optionally perform dependency audits and typo checks.

```bash
# Unix
./scripts/test.sh

# Windows (PowerShell)
.\scripts\test.ps1
```

The CI pipeline runs automatically on every push and PR:

| Workflow | Trigger | Purpose |
| :--- | :--- | :--- |
| **CI** | Push / PR | Build, clippy, tests |
| **Release** | Tag `v*` | Build artifacts for all platforms & publish GitHub release |
| **CodeQL** | Push / PR / Schedule | Security & code quality analysis |
| **Cargo Audit** | Push / Schedule | Dependency vulnerability scanning |
| **Cargo Deny** | Push / PR | License & advisory compliance |
| **Typos** | Push / PR | Spell check across the codebase |
| **Link Check** | Push / PR | Verify all URLs in docs are alive |

---

## 🗺️ Roadmap

Read [TODO.md](./TODO.md) for the current development backlog and roadmap.

---

## 🤝 Contributing

PRs are welcome! If you're planning to touch the audio core or the canvas engine, open an issue first to discuss the approach before sending code — those are the most delicate parts of the project.

Read [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

### Contributors

<a href="https://github.com/GenaDeev/spotifust/graphs/contributors">
  <img alt="Spotifust contributor panel" src="https://contrib.rocks/image?repo=GenaDeev/spotifust" />
</a>

---

## 📄 License

This project is licensed under the [GNU General Public License v3.0](./LICENSE).

---

## 📊 Project Analytics

<div align="center">

### Repobeats

<!-- TODO: Generate the real embed URL at https://repobeats.axiom.co for GenaDeev/spotifust -->
![Repobeats analytics](https://repobeats.axiom.co/api/embed/5732d66e101f2fda36c9bb8aa0d2954cc3b5cd2e.svg "Repobeats analytics image")

### Activity

![Activity Graph](https://github-readme-activity-graph.vercel.app/graph?username=GenaDeev&repo=spotifust&theme=xcode)

</div>
