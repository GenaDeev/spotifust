---
name: spotifust-iced-ui
description: Spotifust UI architecture and iced 0.14 / tiny-skia pitfalls. Use when editing src/app.rs or anything in src/ui/ — adding views or widgets, fixing visual glitches, slow scrolling, lag, images not showing, layout overlap, context menus, or the right panel (lyrics, now playing, queue).
---

# Spotifust UI (iced 0.14, tiny-skia)

The renderer is **tiny-skia, CPU only**. Every pixel and every image decode costs CPU time on the UI thread. Most "glitch" and "lag" reports trace back to one of the pitfalls below.

## Architecture map

- `src/app.rs`: `App`, `AppState::Main { … }` (the whole model), `Message`, `update` / `update_inner`, `subscription`, `view` (passes model fields into `ui::main_layout::view`).
- `src/ui/main_layout.rs`: all main-window views. Shared building blocks:
  - `main_page_frame(page)`: main scrollable with id `MAIN_SCROLL_ID`, reports `Message::MainContentScrolled(ScrollWindow)`.
  - `virtual_rows(count, list_top, window, build)` and `visible_row_range(…)`: list virtualization; every row is `TRACK_ROW_HEIGHT` tall.
  - `track_row(&TrackRowSpec, …)`: the standard track row (index, optional cover, title/artist, album, duration, right-click menu).
  - `single_line(text, size, color, bold)`: text with no wrapping, clipped to its box. Use it for any user data in a row or card.
  - `card_shelf(count, build)`: a `responsive` shelf that shows as many `media_card_with_image` cards as fit.
  - `detail_header`, `detail_action_row`, `track_table_header`, `section_title`, `empty_state`, `render_skeleton_*`.
- `src/ui/context_menu.rs`: context menu, modals, toasts (stacked over the base layout).
- `src/ui/theme.rs`: color, radius, spacing and font tokens. `AccentTone` is user-selectable, but most code still uses the fixed `theme::ACCENT` const (known debt).
- `src/ui/mod.rs`: `logo_handle()`, a pre-scaled `OnceLock` image handle.

## The update wrapper

`App::update` runs `update_inner`, then compares:

- `panel_key()` (current track URI plus open right-panel tab) → calls `refresh_track_panels()` to load lyrics/bio, and resets the right-panel scroll and `lyrics_line` when the tab changes.
- `page_key()` (`NavDestination`) → resets `main_scroll` and snaps `MAIN_SCROLL_ID` to the top.

When a new side effect should follow a state change no matter which message caused it, add it here, not in individual handlers.

## Pitfalls (each one has bitten this project)

1. **Image handles in `view()`**: `image::Handle::from_bytes` uses `Id::unique()`, so building one in `view()` makes the raster cache miss on every frame. The 1080px logo was decoded on every redraw. Create handles once (`ImageLoaded` → `loaded_images`, or a `OnceLock`).
2. **Image cache size**: `loaded_images` is an `LruCache` with `IMAGE_CACHE_CAPACITY` (160) entries of 256px JPEG thumbnails. Too small a cap makes covers vanish in long lists. Request images through `load_image_tasks` → `Message::RequestImages`, which dedupes in-flight downloads via `pending_images`.
3. **Nested scrollables**: a vertical scrollable containing another `Fill`-height vertical scrollable renders badly. The right panel body is plain content inside one `thin_scrollable` (id `RIGHT_PANEL_SCROLL_ID`).
4. **Horizontal scrollables steal the wheel**: iced maps vertical wheel deltas onto a horizontal-only scrollable, so the page stops scrolling under the cursor. Use `card_shelf` or `Row::wrap()`.
5. **Long lists**: build only visible rows (`virtual_rows`). `DETAIL_LIST_TOP` must equal the real height above the first row: header 230, spacing 20, action row 56, spacing 20, table header 40. Change both together. `MainContentScrolled` also loads covers for the visible rows of the selected playlist.
6. **Text overflow**: plain `Text` wraps and pushes rows taller or overlaps neighbours. Use `single_line` in rows, cards and the playback bar.
7. **Fixed widths**: fixed px columns overlap on small windows (the playback bar used 300/500/300). Use `FillPortion` plus `max_width`.
8. **tiny-skia limits**: `Image::border_radius` is ignored (images stay square). Blurred shadows are expensive, so use them sparingly.
9. **Cursor position**: don't subscribe to `CursorMoved` into `update`, because every mouse move would rebuild the view. `RightClickRecipe` keeps the cursor position inside its stream and emits only `RightClickAt`. Context menus open un-anchored, and `RightClickAt` anchors them (widget messages arrive before subscription events).
10. **Hot `view()`**: `PlaybackPositionReceived` arrives about 4 times a second and every message re-runs `view()`. Avoid per-row clones of big structs. Build a `TrackInfo` only for rows that are actually rendered.
11. **Honest UI**: don't show fake data (follower counts, verified badges, "Saved" toasts for no-ops). Hide the element and log it in `TODO.md`.
12. **Strings**: UI text is English. The language setting doesn't translate yet (debt), so don't add strings in other languages.

## Verifying UI work

- There is no screenshot tooling for the native window, and no Xvfb on the dev machine. Run `cargo run --release` (or `./target/release/spotifust`), describe what to check, and ask the user.
- For memory: `ps -o rss= -C spotifust`. To separate mapped system fonts from private memory, inspect `/proc/<pid>/smaps` (font files account for ~65 MB of RSS).
- Unit-test update logic by building an `App` in `AppState::Main` (see `main_app_for_tests()` in `src/app.rs` tests) and asserting on state after `app.update(msg)`.
