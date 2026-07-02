# Architecture

Spotifust is a lightweight Spotify client written in Rust.

## Model-View-Update (MVU)
The UI is built using the `iced` GUI library, which follows the Elm architecture (Model-View-Update). State mutations only happen by passing `Message` variants to `update()`.

## Canvas Layout Engine
The main UI is rendered using `iced::widget::canvas`. To ensure `<25MB` RAM usage, the layout is highly optimized:
* Static layers are cached heavily using `canvas::Cache`.
* Dynamic interactions (hover, drag boundaries) bypass the cache to ensure 60FPS fluid animations without invalidating the static geometry.

## Audio and Playback
Audio processing occurs on a separate background thread via `tokio::spawn`. We bridge `librespot` decoders directly to `rodio` or `cpal` sinks via bounded `mpsc` channels to avoid intermediate buffers and keep memory footprint low.

## OAuth and Networking
We use PKCE with `rspotify`. We intercept the OAuth callback using a fixed-port loopback `TcpListener` that matches the registered redirect URI in the Spotify Developer dashboard.

## Caching
Metadata and images are cached strictly in memory or standard disk caches. Secrets are stored securely in the OS keychain via the `keyring` crate.

## State Management
Global state is fully deterministic and resides entirely in `src/app.rs`.
