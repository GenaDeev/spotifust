# Project State Machine

## Current Focus

- [ ] Optimize long playlist loading with incremental chunking/streaming or virtualized pagination to avoid UI lag
  - Audit 2026-10-03: rendering is virtualized since PR #50 (only visible rows are built, covers load per visible row). Still missing: `fetch_playlist_tracks` downloads every page before showing anything. Stream the pages into the view (show the first 100 rows, then append).

## Development Backlog

### Phase 1: Bootstrapping, Core Architecture & Memory Baseline

- [x] Configure Cargo.toml with feature flags for Iced (tiny-skia backend), RSpotify, and Librespot
- [x] Define central `AppError` enum (thiserror) with per-subsystem variants
- [x] Set up base Model-View-Update loop in `src/app.rs`
- [x] Set up full GitHub Actions CI/CD infrastructure, Issue templates, and documentation
- [x] Verify all `librespot` and `rspotify` raw error types are wrapped in `AppError` before reaching `Message` variants
- [x] Audit and eliminate any remaining `.unwrap()` / `.expect()` calls outside `main()` bootstrap
- [ ] Reduce RAM baseline from ~45 MB down to the target < 25 MB ceiling
  - Audit 2026-10-03: not met. Measured RSS ~27-32 MB at idle and ~146 MB while playing: ~50 MB private (heap + anon) plus ~65 MB of mmapped system fonts (see the font fallback item under Architectural Debt). No profiling harness exists yet.

### Phase 2: Spotify Resizable Panel Layout Engine

- [x] Implement 3-column layout structure (Left Sidebar library, Main content, Right panel)
- [x] Add interactive drag handles with `ResizingHorizontally` mouse cursor interaction
- [x] Handle global pointer move/up events for robust dragging/resizing
- [x] Implement right dynamic slot panel showing Now Playing or Queue based on playback bar triggers
- [x] Implement left library sidebar collapse to icon-only compact layout below width threshold
- [x] Persist layout panel widths to disk (`save_layout`, `load_layout`)

### Phase 3: Librespot Audio Engine & Rodio Output Pipeline

- [x] Implement `librespot::core::session::Session` setup and credential-based login
- [x] Implement a custom `librespot` audio `Sink` that captures decoded PCM frames
- [x] Route PCM frames from the custom Sink through a bounded `mpsc` channel to a `rodio` playback thread
- [x] Wire a synthetic sine-wave test pipeline to validate the `rodio` backend end-to-end
- [x] Wire UI Play command to call `player.load()` on the active `librespot` player instance
- [x] Wire UI Pause / Resume commands to the librespot player cleanly without freezing or deadlocks
- [x] Wire UI Skip Next / Skip Previous commands to the librespot player
- [x] Implement Seek: accept a `f32` position ratio, flush in-flight audio buffers, and seek `player` cleanly without stutter
- [x] Extract current track metadata (title, artist, album, duration) from `PlayerEvent` and emit them as `Message::TrackChanged`
- [x] Stream accurate playback position directly from decoded audio stream without wall-clock drift
- [x] Implement end-of-track detection via `PlayerEvent::EndOfTrack` and auto-advance to next track
- [x] Validate that the mpsc channel remains bounded under sustained high-throughput decoding
- [x] Wire volume control: slider value in UI → `rodio::Sink::set_volume()` (full 0.0–1.0 range, not binary)
- [x] Fix seek bar so it travels the full 0–100% range and reflects real playback position
- [x] Handle `librespot` session expiry and reconnection without crashing
- [x] Fix app crash during track playback (`src/audio/sink.rs:35:14: Cannot block the current thread from within a runtime` & `Invalid Spotify URI ''`)
- [x] Refine audio pipeline for 320kbps high-quality bitrate, synchronized rodio pause/resume and instant volume binding

### Phase 4: RSpotify Web API, PKCE Auth & Aggressive Caching

