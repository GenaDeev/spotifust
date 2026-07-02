<div align="center">

<img src=".github/assets/banner.png" alt="Spotifust banner" width="100%" />

# 🎧 Spotifust

**A multi-platform, ultra-lightweight Spotify client built entirely from scratch in Rust.**

[![CI](https://img.shields.io/github/actions/workflow/status/GenaDeev/spotifust/ci.yml?branch=main&label=CI)](https://github.com/GenaDeev/spotifust/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/GenaDeev/spotifust)](https://github.com/GenaDeev/spotifust/releases)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE)

[![Rust](https://img.shields.io/badge/Rust-1.78%2B-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MSRV](https://img.shields.io/badge/MSRV-1.78-blue)](https://www.rust-lang.org/)
[![iced](https://img.shields.io/badge/GUI-iced%200.12-6574CD?logo=rust&logoColor=white)](https://github.com/iced-rs/iced)
[![wgpu](https://img.shields.io/badge/Renderer-wgpu-orange)](https://wgpu.rs/)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-informational)]()

[![Last Commit](https://img.shields.io/github/last-commit/GenaDeev/spotifust)](https://github.com/GenaDeev/spotifust/commits/main)
[![Repo Size](https://img.shields.io/github/repo-size/GenaDeev/spotifust)](https://github.com/GenaDeev/spotifust)
[![Issues](https://img.shields.io/github/issues/GenaDeev/spotifust)](https://github.com/GenaDeev/spotifust/issues)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

> ⚡ Single process
>
> 🦀 100% Rust
>
> 🎨 GPU accelerated (wgpu)
>
> 🎵 Embedded librespot
>
> 🖥️ Windows • macOS • Linux
>
> 📦 No Electron • No Chromium • No Node.js

</div>

---

## 📌 What is this?

Spotifust started from a simple but slightly stubborn idea: **why does a music player need to load an entire browser inside it?** This project ditches heavy web engines (Electron/Chromium) and native OS wrappers (WinUI 3/WinRT) to deliver a single-process application with hardware-accelerated graphics and embedded audio streaming, straight from Rust.

No Node.js running behind the scenes, no full Chromium instance rendering four buttons. One binary, one process, and the GPU doing what it does best.

---

## 🛠️ Tech Stack

| Component | Technology / Crate | Description |
| :--- | :--- | :--- |
| **GUI Framework** | `iced` (v0.12+) | Cross-platform GUI framework based on the Elm Architecture, focused on type-safety. |
| **Graphics Engine** | `wgpu` (via `iced_wgpu`) | Hardware-accelerated renderer leveraging Vulkan, DirectX 12, and Metal. |
| **UI Layout Core** | `iced::widget::canvas` | Custom 2D layout engine for draggable, resizable fluid cards. |
| **Spotify Web API** | `rspotify` | Asynchronous Spotify Web API wrapper for search, playlists, and metadata. |
| **Audio Streaming** | `librespot-core` / `protocol` | Embedded core engine for session management, DRM decryption, and raw chunk fetching. |
| **Audio Playback** | `rodio` or `cpal` | Multi-platform low-level audio delivery to system sound drivers. |
| **Async Runtime** | `tokio` | Multi-threaded asynchronous event loop for I/O bound operations. |

---

## 🏗️ Architectural Blueprint

Unlike traditional applications, Spotifust does not run separate sidecar processes. The entire ecosystem lives inside a single monolithic Rust binary:

1. **The Elm Engine (Model-View-Update):** `iced` drives the state. The `Model` holds the application data, the `View` renders the canvas primitives, and the `Update` processes incoming asynchronous events smoothly.
2. **The Canvas Layout System:** Instead of standard flexbox-style UI containers, the main dashboard uses a low-level `Canvas` widget. Inside, a custom spatial data structure tracks bounding boxes ($X, Y, W, H$) for each modular card, handling hardware input events directly for dragging and resizing.
3. **In-Process Audio Core:** `librespot` is compiled directly as an internal module. It establishes direct TCP/TLS connections with Spotify's infrastructure, performs AES-128 DRM decryption internally, and feeds decoded PCM arrays directly into the system's hardware audio buffers via a lock-free ring channel.

---

## 🚀 Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.78 or later (2021 edition)
- A **Spotify Premium** account (required: Spotify's streaming API doesn't allow full playback on free accounts)
- Up-to-date GPU drivers with Vulkan, DirectX 12, or Metal support depending on your OS

### Installation

Clone the repo and build in release mode (Rust's debug mode with wgpu performs noticeably worse, so go straight to release if you want to actually test the player):

```bash
git clone https://github.com/GenaDee/spotifust.git
cd spotifust
cargo build --release
```

### Running

```bash
cargo run --release
```

On first launch, it'll ask for your Spotify Premium credentials to initialize the `librespot` session. Once authenticated, the session gets cached locally for future launches.

### Environment Variables (optional)

If you're registering your own app in the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) to use `rspotify` with your own API credentials:

```bash
export SPOTIFY_CLIENT_ID="your_client_id"
export SPOTIFY_CLIENT_SECRET="your_client_secret"
```

---

## 🗺️ Roadmap

Read [TODO.md](./TODO.md) for the current roadmap and backlog.

---

## 🤝 Contributing

PRs are welcome. If you're planning to touch the audio core or the canvas engine, open an issue first to discuss the approach before sending code — those are the most delicate parts of the project.

## 📄 License

This project is licensed under [GPLv3](./LICENSE).
