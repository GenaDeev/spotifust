# Project State Machine

## Current Focus

- [ ] (Phase 3) Implement Librespot Authentication (Discovery/Zeroconf or Credentials)

## Development Backlog

### Phase 1: Bootstrapping & Core Architecture

- [x] Configure Cargo.toml with feature flags for Iced (wgpu backend), RSpotify, and Librespot
- [x] Define central `AppError` enum (thiserror) with per-subsystem variants
- [x] Set up base Model-View-Update loop in `src/app.rs`
- [x] Set up full GitHub Actions CI/CD infrastructure, Issue templates, and documentation

### Phase 2: Custom Canvas Layout Engine

- [x] Implement bounding-box tracking struct for responsive cards
- [x] Handle PointerPressed / Moved / Released inside `canvas::Program::update`
- [x] Wire `canvas::Cache` invalidation to interaction messages only
- [x] Wire synthetic audio engine test pipeline (`rodio` backend)

### Phase 3: Librespot Audio & Session Pipeline (Next up)

- [x] **Librespot Authentication**: Setup `librespot::core::session::Session` and login using credentials or Zeroconf.
- [ ] **Librespot Audio Backend**: Implement a custom `librespot::playback::audio_backend::Sink` that captures decoded PCM frames from Spotify.
- [ ] **Audio Bridge**: Route the decoded PCM frames from the Librespot custom Sink through our bounded `mpsc` channel directly to our `rodio` playback thread.
- [ ] **Playback Control**: Wire UI commands (Play, Pause, Skip, Seek) through `iced::Subscription` down to the `librespot` player instance.
- [ ] **Track Metadata**: Extract current track information (Title, Artist, Duration, Position) from Librespot events and stream them to the UI state.

### Phase 4: RSpotify Web API, Auth & Premium UI

- [ ] **OAuth PKCE Flow**: Implement `rspotify` Authorization Code Flow with PKCE using a custom protocol callback (`spotifust://callback`).
- [ ] **Keychain Storage**: Securely store the OAuth refresh token via the OS credential store (`keyring` crate).
- [ ] **Main Layout UI**: Build the primary layout grid using standard `iced` widgets (Sidebar, Main Content Area, Bottom Playback Bar) wrapping our Canvas cards.
- [ ] **Playback Bar UI**: Implement a dynamic Bottom Playback Bar with Album Art, Title, Artist, Play/Pause/Skip buttons, and a draggable Seek Bar.
- [ ] **Library & Search UI**: Fetch and display the user's saved playlists, albums, and top tracks using the RSpotify client.
- [ ] **Album Art Caching**: Fetch album cover images asynchronously and cache them to disk in `src/api/cache.rs` to avoid redundant network calls.

### Phase 5: Polish & Optimization

- [ ] **Memory Profiling**: Verify the application stays under the strict 25MB RAM baseline.
- [ ] **Micro-animations**: Add smooth hover transitions and interactions to UI elements.
- [ ] **System Tray Integration**: Add a system tray icon with basic playback controls (if supported by OS).

## Architectural Debt

- [ ] Ensure all `librespot` and `rspotify` errors are properly wrapped in `AppError` variants before reaching the `iced::Message` enum.

## Blocked / Needs Human Decision

- [ ] (None currently - waiting for tomorrow's session)