- [x] Implement PKCE Authorization Code Flow with `rspotify`
- [ ] Register `spotifust://callback` custom protocol handler for the OAuth redirect
  - Audit 2026-10-03: only registered on Linux (`install.sh` and `assets/spotifust.desktop`). The macOS `.dmg` (no `CFBundleURLSchemes` in Info.plist) and the Windows `.msi` (no `URL Protocol` registry key) don't register it, so login can't complete there. The callback also reaches the running instance through a plaintext temp file (`spotifust_auth.txt`, polled every 2 s); it holds a one-time auth code, not a token, but single-instance IPC would be cleaner.
- [x] Verify the refresh token is stored exclusively via the OS keychain (`keyring` crate), never as plaintext
- [ ] Implement token refresh on expiry: detect 401 responses and silently re-authenticate
  - Audit 2026-10-03: `with_auto_reauth` only refreshes when the token is already expired by the clock. A 401 on a token that isn't clock-expired (revoked or rotated) is retried once unchanged, then surfaces as `AppError::Auth`, which logs the user out. Force a refresh on 401 instead.
- [x] Fetch the authenticated user's profile (`/me`) and display name and avatar in the sidebar
- [x] Fetch the user's full playlist library (`/me/playlists`, paginated) and stream items into the sidebar list
- [x] Fetch playlist track listings on demand when a playlist is selected
- [x] Fetch the user's saved albums and expose them in a dedicated Albums view
- [x] Fetch the user's top tracks and expose them in a Home/For You view
- [x] Implement search: send queries to `/search` and display track, album, and artist results
- [x] Implement album detail view: fetch `/albums/{id}` and list its tracks
- [x] Support multidisc albums with disc groupings and disc headers in album detail view
- [ ] Implement Spotify-styled dedicated artist page with banner, verified badge, monthly listeners, top tracks, and discography
  - Audit 2026-10-03: the page now has a header, "Popular" (search-based fallback) and discography. There is no banner image. Verified badge and monthly listeners were removed because they were fake: Spotify doesn't expose them to development-mode apps, and `followers`/`genres` come back empty.
- [ ] Fetch currently playing track via `/me/player/currently-playing` on startup and sync UI state
  - Audit 2026-10-03: it copies `is_playing` from whatever device is playing (e.g. the phone), so the local UI can show "playing" while Spotifust is silent. It should restore the track as paused and local, or show the remote device instead.
- [x] Implement album art fetching: download cover images asynchronously and cache to disk in `src/api/cache.rs`
- [ ] Implement a metadata cache layer in `src/api/cache.rs` to avoid redundant API calls (TTL-based)
  - Audit 2026-10-03: `MetadataCache` (TTL) is dead code, never instantiated. Only `DiskMetadataCache` is used, with no expiry: it shows stale data instantly but every launch still refetches everything.
- [x] Implement rate-limit handling: respect `Retry-After` headers from the Spotify API
- [x] Display large cover art in playlist and album detail header views
- [ ] Audit and remove all remaining mock data across all UI views and components, fetching 100% live Spotify API data
  - Audit 2026-10-03: the mini player still shows the placeholder "Synthetic Horizon / Spotifust Audio Engine" when nothing is playing (`view_mini_player`). "Made For You" isn't personalized: `featured_playlists` is 403 for dev-mode apps, so it falls back to a generic "Top Hits" playlist search.
- [x] Validate existing token/session before rendering initial screen to eliminate temporary login flicker
- [x] Achieve near-instant API data loading through aggressive metadata and persistent disk caching in XDG cache dir
- [ ] Implement Track & Artist Radio / Recommendations endpoint (`GET /v1/recommendations`, "Made for You", "New Releases")
  - Audit 2026-10-03: `/v1/recommendations` returns 404 and `/browse/new-releases` and `featured-playlists` return 403 for dev-mode apps. Current fallbacks are a `tag:new` album search (New Releases), a "Top Hits" playlist search (Made For You) and an artist search (autoplay). There is no track or artist radio.
- [x] Fix session loss handling, purge cache on expiry, prevent login flicker and polish non-card login UI
- [ ] Optimize long playlist loading with incremental chunking/streaming or virtualized pagination to avoid UI lag

