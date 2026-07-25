# Project State Machine

## Current Focus

- [ ] Architectural Debt Resolution & App Speed Optimization

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
- [x] Fix app crash during track playback (`src/audio/sink.rs:35:14: Cannot block the current thread from within a runtime` & `Invalid Spotify URI ''`)
- [x] Refine audio pipeline for 320kbps high-quality bitrate, synchronized rodio pause/resume and instant volume binding

### Phase 4: RSpotify Web API, Auth & Aggressive Caching

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
- [x] Implement artist detail view: fetch `/artists/{id}` with top tracks and discography
- [x] Fetch currently playing track via `/me/player/currently-playing` on startup and sync UI state
- [x] Implement album art fetching: download cover images asynchronously and cache to disk in `src/api/cache.rs`
- [x] Implement a metadata cache layer in `src/api/cache.rs` to avoid redundant API calls (TTL-based)
- [x] Implement rate-limit handling: respect `Retry-After` headers from the Spotify API
- [x] Display large cover art in playlist and album detail header views
- [x] Audit and remove all remaining mock data across all UI views and components, fetching 100% live Spotify API data
- [x] Optimize long playlist loading with incremental chunking/streaming or virtualized pagination to avoid UI lag
- [x] Validate existing token/session before rendering initial screen to eliminate temporary login flicker
- [x] Achieve near-instant API data loading through aggressive metadata and disk image caching (TTL-based, local disk cache for instant startup render)
- [ ] Add visual "Local Track" badge for unplayable local tracks in playlists owned by playlist creators

### Phase 5: UI Design System, Component Polish & Settings Page

- [x] Define a unified design token system (color palette, spacing scale, typography scale) in a central `theme.rs`
- [ ] Replace all ad-hoc hardcoded color literals and magic numbers with design tokens
- [ ] Implement smooth hover transitions on sidebar items, buttons, and playback controls
- [x] Implement animated loading skeletons for album art, playlist headers, and track list placeholders while initial Spotify API data is fetching (zero mock/temp data, instant Spotify data render)
- [x] Remove "Explore Premium" / "Explorar Premium" button from sidebar and navigation
- [x] Add waveform or animated equalizer bars to the Now Playing area during active playback
- [ ] Implement smooth progress bar animation that interpolates position between tick updates
- [x] Add context menus (right-click) on tracks and playlist items (Add to queue, Go to album, etc.)
- [x] Implement a proper volume slider that covers the full 0–100% range with a mute toggle
- [x] Add keyboard shortcuts for Play/Pause (Space), Skip (→/←), Volume (↑/↓)
- [ ] Implement a mini-player / compact mode for when the window is resized to small dimensions
- [ ] Implement drag-and-drop track reordering within a playlist queue view
- [x] Add toast / snackbar notifications for user-facing errors and confirmations
- [x] Audit and refine all font sizes, weights, and line heights for visual consistency
- [ ] Ensure the entire UI is navigable via keyboard (tab order, focus rings)
- [ ] SETTINGS PAGE: Build a comprehensive Settings page with a full suite of configurable app, audio, quality, local path, crossfade, theme, and Spotify Connect settings
- [ ] LYRICS: Implement fully functional synchronized Lyrics view with multi-provider API integration

### Phase 6: Queue, Playback State, Shuffle & Advanced Audio

- [ ] Implement an internal play queue data structure in the `Model`
- [ ] Display the current queue in a slide-out panel
- [ ] Implement Shuffle mode: randomise queue order and persist the shuffle seed
- [ ] Implement Repeat modes: No Repeat, Repeat Queue, Repeat One
- [ ] Implement "Add to queue" action from track context menus
- [ ] Spotify Connect: Full bi-directional Spotify Connect integration for remote control and device sync
- [ ] Crossfade: Smooth audio crossfade between tracks (configurable duration in Settings)

### Phase 7: System Integration & Local Files

- [x] Add application window and taskbar/dock icon support for Windows, macOS, and Linux distros
- [ ] Add a system tray icon with Play/Pause, Skip, and Quit actions
- [ ] Register global media key bindings (MPRIS on Linux, MediaSession on Windows/macOS)
- [ ] Implement MPRIS2 D-Bus interface on Linux for desktop environment integration
- [x] Local Files: Implement local audio file scanner and playback for custom local music directory path
- [ ] Package the binary as a `.deb` and `.rpm` for Linux
- [x] Package the binary as a `.dmg` / `.app` bundle for macOS
- [x] Package the binary as an `.msi` installer for Windows
- [ ] Integrate auto-update check: compare current version against GitHub Releases on startup
- [ ] Write end-to-end integration tests for the auth flow and audio pipeline

### Phase 8: Performance & Speed Optimization

- [ ] Optimize general app execution speed, reducing UI update latency and startup load time
- [ ] Run a full memory profile and verify the application stays under 25 MB baseline at idle
- [ ] Profile and eliminate any hot-path allocations in the canvas render loop and audio callback
- [ ] Replace any `.clone()` / `.to_string()` in hot paths with borrows (`&str`, `&[u8]`) where applicable
- [ ] Run `cargo clippy --all-targets -- -D warnings` clean and resolve all lints
- [ ] Run `cargo deny check` and ensure no disallowed licenses or duplicated dependencies
- [ ] Set up memory-leak detection in CI (Valgrind or similar) for the audio pipeline
- [ ] Add structured logging (`tracing` crate) with configurable verbosity levels
- [ ] Implement graceful shutdown: flush audio buffers and close the librespot session cleanly on exit

- [x] RAM baseline optimization: bounded image cache handle capacity to 64 items to keep RAM under 25 MB ceiling
