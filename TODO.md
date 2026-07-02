# Project State Machine

## Current Focus
- [ ] Implement bounding-box tracking struct for responsive cards

## Development Backlog

### Phase 1: Bootstrapping & Core Architecture
- [x] Configure Cargo.toml with feature flags for Iced (wgpu backend), RSpotify, and Librespot
- [x] Define central `AppError` enum (thiserror) with per-subsystem variants
- [x] Set up base Model-View-Update loop in `src/app.rs`

### Phase 2: Custom Canvas Layout Engine
- [ ] Implement bounding-box tracking struct for responsive cards
- [ ] Handle PointerPressed / Moved / Released inside `canvas::Program::update`
- [ ] Wire `canvas::Cache` invalidation to interaction messages only

### Phase 3: Audio & Session Pipeline
- [ ] Spawn librespot session on tokio::spawn, bridged via mpsc → iced::Subscription
- [ ] Wire rodio/cpal sink for decoded PCM playback

### Phase 4: Web API & Auth
- [ ] Implement PKCE flow with fixed-port loopback TcpListener
- [ ] Store refresh token via OS keychain (not plaintext)

## Architectural Debt
- [ ] (Anything discovered mid-implementation that isn't yet in the backlog above)

## Blocked / Needs Human Decision
- [x] Decide on the final audio playback backend (rodio vs cpal) - rodio chosen