### Phase 5: UI Design System, View Transitions & Theming

- [x] Define a unified design token system (color palette, spacing scale, typography scale) in a central `theme.rs`
- [ ] Replace all ad-hoc hardcoded color literals and magic numbers with design tokens
  - Audit 2026-10-03: ~30 `Color::from_rgb(a)`/`Color {..}` literals remain (main_layout 13, context_menu 8, login 9), plus many inline sizes and paddings.
- [ ] Support customizing application accent color tone (Spotify Green, Rust Orange, Electric Blue, Deep Purple, Rose Pink) with disk persistence
  - Audit 2026-10-03: the choice persists but only recolors the big play button and the Settings page. Everything else uses the fixed `theme::ACCENT` constant (17 uses in main_layout/context_menu). The tone needs to flow through a theme value instead of a const.
- [ ] Implement animated loading skeletons for album art, playlist headers, and track list placeholders while initial Spotify API data is fetching
  - Audit 2026-10-03: skeletons exist but are static, with no shimmer or pulse animation.
- [x] Remove "Explore Premium" / "Explorar Premium" button from sidebar and navigation
- [ ] Add waveform or animated equalizer bars to the Now Playing area during active playback
  - Audit 2026-10-03: not implemented. The code has no equalizer, waveform or animation (no `window::frames` subscription). The current track row only shows a static volume icon.
- [x] Redesign context menus (right-click) into compact Spotify-styled popovers with accurate ID routing and clean action triggers
- [x] Implement a proper volume slider that covers the full 0–100% range with a mute toggle
- [ ] Add keyboard shortcuts for Play/Pause (Space), Skip (→/←), Volume (↑/↓), Search (Ctrl+F), Mute (Ctrl+M), Shuffle (Ctrl+S), Repeat (Ctrl+R), Queue (Ctrl+Q), Lyrics (Ctrl+D), Back/Forward (Ctrl/Alt + ←/→)
  - Audit 2026-10-03: all of these are mapped, but Ctrl+F only switches to the Search page: it doesn't focus the search input (no widget id or `focus` operation), so the user still has to click it.
- [x] Implement a mini-player / compact mode for when the window is resized to small dimensions
- [x] Add toast / snackbar notifications for user-facing errors and confirmations
- [x] Audit and refine all font sizes, weights, and line heights for visual consistency
- [x] Unify top bar button sizing to 40px, circular avatar, and uniform pill radius
- [x] SETTINGS PAGE: Build base Settings page layout frame
- [x] LYRICS: Implement base Lyrics view layout frame
- [ ] Integrate LRCLIB REST API for millisecond-synced `.lrc` lyrics auto-scrolling with Genius plain lyrics fallback
  - Audit 2026-10-03: LRCLIB works since PR #50 (it never did before: camelCase fields were parsed as snake_case). Auto-follow is proportional (it estimates line position, not exact). There is no Genius fallback; adding one needs an API token, so decide if it's wanted (§6).
- [ ] Integrate Last.fm API (`artist.getInfo`) + Wikipedia REST API for artist bio, curiosities, genres, and similar artists in Now Playing right panel
  - Audit 2026-10-03: only the Wikipedia summary (en, then es) is implemented, and it was broken until PR #50 by a double slash in the URL. There is no Last.fm integration (it needs an API key, §6), no curiosities, no genres and no similar artists.
- [ ] Enhance Search screen with Category Pill filters (Tracks, Albums, Artists, Playlists) and Top Result spotlight card
  - Audit 2026-10-03: pills exist for All/Songs/Artists/Albums. There is no Playlists pill and playlists aren't fetched by `execute_search`. The Top Result card always shows the first track, even when an artist is the better match.
- [x] Implement navigation history with Back & Forward buttons for fluid page transitions
- [ ] Implement smooth hover transitions on sidebar items, buttons, and playback controls
- [ ] Implement smooth progress bar animation that interpolates position between tick updates
- [ ] Ensure the entire UI is navigable via keyboard (tab order, focus rings)
- [ ] Implement Friend Activity / Social Feed side panel in right panel slot

