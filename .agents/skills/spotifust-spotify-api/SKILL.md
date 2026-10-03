---
name: spotifust-spotify-api
description: Spotify Web API, auth and third-party data providers in Spotifust (rspotify 0.16 PKCE, keychain tokens, dev-mode endpoint restrictions, latency, images, LRCLIB lyrics, Wikipedia bios). Use when editing src/api/, adding or debugging an endpoint, 403/404/429 errors, login or token problems, slow loading or search, missing covers, lyrics or artist bios.
---

# Spotify Web API and data providers

## Client and auth

- `api::auth::get_spotify_client()` builds an `rspotify::AuthCodePkceSpotify` with Spotifust's own client id (`CLIENT_ID`, overridable via the `SPOTIFY_CLIENT_ID` env var) and redirect `spotifust://callback`. There is no client secret.
- Login (`do_login_flow`): opens the browser. The OS launches a second `spotifust spotifust://callback?...`, which writes the URL to `$TMP/spotifust_auth.txt` and exits (`main.rs`). The main instance polls that file. The protocol is registered only on Linux (`install.sh`, `.deb` desktop file).
- Keychain entries (service = `keyring_service()`):

  | User | Content |
  | :--- | :--- |
  | `spotify_refresh_token` | Web API refresh token |
  | `spotify_session_token` | Full `rspotify::Token` JSON. Startup reuses it while it has more than 5 min left (`token_is_fresh`), skipping a refresh round trip |
  | `librespot_credentials` | Playback credentials (see the `spotifust-audio` skill) |

- `keyring_service()` returns `"spotifust-test"` under `cfg(test)`, and `get_cache_dir()` returns a temp dir under test. That stops unit tests from wiping the developer's real login and cache, which they used to do. Never hard-code the service name.
- Wrap every call in `with_auto_reauth(spotify, || async { … })` and map errors with `map_rspotify_error`. A 429 sleeps for `Retry-After` and retries (up to 3 times). An `Auth` error (401) retries once after `refresh_token_if_expired`, which only refreshes a token that is already expired by the clock. A revoked token that hasn't expired yet still ends in a logout (known debt in `TODO.md`).
- `AppError::Auth` logs the user out and clears the cache. `classify_refresh_error` returns `Auth` only for HTTP 400/401 on refresh; everything else is `Network`.

## Development-mode restrictions

Spotifust's client id is a development-mode app. Measured on 2026-10-03 with a real account:

| Endpoint or field | Result | Current handling |
| :--- | :--- | :--- |
| `GET /artists/{id}/top-tracks` | 403 | `top_tracks_via_search` (`artist:"Name"` track search, filtered by artist id) |
| `GET /artists/{id}/related-artists` | 403 | not used |
| `GET /browse/new-releases` | 403 | `tag:new` album search fallback |
| `GET /recommendations` | 404 | artist-search fallback in `fetch_recommendations` (autoplay) |
| `GET /browse/featured-playlists` | unreliable | "Top Hits" playlist search fallback |
| Artist `followers`, `genres`, `popularity` | absent | rspotify 0.16 defaults them; don't display zeros |
| Playlist object `tracks` | renamed to `items` | rspotify 0.16 exposes both (`#[allow(deprecated)]`) |
| `/me`, `/me/playlists`, `/me/albums`, `/me/top/tracks`, `/search`, `/albums/{id}`, `/artists/{id}`, `/artists/{id}/albums`, `/me/player/currently-playing` | 200 | |

Before you build on any other endpoint, probe it (see Recipes). A 403 on one call must not fail a whole page: degrade that section only (see `fetch_artist_details`).

## Latency

- From the developer's network: TCP RTT ~240 ms to `api.spotify.com`, and a typical authenticated request takes ~0.5–0.7 s. Every sequential request adds that much.
- reqwest has the `http2` feature, so concurrent requests multiplex over one connection. Fire independent requests with `Task::batch` (see `LoginSuccess`).
- Search makes **one** `search_multiple` request (tracks, albums, artists), with a 150 ms debounce (`SEARCH_DEBOUNCE`), a per-query LRU (`search_cache`, 32 entries) and a stale-response guard (`SearchResultsFetched(query, …)`). Previous results stay visible while the next query loads.
- Startup shows disk-cached metadata (`DiskMetadataCache`) and refetches in the background. The in-memory TTL `MetadataCache` is unused (debt).

## Images

- `api::best_image_url(&images)` picks the smallest image at least 300px wide, instead of the 640px original.
- `ImageCache::fetch_image_bytes` uses a pooled client, caches on disk, and decodes and thumbnails (256px) on `spawn_blocking`.

## Lyrics (`api/lyrics.rs`, LRCLIB)

- LRCLIB responds in **camelCase** (`syncedLyrics`, `plainLyrics`). Snake-case structs silently get `None`, which is the bug that left lyrics empty.
- It queries `/api/get` with the primary artist (`primary_artist`), album and duration. If that has no synced lyrics, it falls back to `/api/search` with a cleaned title (`clean_title`) and picks the best match (synced first, then closest duration within 8 s).
- Send a descriptive `User-Agent`. `LyricsData.synced == false` means plain text: don't highlight or seek with it.

## Artist bio (`api/artist.rs`, Wikipedia REST)

- Build summary URLs with `path_segments_mut().pop_if_empty().push(title)`. Without `pop_if_empty` the URL became `/summary//Title` and every lookup failed.
- Try the plain name, then `(band)`, `(musician)`, `(rapper)`, `(singer)`, `(group)`, then a title-prefix search, in English first and then Spanish. Accept a page only if `looks_like_music_act` (whole-word keyword match), so "Oasis" doesn't resolve to the desert landform.

## Recipes

- **Live test against the real account** (read-only): add an `#[ignore]` test in the module and run `SPOTIFUST_LIVE_KEYRING=1 cargo test <name> -- --ignored --nocapture`. Existing examples: `api::search::live_tests::live_search_latency`, `api::artist::live_tests::live_bio_lookup`. `check_existing_login()` gives you an authenticated client. Never print tokens.
- **Probe an endpoint's status**: inside such a test, read the access token from `spotify.get_token()`, call the endpoint with `reqwest` and print only the status and JSON keys.
- **Measure network vs app time**: `curl -s -o /dev/null -w "connect %{time_connect} ttfb %{time_starttransfer}\n" https://api.spotify.com/v1/search?q=x&type=track`.
