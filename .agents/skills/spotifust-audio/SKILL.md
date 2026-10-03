---
name: spotifust-audio
description: Spotifust audio pipeline (librespot session, device pairing, PCM sink, rodio, playback clock). Use when editing src/audio/ or playback messages in src/app.rs — tracks "unavailable for playback", pairing/login5 errors, playback position drift, pause/resume bugs, auto-skip, gapless, audio memory growth, ALSA underruns.
---

# Spotifust audio pipeline

## Topology

```text
keychain (librespot_credentials)
  → librespot Session (SessionConfig::default(): desktop/keymaster client id)
  → librespot Player (bitrate, normalisation, gapless from settings)
  → MpscSink::write: f64→f32, tag packet with PlaybackClock::requested() generation
  → std::sync::mpsc::sync_channel::<PcmPacket>(8)          (bounded: backpressure #1)
  → rodio thread: drop stale generations; wait while sink.len() >= MAX_QUEUED_PACKETS (24)
                                                             (bounded: backpressure #2)
  → CountingSource (counts samples as rodio pulls them) → PlaybackClock
  → session task (tokio::spawn): 250 ms tick → AudioSessionEvent::PositionMs
  → bounded tokio mpsc(32) → PlayerEventsRecipe subscription → Message::PlaybackPositionReceived
```

Files: `src/audio/session.rs` (connect, command loop, `PlayerCommand`, `AudioSessionEvent`), `src/audio/sink.rs` (`MpscSink`, `PlaybackClock`, `CountingSource`, rodio thread), `src/audio/credentials.rs` (pairing and keychain).

## Credentials and pairing (why playback works at all)

- login5, which every track load needs, only accepts stored credentials issued to Spotify's **desktop (keymaster) client id**. A Web API token from Spotifust's own client id connects to the access point, but loads fail with `Login request was denied: BAD_REQUEST` and the player emits `PlayerEvent::Unavailable`.
- Pairing uses `librespot::oauth::DeviceAuthClient` (OAuth device authorization flow): the user approves at `spotify.com/pair?code=…`, with no redirect and no local port. After `Session::connect`, `store_session_credentials` saves the reusable `auth_data` as JSON in the keychain.
- `connect_with_credentials` verifies `session.login5().auth_token()` before returning. Failures map to `AppError::PlaybackPairing`, and the UI then starts pairing (`Message::PlaybackPairingStarted`, banner in `view_playback_link_banner`).
- App-side state: `PlaybackLink::{Connecting, AwaitingApproval, Ready, Failed}`. While audio isn't ready, `PlayTrack` stores `pending_play_uri` and plays it on `AudioSessionConnected`.
- A dropped session (event channel closed, `session.is_invalid()`) → `Message::SessionExpired` → reconnect with stored credentials. It never logs the user out.

## Position and timing rules

- **Never** feed `PlayerEvent::{Playing, Paused, Seeked}.position_ms` to the UI. Those are decoder positions, which run ahead of the speakers. Using them made each pause/resume jump forward until the track "ended".
- Each load or seek calls `PlaybackClock::next_generation(flush)`. The audible position is `base_ms + played samples of that generation`. Until the first sample of the new generation plays, the position stays at `base_ms`, so the bar waits for the audio.
- `EndOfTrack` fires when decoding finishes, ~1 s before the audible end. The next `Play` after a natural end doesn't flush the sink (`track_ended`), so the tail plays out.
- `PlayerCommand::PlayFrom(uri, ms)` loads at a position. It resumes tracks restored from disk (`PlaybackState::track_loaded == false`).
- Skip next/prev is implemented in the app queue (`Message::SkipNext/SkipPrev` → `PlayTrack`). `PlayerCommand::SkipNext/SkipPrev` are no-ops.

## Memory

- Without backpressure #2, rodio's queue held whole decoded tracks (~75 MB of f32 per song). If memory grows during playback, check that path first.
- 44.1 kHz stereo f32 is ~350 KB/s. 24 packets is roughly 0.5–1 s of buffer.

## Known gaps (see TODO.md)

- Bitrate, normalisation and gapless only apply when the session reconnects (next launch).
- No `player.preload()` for the next track, so it isn't true gapless.
- Graceful shutdown doesn't wait for `Stop` or close the `Session`.
- No logger is initialised, so `RUST_LOG` does nothing in the app. For librespot internals, write a small `examples/` probe or add logging after a dependency decision (§6).

## Testing

- Unit tests: `PlaybackClock` (`sink.rs` tests) and the bounded-channel test. Add tests for any new clock or generation logic.
- End-to-end playback needs the user's paired account: build release, run the app, ask the user to play, pause/resume, seek and let a track end. Watch RSS with `ps -o rss= -C spotifust`.