### Phase 6: Playback Queue, History & Audio Controls

- [x] Implement an internal play queue data structure in the `Model`
- [x] Display the current queue in a slide-out panel
- [ ] Implement Shuffle mode: randomise queue order and persist the shuffle seed
  - Audit 2026-10-03: shuffle works, but it is seeded from the current time and neither the seed nor the shuffle on/off state is persisted (`is_shuffled` resets to false on every launch).
- [x] Implement Repeat modes: No Repeat, Repeat Queue, Repeat One
- [x] Implement "Add to queue" action from track context menus
- [x] Implement track reordering and control within the play queue view
- [x] Implement Spotify-style structured User Queue, Context Queue, and playback History stack
- [x] Eliminate progress bar jumps and sync position directly with audio stream
- [ ] Implement Audio Loudness Normalization (ReplayGain / Spotify Normalization) toggle with persistence and player configuration
  - Audit 2026-10-03: persisted, but toggling it doesn't rebuild the librespot `Player`, so the change only applies after a relaunch.
- [ ] Implement Gapless Playback transition between tracks with player configuration and settings toggle
  - Audit 2026-10-03: same as normalization: it only applies after a relaunch. The app never calls `player.preload()` for the next queue item, so transitions mostly rely on the sink tail playing out (added in PR #50) rather than true gapless decoding.
- [ ] Implement drag-and-drop track reordering within a playlist queue view
- [ ] Crossfade: Smooth audio crossfade between tracks (configurable duration in Settings)
- [ ] Spotify Connect: Full bi-directional Spotify Connect integration for remote control and device sync

### Phase 7: Settings System (100% Backend Wired & Persisted)

- [ ] SECTION 1 - Account & Language: External browser link to login methods (`spotify.com/account`) & persistent i18n UI language selector dropdown
  - Audit 2026-10-03: the account link and language persistence work, but the selector has no effect: every UI string is hard-coded English (see Localization under Architectural Debt).
- [ ] SECTION 2.1 - Explicit Content: Explicit content filter toggle (hide `explicit == true` tracks) and [E] badge indicator
  - Audit 2026-10-03: the filter and the [E] badge only apply to search results. Playlists, albums, artist pages, Home and the queue ignore it. The setting isn't persisted either (`allow_explicit_content: true` is hard-coded at startup).
- [ ] SECTION 2.2 - Autoplay: Autoplay toggle switch in Settings and automatic recommendation playback (`/v1/recommendations`) on end of queue
  - Audit 2026-10-03: `/v1/recommendations` returns 404, so autoplay depends on an artist-search fallback in `fetch_recommendations`. The toggle isn't persisted (`autoplay_enabled: true` is hard-coded at startup).
- [ ] SECTION 3 - Audio Quality: Bitrate selector pills (Normal 96k, High 160k, Very High 320k bound to librespot decoder) with disk persistence
  - Audit 2026-10-03: persisted, but changing it doesn't reconnect the audio session, so the new bitrate only applies after a relaunch.
- [x] SECTION 5 - UI Scaling & Hotkeys: UI Scale selector (70%-130%) with `Ctrl +` / `Ctrl -` hotkeys and Reset button
- [ ] SECTION 7.1 - Audio Loudness Normalization toggle switch with disk persistence
  - Audit 2026-10-03: the toggle persists but only takes effect after a relaunch (see the Phase 6 item).
- [ ] SECTION 7.2 - Gapless Playback transition toggle switch with disk persistence
  - Audit 2026-10-03: the toggle persists but only takes effect after a relaunch (see the Phase 6 item).
- [x] SECTION 8 - Storage & Cache: Storage usage indicator (Cache size calculation) and Clear Cache button in Settings (`src/api/cache.rs`)
- [ ] SECTION 4 - Display & Canvas: Display toggles (auto-open Now Playing on play, desktop overlay on playback controls) & Canvas/Video toggles
- [ ] SECTION 6 - Privacy & Profile: Private Session toggle (6h auto-off), recent activity visibility dropdown, connected apps link, and profile element toggles
- [ ] SECTION 7.3 - Playback Controls: Crossfade slider (0-12s), Automix toggle, Smart Shuffle switch, Mono Audio downmix toggle, and audio output device selector dropdown bound to rodio output enumeration
- [ ] SECTION 9 - System, Storage & Hardware: Auto-start on system boot dropdown, Close button minimizes to system tray toggle, Offline storage path relocation picker, Proxy configuration selector, and Hardware Acceleration switch

### Phase 8: Packaging, Distribution & Updates

- [x] Add application window and taskbar/dock icon support for Windows, macOS, and Linux distros
- [x] Package the binary as a `.dmg` / `.app` bundle for macOS via GitHub Actions
- [x] Package the binary as an `.msi` installer for Windows via GitHub Actions
- [x] Package the binary as a `.deb` package for Debian/Ubuntu via GitHub Actions
- [ ] Integrate auto-update check: compare current version against GitHub Releases on startup
  - Audit 2026-10-03: `src/api/updater.rs` exists but nothing calls it: there's no startup task and no UI for an available update.
- [ ] Package the binary as an `.rpm` package for Fedora/RHEL/openSUSE
  - Audit 2026-10-03: `release.yml` already runs `cargo generate-rpm` and `Cargo.toml` has `[package.metadata.generate-rpm]`. Verify that a release actually publishes the `.rpm`, then tick this.
- [ ] Package the binary as Flatpak and AppImage for universal Linux distribution

### Phase 9: Performance, Verification & Hardening

- [x] Run `cargo clippy --all-targets -- -D warnings` clean and resolve all lints
- [x] Run `cargo deny check` and ensure no disallowed licenses or duplicated dependencies
- [ ] Implement graceful shutdown: flush audio buffers and close the librespot session cleanly on exit
  - Audit 2026-10-03: `perform_graceful_shutdown` saves state and `try_send`s `PlayerCommand::Stop`, then `std::process::exit(0)` runs right away. Nothing waits for the stop, the rodio sink isn't drained and the librespot `Session` isn't shut down.
- [ ] RAM baseline optimization: bounded image cache handle capacity with true LRU eviction (16-24 items) to keep RAM under 25 MB ceiling
  - Audit 2026-10-03: the 20-entry cap made covers vanish from any list longer than 20, so PR #50 raised it to 160 (256px JPEG thumbnails, ~3-4 MB). The 25 MB goal isn't met (see the Phase 1 item). The LRU itself works, but views read through `inner_map()` without promoting entries, so recency only updates when images are requested.
- [ ] Run a full memory profile and verify the application stays under 25 MB baseline at idle
- [ ] Profile and eliminate any hot-path allocations in the canvas render loop and audio callback
- [ ] Replace any `.clone()` / `.to_string()` in hot paths with borrows (`&str`, `&[u8]`) where applicable
- [ ] Set up memory-leak detection in CI (Valgrind or similar) for the audio pipeline
- [ ] Add structured logging (`tracing` crate) with configurable verbosity levels
- [ ] Write end-to-end integration tests for the auth flow and audio pipeline

### Phase 10: Desktop Integration, DSP Audio & Future Roadmap

- [ ] Linux MPRIS2 D-Bus Interface: Implement `org.mpris.MediaPlayer2` and `org.mpris.MediaPlayer2.Player` for native media controls, lock screen metadata, and hotkey integration
- [ ] Multi-Band DSP Equalizer: Implement interactive 6-band audio equalizer (60Hz, 150Hz, 400Hz, 1kHz, 2.4kHz, 15kHz) with biquad peaking/shelving filters and genre presets (Flat, Bass Boost, Vocal, Rock, Electronic)
- [ ] Native System Tray: Wire system tray icon for Linux, macOS, and Windows with minimize-to-tray and playback quick menu (Play/Pause, Next, Prev, Show/Hide, Quit)
- [ ] Local Music Library: Implement background local audio file scanner (MP3, FLAC, OGG, WAV, AAC) with ID3 tag parsing and dedicated Local Files sidebar navigation
- [ ] Synchronized Lyrics Enhancements: Click-to-seek directly from lyric line timestamps, smooth line transition highlighting, and romanized lyrics / translation tabs
- [ ] Offline Mode & Resilience: Offline state detection, cached track metadata playback, and visual offline indicator badge in header
- [ ] Drag-and-Drop Playlist Management: Drag tracks onto left sidebar playlists to append items seamlessly
- [ ] Custom Accent Color Picker: Interactive hex code / RGB custom color input in Settings with live application theme re-rendering
- [ ] Global Media Key Bindings: Global media keys listener on Windows (`MediaSession`), macOS (`MPRemoteCommandCenter`), and Linux

## Architectural Debt

- [x] Disk persistence test concurrency & settings preservation: prevent `clear_cache_disk` from wiping user settings and serialize disk tests with a mutex to eliminate multi-threaded test race conditions.
- [ ] Memory profiling harness on Linux: set up automated RSS tracking with `heaptrack` or `valgrind --tool=massif` to guarantee the < 25 MB ceiling under long-running playback.
- [ ] Bounded channel capacity tuning: monitor high-bitrate (320kbps) audio decoding backpressure against rodio sink buffer consumption under low-spec CPU constraints.
- [x] Polish pass (2026-10-03): playback auth via one-time device pairing (keymaster client id, credentials in OS keychain), rodio queue backpressure (whole tracks were buffered in RAM), playback position measured from audibly played samples, LRCLIB camelCase fix + synced-lyrics follow, artist page resilient to 403s, Wikipedia bio disambiguation, image cache 20 → 160 handles with lazy per-row cover loading, virtualized playlist/album lists, static pre-scaled logo handle (was re-decoded every frame), no nested/horizontal scrollables, clipped single-line text, context menus anchored at the cursor.
- [ ] Font fallback RSS: cosmic-text maps ~380 system font files (~65 MB of shared, file-backed RSS) while searching fallback glyphs; consider bundling a font and restricting the font database to keep RSS near the 25 MB target.
- [ ] Spotify development-mode API restrictions: `/artists/{id}/top-tracks`, `/artists/{id}/related-artists` and `/browse/new-releases` return 403 and `/recommendations` returns 404. Artist "Popular" now falls back to search, but "New Releases" and autoplay recommendations need replacement data sources.
- [ ] Audio settings (bitrate, normalisation, gapless) are persisted but only apply after the audio session reconnects (next launch); rebuild the session when they change.
- [ ] Localization: UI strings are hard-coded English even though a UI language setting exists; route them through a translation table.
- [ ] Elm-rule exception: `AudioSession.events` is an `Arc<tokio::sync::Mutex<Receiver>>` stored in the model so `PlayerEventsRecipe` can own the receiver. Replace it with a subscription that owns the channel (e.g. `Subscription::run_with` keyed by a session id) and remove the exception from `AGENTS.md` §1.
- [ ] Stale architecture docs: `README.md` (Tech Stack "UI Layout: iced canvas", architecture diagram, "Canvas Layout System") and `docs/architecture.md` describe a canvas card engine that doesn't exist. The UI is regular iced widgets in `src/ui/main_layout.rs`. Agents read these files, so rewrite them to match the code.

## Blocked / Needs Human Decision

- [ ] Decision on D-Bus / MPRIS2 crate dependency: select between lightweight raw D-Bus connection or `zbus` crate for Linux desktop media player integration.
- [ ] Decision on System Tray crate dependency: select between `tray-icon` (cross-platform, Tauri-maintained) or minimal platform-native hooks for system tray integration.
- [ ] Confirm the playback auth approach: librespot streaming only works with credentials issued to Spotify's desktop (keymaster) client id, obtained through the OAuth device flow (`spotify.com/pair`, no local port). Web API calls still use Spotifust's own PKCE client. Approve keeping this, or pick an alternative.
- [ ] Keep or remove `examples/playback_probe.rs` (diagnostic tool, adds the `uuid` dev-dependency).
