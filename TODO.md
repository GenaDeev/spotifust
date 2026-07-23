# Project State Machine

## Current Focus

- [ ] Implement artist detail view: fetch `/artists/{id}` with top tracks and discography

## Development Backlog

### Phase 1: Bootstrapping & Core Architecture

- [x] Configure Cargo.toml with feature flags for Iced (tiny-skia backend), RSpotify, and Librespot
- [x] Define central `AppError` enum (thiserror) with per-subsystem variants
- [x] Set up base Model-View-Update loop in `src/app.rs`
- [x] Set up full GitHub Actions CI/CD infrastructure, Issue templates, and documentation
- [x] Verify all `librespot` and `rspotify` raw error types are wrapped in `AppError` before reaching `Message` variants
- [x] Audit and eliminate any remaining `.unwrap()` / `.expect()` calls outside `main()` bootstrap
- [x] Reduce RAM baseline from ~45 MB down to the target < 25 MB ceiling

### Phase 2: Spotify Resizable Panel Layout Engine

- [x] Implement 3-column layout structure (Left Sidebar library, Main content, Right panel)
- [x] Add interactive drag handles with `ResizingHorizontally` mouse cursor interaction
- [x] Handle global pointer move/up events for robust dragging/resizing
- [x] Implement right dynamic slot panel showing Now Playing or Queue based on playback bar triggers
- [x] Implement left library sidebar collapse to icon-only compact layout below width threshold
- [x] Persist layout panel widths to disk

### Phase 3: Librespot Audio & Session Pipeline

- [x] Implement `librespot::core::session::Session` setup and credential-based login
- [x] Implement a custom `librespot` audio `Sink` that captures decoded PCM frames
- [x] Route PCM frames from the custom Sink through a bounded `mpsc` channel to a `rodio` playback thread
- [x] Wire a synthetic sine-wave test pipeline to validate the `rodio` backend end-to-end
- [x] Wire UI Play command to call `player.load()` on the active `librespot` player instance
- [x] Wire UI Pause / Resume commands to the librespot player
- [x] Wire UI Skip Next / Skip Previous commands to the librespot player
- [x] Implement Seek: accept a `f32` position ratio from the seek bar and call `player.seek(ms)`
- [x] Extract current track metadata (title, artist, album, duration) from `PlayerEvent` and emit them as `Message::TrackChanged`
- [x] Stream playback position (elapsed ms) from the audio task to the UI via the mpsc channel
- [x] Implement end-of-track detection via `PlayerEvent::EndOfTrack` and auto-advance to next track
- [x] Validate that the mpsc channel remains bounded under sustained high-throughput decoding
- [x] Wire volume control: slider value in UI → `rodio::Sink::set_volume()` (full 0.0–1.0 range, not binary)
- [x] Fix seek bar so it travels the full 0–100% range and reflects real playback position
- [x] Handle `librespot` session expiry and reconnection without crashing

### Phase 4: RSpotify Web API & Auth

- [x] Implement PKCE Authorization Code Flow with `rspotify`
- [x] Register `spotifust://callback` custom protocol handler for the OAuth redirect
- [x] Verify the refresh token is stored exclusively via the OS keychain (`keyring` crate), never as plaintext
- [x] Implement token refresh on expiry: detect 401 responses and silently re-authenticate
- [x] Fetch the authenticated user's profile (`/me`) and display name and avatar in the sidebar
- [x] Fetch the user's full playlist library (`/me/playlists`, paginated) and stream items into the sidebar list
- [x] Fetch playlist track listings on demand when a playlist is selected
- [x] Fetch the user's saved albums and expose them in a dedicated Albums view
- [x] Fetch the user's top tracks and expose them in a Home/For You view
- [x] Implement search: send queries to `/search` and display track, album, and artist results
- [x] Implement album detail view: fetch `/albums/{id}` and list its tracks
- [ ] Implement artist detail view: fetch `/artists/{id}` with top tracks and discography
- [ ] Fetch currently playing track via `/me/player/currently-playing` on startup and sync UI state
- [ ] Implement album art fetching: download cover images asynchronously and cache to disk in `src/api/cache.rs`
- [ ] Implement a metadata cache layer in `src/api/cache.rs` to avoid redundant API calls (TTL-based)
- [ ] Implement rate-limit handling: respect `Retry-After` headers from the Spotify API

### Phase 5: UI Design System & Component Polish

- [ ] Define a unified design token system (color palette, spacing scale, typography scale) in a central `theme.rs`
- [ ] Replace all ad-hoc hardcoded color literals and magic numbers with design tokens
- [ ] Implement smooth hover transitions on sidebar items, buttons, and playback controls
- [ ] Implement animated loading skeletons for album art and track list placeholders while data is fetching
- [ ] Add waveform or animated equalizer bars to the Now Playing area during active playback
- [ ] Implement smooth progress bar animation that interpolates position between tick updates
- [ ] Add context menus (right-click) on tracks and playlist items (Add to queue, Go to album, etc.)
- [ ] Implement a proper volume slider that covers the full 0–100% range with a mute toggle
- [ ] Add keyboard shortcuts for Play/Pause (Space), Skip (→/←), Volume (↑/↓)
- [ ] Implement a mini-player / compact mode for when the window is resized to small dimensions
- [ ] Implement drag-and-drop track reordering within a playlist queue view
- [ ] Add toast / snackbar notifications for user-facing errors and confirmations
- [ ] Audit and refine all font sizes, weights, and line heights for visual consistency
- [ ] Ensure the entire UI is navigable via keyboard (tab order, focus rings)

### Phase 6: Queue, Playback State & Shuffle

- [ ] Implement an internal play queue data structure in the `Model`
- [ ] Display the current queue in a slide-out panel
- [ ] Implement Shuffle mode: randomise queue order and persist the shuffle seed
- [ ] Implement Repeat modes: No Repeat, Repeat Queue, Repeat One
- [ ] Implement "Add to queue" action from track context menus
- [ ] Sync queue state back to Spotify Connect so other devices see the same queue
- [ ] Implement Crossfade between tracks (configurable duration)

### Phase 7: System Integration & Distribution

- [x] Add application window and taskbar/dock icon support for Windows, macOS, and Linux distros
- [ ] Add a system tray icon with Play/Pause, Skip, and Quit actions
- [ ] Register global media key bindings (MPRIS on Linux, MediaSession on Windows/macOS)
- [ ] Implement MPRIS2 D-Bus interface on Linux for desktop environment integration
- [ ] Package the binary as a `.deb` and `.rpm` for Linux
- [x] Package the binary as a `.dmg` / `.app` bundle for macOS
- [x] Package the binary as an `.msi` installer for Windows
- [ ] Integrate auto-update check: compare current version against GitHub Releases on startup
- [ ] Write end-to-end integration tests for the auth flow and audio pipeline

### Phase 8: Performance & Hardening

- [ ] Run a full memory profile and verify the application stays under 25 MB baseline at idle
- [ ] Profile and eliminate any hot-path allocations in the canvas render loop and audio callback
- [ ] Replace any `.clone()` / `.to_string()` in hot paths with borrows (`&str`, `&[u8]`) where applicable
- [ ] Run `cargo clippy --all-targets -- -D warnings` clean and resolve all lints
- [ ] Run `cargo deny check` and ensure no disallowed licenses or duplicated dependencies
- [ ] Set up memory-leak detection in CI (Valgrind or similar) for the audio pipeline
- [ ] Add structured logging (`tracing` crate) with configurable verbosity levels
- [ ] Implement graceful shutdown: flush audio buffers and close the librespot session cleanly on exit

## Architectural Debt

- [ ] The Canvas Layout Engine was listed as completed but was never implemented; all Phase 2 items are open
- [x] Volume slider and seek bar have only two discrete positions and do not cover their full range
- [ ] All playlist/album/track data shown in the UI is hardcoded mock data, not from the Spotify API
- [ ] RAM usage is currently ~45 MB, nearly double the 25 MB target
- [x] The librespot session may be storing or caching credentials insecurely; needs a full audit (Audit complete: keyring used, no plaintext or librespot cache on disk)
- [x] No structured error surfacing to the user: errors silently fail or panic instead of showing in the UI

## Blocked / Needs Human Decision

- [ ] Decide whether to support Spotify Connect (remote control from phone/web) in scope for v1.0
- [ ] Decide on the crossfade implementation approach before Phase 6 begins
