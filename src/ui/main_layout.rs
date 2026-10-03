use crate::app::{
    Message, NavigationItem, PlaybackState, RightPanelTab, SearchCategoryFilter, SidebarFilter,
};
use crate::ui::icons::Icon;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Theme,
    widget::{
        Button, Column, Container, Image, Row, Scrollable, Space, Text, TextInput, container,
        scrollable, slider, text_input,
    },
};

/// Height of one row in virtualized track lists (playlist / album pages).
pub const TRACK_ROW_HEIGHT: f32 = 56.0;
/// Rows rendered above and below the viewport so fast scrolling never shows gaps.
const VIRTUAL_BUFFER_ROWS: usize = 10;
/// Offset from the top of a detail page's scroll content to its first track row:
/// header (230) + spacing (20) + action row (56) + spacing (20) + table header (40).
pub const DETAIL_LIST_TOP: f32 = 366.0;
/// Scrollable id of the main content column, used to reset scroll on navigation.
pub const MAIN_SCROLL_ID: &str = "main-content-scroll";
/// Scrollable id of the right panel, used to follow the active lyric line.
pub const RIGHT_PANEL_SCROLL_ID: &str = "right-panel-scroll";

const MEDIA_CARD_WIDTH: f32 = 174.0;
const MEDIA_CARD_HEIGHT: f32 = 236.0;
const CARD_SPACING: f32 = 16.0;

/// Rows `[first, last)` of a uniform list that intersect the viewport (plus a buffer).
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn visible_row_range(
    count: usize,
    list_top: f32,
    window: crate::app::ScrollWindow,
) -> (usize, usize) {
    let relative = (window.offset_y - list_top).max(0.0);
    let first = ((relative / TRACK_ROW_HEIGHT) as usize)
        .saturating_sub(VIRTUAL_BUFFER_ROWS)
        .min(count);
    let span = (window.viewport_height.max(0.0) / TRACK_ROW_HEIGHT).ceil() as usize
        + 2 * VIRTUAL_BUFFER_ROWS
        + 1;
    (first, (first + span).min(count))
}

/// Builds only the rows near the viewport, padding the rest with empty space so the
/// scrollbar still reflects the full list. Every row must be [`TRACK_ROW_HEIGHT`] tall.
#[allow(clippy::cast_precision_loss)]
fn virtual_rows<'a>(
    count: usize,
    list_top: f32,
    window: crate::app::ScrollWindow,
    mut build: impl FnMut(usize) -> Element<'a, Message>,
) -> Element<'a, Message> {
    let (first, last) = visible_row_range(count, list_top, window);
    let mut col = Column::new().width(Length::Fill);
    if first > 0 {
        col = col.push(Space::new().height(Length::Fixed(first as f32 * TRACK_ROW_HEIGHT)));
    }
    for idx in first..last {
        col = col.push(
            Container::new(build(idx))
                .height(Length::Fixed(TRACK_ROW_HEIGHT))
                .align_y(iced::alignment::Vertical::Center),
        );
    }
    if last < count {
        col =
            col.push(Space::new().height(Length::Fixed((count - last) as f32 * TRACK_ROW_HEIGHT)));
    }
    col.into()
}

/// One line of text that never wraps and is clipped to its box, so long titles
/// can't push rows taller or spill over neighbouring columns.
fn single_line<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
    size: f32,
    color: Color,
    bold: bool,
) -> Container<'a, Message> {
    let mut text = Text::new(content)
        .size(size)
        .color(color)
        .wrapping(iced::widget::text::Wrapping::None);
    if bold {
        text = text.font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        });
    }
    Container::new(text).width(Length::Fill).clip(true)
}

/// A horizontal shelf that shows as many cards as fit the available width instead of
/// a nested horizontal scrollable (which swallowed vertical wheel events).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn card_shelf<'a>(
    count: usize,
    build: impl Fn(usize) -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    iced::widget::responsive(move |size| {
        let fit = ((size.width + CARD_SPACING) / (MEDIA_CARD_WIDTH + CARD_SPACING)).floor();
        let shown = (fit.max(1.0) as usize).min(count);
        let mut row = Row::new().spacing(CARD_SPACING);
        for idx in 0..shown {
            row = row.push(build(idx));
        }
        row.into()
    })
    .height(Length::Fixed(MEDIA_CARD_HEIGHT))
    .into()
}

fn view_image_or_icon<'a>(
    url: Option<&str>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    fallback_icon: Icon,
    size: f32,
    radius: f32,
) -> Element<'a, Message> {
    if let Some(url_str) = url {
        if let Some(handle) = loaded_images.get(url_str) {
            let img = Image::new(handle.clone())
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .content_fit(iced::ContentFit::Cover);

            return Container::new(img)
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .style(move |_theme| container::Style {
                    border: Border {
                        radius: radius.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into();
        }
    }
    Container::new(fallback_icon.view_colored(size * 0.45, theme::TEXT_SECONDARY))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: radius.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

#[allow(
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    clippy::too_many_lines
)]
pub fn view<'a>(
    nav_item: &'a NavigationItem,
    playback: &'a PlaybackState,
    sidebar_width: f32,
    right_panel_width: f32,
    active_right_panel: Option<RightPanelTab>,
    user_profile: Option<&'a crate::api::user::UserProfile>,
    user_playlists: &'a [crate::api::playlist::PlaylistSummary],
    user_albums: &'a [crate::api::album::AlbumSummary],
    user_top_tracks: &'a [crate::api::tracks::TopTrack],
    featured_playlists: &'a [crate::api::playlist::PlaylistSummary],
    featured_albums: &'a [crate::api::album::AlbumSummary],
    search_query: &'a str,
    search_results: &'a crate::api::search::SearchResults,
    is_searching: bool,
    sidebar_filter: SidebarFilter,
    selected_playlist: Option<&'a crate::app::SelectedPlaylistState>,
    selected_album: Option<&'a crate::app::SelectedAlbumState>,
    selected_artist: Option<&'a crate::app::SelectedArtistState>,
    user_queue: &'a [crate::app::TrackInfo],
    context_queue: &'a [crate::app::TrackInfo],
    context_index: usize,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    window_width: f32,
    window_height: f32,
    active_context_menu: Option<&'a crate::app::ContextMenuState>,
    active_modal: Option<&'a crate::app::ActiveModal>,
    toast_notification: Option<&'a String>,
    can_go_back: bool,
    can_go_forward: bool,
    current_lyrics: Option<&'a crate::api::lyrics::LyricsData>,
    is_loading_lyrics: bool,
    current_artist_bio: Option<&'a crate::api::artist::ArtistBio>,
    is_loading_artist_bio: bool,
    autoplay_enabled: bool,
    search_category_filter: SearchCategoryFilter,
    cache_size_bytes: u64,
    allow_explicit_content: bool,
    ui_scale: f32,
    accent_tone: crate::ui::theme::AccentTone,
    ui_language: crate::app::UiLanguage,
    audio_bitrate: crate::audio::session::AudioBitrate,
    audio_normalization: bool,
    gapless_playback: bool,
    main_scroll: crate::app::ScrollWindow,
    playback_link: &'a crate::app::PlaybackLink,
) -> Element<'a, Message> {
    if window_width < 600.0 {
        return view_mini_player(playback, loaded_images);
    }

    let top_bar = view_top_bar(
        *nav_item,
        user_profile,
        search_query,
        loaded_images,
        can_go_back,
        can_go_forward,
    );
    let sidebar = view_sidebar_panel(
        sidebar_width,
        user_playlists,
        user_albums,
        sidebar_filter,
        selected_playlist,
        selected_album,
        loaded_images,
    );
    let main_content = view_main_content(
        *nav_item,
        selected_playlist,
        selected_album,
        selected_artist,
        user_playlists,
        user_albums,
        user_top_tracks,
        featured_playlists,
        featured_albums,
        search_results,
        is_searching,
        loaded_images,
        autoplay_enabled,
        search_category_filter,
        cache_size_bytes,
        allow_explicit_content,
        ui_scale,
        accent_tone,
        user_profile,
        ui_language,
        audio_bitrate,
        audio_normalization,
        gapless_playback,
        main_scroll,
        playback.current_track.as_ref().map(|t| t.uri.as_str()),
    );
    let right_panel = view_right_panel(
        active_right_panel,
        right_panel_width,
        playback,
        user_queue,
        context_queue,
        context_index,
        loaded_images,
        current_lyrics,
        is_loading_lyrics,
        current_artist_bio,
        is_loading_artist_bio,
    );
    let playback_bar = view_playback_bar(playback, active_right_panel, loaded_images);

    let mut middle_row = Row::new()
        .push(sidebar)
        .push(view_drag_handle(true))
        .push(main_content);

    if active_right_panel.is_some() {
        middle_row = middle_row.push(view_drag_handle(false)).push(right_panel);
    }

    let middle_section = middle_row
        .padding(iced::Padding {
            top: 0.0,
            right: 8.0,
            bottom: 8.0,
            left: 8.0,
        })
        .height(Length::Fill);

    let mut layout = Column::new().push(top_bar);
    if let Some(banner) = view_playback_link_banner(playback_link) {
        layout = layout.push(banner);
    }
    let layout = layout.push(middle_section).push(playback_bar);

    let base_container = Container::new(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::BG_BASE)),
            ..Default::default()
        });

    let mut stack = iced::widget::Stack::new().push(base_container);

    if let Some(ctx_state) = active_context_menu {
        stack = stack.push(crate::ui::context_menu::view_context_menu(
            ctx_state,
            iced::Size::new(window_width, window_height),
        ));
    }

    if let Some(modal) = active_modal {
        stack = stack.push(crate::ui::context_menu::view_modal(modal, user_playlists));
    }

    if toast_notification.is_some() {
        stack = stack.push(crate::ui::context_menu::view_toasts(toast_notification));
    }

    stack.into()
}

#[allow(clippy::too_many_lines)]
fn view_top_bar<'a>(
    current_nav: NavigationItem,
    user_profile: Option<&'a crate::api::user::UserProfile>,
    search_query: &'a str,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    can_go_back: bool,
    can_go_forward: bool,
) -> Element<'a, Message> {
    let logo_handle = crate::ui::logo_handle();
    let logo_img = Image::new(logo_handle)
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .filter_method(iced::widget::image::FilterMethod::Linear);

    let logo_section = Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .push(logo_img)
        .push(
            Text::new("Spotifust")
                .size(20)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        );

    let back_btn =
        icon_button_circle_disabled_top_bar(Icon::ChevronLeft, Message::NavigateBack, can_go_back);
    let forward_btn = icon_button_circle_disabled_top_bar(
        Icon::ChevronRight,
        Message::NavigateForward,
        can_go_forward,
    );

    let home_btn = icon_button_circle_active(
        Icon::Home,
        Message::NavigationSelected(NavigationItem::Home),
        current_nav == NavigationItem::Home,
    );

    let search_input = TextInput::new("What do you want to play?", search_query)
        .on_input(Message::SearchInputChanged)
        .size(14)
        .width(Length::Fill)
        .style(|_theme: &Theme, status| {
            let base = text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: Border {
                    width: 0.0,
                    color: Color::TRANSPARENT,
                    radius: 0.0.into(),
                },
                icon: theme::TEXT_SECONDARY,
                placeholder: theme::TEXT_SECONDARY,
                value: theme::TEXT_PRIMARY,
                selection: theme::ACCENT,
            };
            match status {
                text_input::Status::Focused { .. } => text_input::Style {
                    border: Border {
                        width: 0.0,
                        color: Color::TRANSPARENT,
                        radius: 0.0.into(),
                    },
                    ..base
                },
                _ => base,
            }
        });

    let search_bar = Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .spacing(10)
            .push(Icon::Search.view_colored(18.0, theme::TEXT_SECONDARY))
            .push(search_input),
    )
    .height(Length::Fixed(40.0))
    .width(Length::Fixed(400.0))
    .padding([0, 16])
    .align_y(iced::alignment::Vertical::Center)
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::SURFACE_CARD)),
        border: Border {
            radius: theme::RADIUS_PILL.into(),
            color: theme::BORDER_SUBTLE,
            width: 1.0,
        },
        text_color: Some(theme::TEXT_SECONDARY),
        ..Default::default()
    });

    let avatar_url = user_profile.and_then(|p| p.avatar_url.as_deref());
    let user_avatar_content = view_image_or_icon(
        avatar_url,
        loaded_images,
        Icon::User,
        40.0,
        theme::RADIUS_PILL,
    );

    let user_avatar_btn = Button::new(
        Container::new(user_avatar_content)
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(Message::OpenSpotifyAccount)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                color: theme::BORDER_SUBTLE,
                width: 1.0,
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    color: theme::TEXT_SECONDARY,
                    width: 1.0,
                },
                ..base
            },
            _ => base,
        }
    });

    let settings_btn = icon_button_circle_top_bar(
        Icon::Settings,
        Message::NavigationSelected(NavigationItem::Settings),
    );

    let plus_btn = icon_button_circle_top_bar(
        Icon::Plus,
        Message::ShowToast("Playlist creation coming soon".to_string()),
    );

    let right_controls = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(settings_btn)
        .push(plus_btn)
        .push(user_avatar_btn);

    Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .push(logo_section)
            .push(Space::new().width(Length::Fill))
            .push(
                Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(back_btn)
                    .push(forward_btn)
                    .push(home_btn)
                    .push(search_bar),
            )
            .push(Space::new().width(Length::Fill))
            .push(right_controls),
    )
    .width(Length::Fill)
    .height(Length::Fixed(72.0))
    .padding(iced::Padding {
        top: 12.0,
        right: 24.0,
        bottom: 6.0,
        left: 24.0,
    })
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::BG_BASE)),
        ..Default::default()
    })
    .into()
}

#[allow(clippy::too_many_lines)]
fn view_sidebar_panel<'a>(
    width: f32,
    playlists: &'a [crate::api::playlist::PlaylistSummary],
    albums: &'a [crate::api::album::AlbumSummary],
    filter: SidebarFilter,
    selected_playlist: Option<&'a crate::app::SelectedPlaylistState>,
    selected_album: Option<&'a crate::app::SelectedAlbumState>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let is_compact = width < 120.0;

    if is_compact {
        let mut list = Column::new().spacing(12).align_x(Alignment::Center);

        list = list.push(
            Button::new(
                Container::new(Icon::Heart.view_colored(18.0, Color::WHITE))
                    .width(Length::Fixed(40.0))
                    .height(Length::Fixed(40.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::ACCENT)),
                        border: Border {
                            radius: theme::RADIUS_MD.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .padding(0)
            .on_press(Message::NavigationSelected(NavigationItem::Home))
            .style(|_theme, status| {
                let base = iced::widget::button::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    ..Default::default()
                };
                match status {
                    iced::widget::button::Status::Hovered => iced::widget::button::Style {
                        background: Some(Background::Color(theme::SURFACE_HOVER)),
                        ..base
                    },
                    _ => base,
                }
            }),
        );

        let library_items = [
            (Icon::MusicNote, SidebarFilter::Playlists),
            (Icon::Album, SidebarFilter::Albums),
        ];

        for (icon, flt) in library_items {
            list = list.push(
                Button::new(
                    Container::new(icon.view_colored(18.0, theme::TEXT_SECONDARY))
                        .width(Length::Fixed(40.0))
                        .height(Length::Fixed(40.0))
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                        .style(|_theme| container::Style {
                            background: Some(Background::Color(theme::SURFACE_CARD)),
                            border: Border {
                                radius: theme::RADIUS_MD.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                )
                .padding(0)
                .on_press(Message::SidebarFilterSelected(flt))
                .style(|_theme, status| {
                    let base = iced::widget::button::Style {
                        background: Some(Background::Color(Color::TRANSPARENT)),
                        ..Default::default()
                    };
                    match status {
                        iced::widget::button::Status::Hovered => iced::widget::button::Style {
                            background: Some(Background::Color(theme::SURFACE_HOVER)),
                            ..base
                        },
                        _ => base,
                    }
                }),
            );
        }

        let scrollable_list = thin_scrollable(list).height(Length::Fill);

        return Container::new(
            Column::new()
                .spacing(16)
                .align_x(Alignment::Center)
                .push(Icon::Library.view_colored(22.0, theme::TEXT_SECONDARY))
                .push(scrollable_list),
        )
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .padding([16, 0])
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_MAIN)),
            border: Border {
                radius: theme::RADIUS_LG.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into();
    }

    let header = Row::new()
        .align_y(Alignment::Center)
        .push(
            Button::new(
                Row::new()
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(Icon::Library.view_colored(22.0, theme::TEXT_SECONDARY))
                    .push(
                        Text::new("Your Library")
                            .size(15)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_SECONDARY),
                    ),
            )
            .padding(0)
            .on_press(Message::ClearSelection)
            .style(|_theme, status| {
                let base = iced::widget::button::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    ..Default::default()
                };
                match status {
                    iced::widget::button::Status::Hovered => iced::widget::button::Style {
                        text_color: theme::TEXT_PRIMARY,
                        ..base
                    },
                    _ => base,
                }
            }),
        )
        .push(Space::new().width(Length::Fill));

    let filter_chips = Row::new()
        .spacing(8)
        .push(filter_chip(
            "All",
            filter == SidebarFilter::All,
            Message::SidebarFilterSelected(SidebarFilter::All),
        ))
        .push(filter_chip(
            "Playlists",
            filter == SidebarFilter::Playlists,
            Message::SidebarFilterSelected(SidebarFilter::Playlists),
        ))
        .push(filter_chip(
            "Albums",
            filter == SidebarFilter::Albums,
            Message::SidebarFilterSelected(SidebarFilter::Albums),
        ));

    let mut list = Column::new().spacing(4);

    let show_playlists = filter == SidebarFilter::All || filter == SidebarFilter::Playlists;
    let show_albums = filter == SidebarFilter::All || filter == SidebarFilter::Albums;

    if show_playlists {
        for p in playlists {
            let is_active = selected_playlist.is_some_and(|sp| sp.id == p.id);
            let sub = format!("Playlist • {} tracks", p.total_tracks);
            let p_id = p.id.clone();
            let p_clone = p.clone();

            let item_element = sidebar_item_with_image(
                &p.name,
                &sub,
                p.image_url.as_deref(),
                loaded_images,
                Icon::MusicNote,
                is_active,
                Message::SelectPlaylist(p_id),
            );

            let item_with_context = iced::widget::mouse_area(item_element).on_right_press(
                Message::OpenPlaylistContextMenu {
                    playlist: p_clone,
                    position: iced::Point::new(200.0, 300.0),
                },
            );

            list = list.push(item_with_context);
        }
    }

    if show_albums {
        for a in albums {
            let is_active = selected_album.is_some_and(|sa| sa.id == a.id);
            let sub = format!("Album • {}", a.artist_name);
            let a_id = a.id.clone();
            let a_clone = a.clone();

            let item_element = sidebar_item_with_image(
                &a.name,
                &sub,
                a.image_url.as_deref(),
                loaded_images,
                Icon::Album,
                is_active,
                Message::SelectAlbum(a_id),
            );

            let item_with_context = iced::widget::mouse_area(item_element).on_right_press(
                Message::OpenAlbumContextMenu {
                    album: a_clone,
                    position: iced::Point::new(200.0, 300.0),
                },
            );

            list = list.push(item_with_context);
        }
    }

    if playlists.is_empty() && albums.is_empty() {
        list = list.push(render_skeleton_rows(5));
    }

    let scrollable_list = thin_scrollable(list).height(Length::Fill);

    let content = Column::new()
        .spacing(14)
        .push(header)
        .push(filter_chips)
        .push(scrollable_list);

    Container::new(content)
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .padding(16)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_MAIN)),
            border: Border {
                radius: theme::RADIUS_LG.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools
)]
fn view_main_content<'a>(
    current_nav: NavigationItem,
    selected_playlist: Option<&'a crate::app::SelectedPlaylistState>,
    selected_album: Option<&'a crate::app::SelectedAlbumState>,
    selected_artist: Option<&'a crate::app::SelectedArtistState>,
    user_playlists: &'a [crate::api::playlist::PlaylistSummary],
    user_albums: &'a [crate::api::album::AlbumSummary],
    user_top_tracks: &'a [crate::api::tracks::TopTrack],
    featured_playlists: &'a [crate::api::playlist::PlaylistSummary],
    featured_albums: &'a [crate::api::album::AlbumSummary],
    search_results: &'a crate::api::search::SearchResults,
    is_searching: bool,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    autoplay_enabled: bool,
    search_category_filter: SearchCategoryFilter,
    cache_size_bytes: u64,
    allow_explicit_content: bool,
    ui_scale: f32,
    accent_tone: crate::ui::theme::AccentTone,
    user_profile: Option<&'a crate::api::user::UserProfile>,
    ui_language: crate::app::UiLanguage,
    audio_bitrate: crate::audio::session::AudioBitrate,
    audio_normalization: bool,
    gapless_playback: bool,
    main_scroll: crate::app::ScrollWindow,
    playing_uri: Option<&str>,
) -> Element<'a, Message> {
    if current_nav == NavigationItem::Settings {
        return view_settings_page(
            autoplay_enabled,
            cache_size_bytes,
            allow_explicit_content,
            ui_scale,
            accent_tone,
            user_profile,
            ui_language,
            audio_bitrate,
            audio_normalization,
            gapless_playback,
        );
    }

    if current_nav == NavigationItem::Search {
        return view_search_results(
            search_results,
            is_searching,
            loaded_images,
            search_category_filter,
            allow_explicit_content,
            playing_uri,
        );
    }

    if let Some(sp) = selected_playlist {
        let cover_url = sp
            .image_url
            .as_deref()
            .or_else(|| sp.tracks.first().and_then(|t| t.image_url.as_deref()));
        let total_ms: u64 = sp.tracks.iter().map(|t| u64::from(t.duration_ms)).sum();
        let subtitle = format!(
            "{} songs • {}",
            sp.tracks.len(),
            format_total_duration(total_ms)
        );
        let header = detail_header(
            "PLAYLIST",
            &sp.name,
            subtitle,
            cover_url,
            Icon::MusicNote,
            loaded_images,
        );

        let body: Element<'a, Message> = if sp.is_loading {
            render_skeleton_rows(8)
        } else if sp.tracks.is_empty() {
            empty_state("No tracks found in this playlist.")
        } else {
            let rows = virtual_rows(sp.tracks.len(), DETAIL_LIST_TOP, main_scroll, |idx| {
                let track = &sp.tracks[idx];
                let unavailable = track.is_local && !track.is_local_available;
                let info = crate::app::TrackInfo {
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                    duration_ms: track.duration_ms,
                    image_url: track.image_url.clone(),
                    uri: track.uri.clone(),
                    explicit: false,
                };
                track_row(
                    &TrackRowSpec {
                        index: idx + 1,
                        title: &track.title,
                        artist: &track.artist,
                        album: Some(&track.album),
                        duration_ms: track.duration_ms,
                        cover: CoverSlot::Shown(track.image_url.as_deref()),
                        is_current: playing_uri == Some(track.uri.as_str()),
                        dimmed: unavailable,
                        badge: track.is_local.then_some("LOCAL"),
                    },
                    loaded_images,
                    Message::PlayTrack(track.uri.clone()),
                    Message::OpenTrackContextMenu {
                        track: info,
                        from_playlist_id: Some(sp.id.clone()),
                        position: iced::Point::new(450.0, 300.0),
                    },
                )
            });
            Column::new()
                .push(track_table_header(true))
                .push(rows)
                .into()
        };

        let page = Column::new()
            .spacing(20)
            .push(header)
            .push(detail_action_row(
                sp.tracks.first().map(|t| t.uri.clone()),
                accent_tone,
            ))
            .push(body);
        return main_page_frame(page);
    }

    if let Some(sa) = selected_album {
        let header = detail_header(
            "ALBUM",
            &sa.name,
            format!(
                "{} • {} • {} songs",
                sa.artist_name,
                sa.release_date.get(..4).unwrap_or(&sa.release_date),
                sa.tracks.len()
            ),
            sa.image_url.as_deref(),
            Icon::Album,
            loaded_images,
        );

        let body: Element<'a, Message> = if sa.is_loading {
            render_skeleton_rows(8)
        } else if sa.tracks.is_empty() {
            empty_state("No tracks found in this album.")
        } else {
            // Disc headers become rows of their own so every row keeps the same
            // height and the list can be virtualized.
            let has_multidisc = sa.tracks.iter().any(|t| t.disc_number > 1);
            let mut items: Vec<AlbumListItem> = Vec::with_capacity(sa.tracks.len() + 4);
            let mut current_disc = 0;
            for (idx, track) in sa.tracks.iter().enumerate() {
                if has_multidisc && track.disc_number != current_disc {
                    current_disc = track.disc_number;
                    items.push(AlbumListItem::Disc(current_disc));
                }
                items.push(AlbumListItem::Track(idx));
            }

            let rows = virtual_rows(items.len(), DETAIL_LIST_TOP, main_scroll, |row| match items
                [row]
            {
                AlbumListItem::Disc(disc) => Container::new(
                    Row::new()
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(Icon::Album.view_colored(16.0, theme::TEXT_SECONDARY))
                        .push(
                            Text::new(format!("Disc {disc}"))
                                .size(13)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                                .color(theme::TEXT_SECONDARY),
                        ),
                )
                .padding([0, 12])
                .into(),
                AlbumListItem::Track(idx) => {
                    let track = &sa.tracks[idx];
                    let info = crate::app::TrackInfo {
                        title: track.title.clone(),
                        artist: track.artist.clone(),
                        album: sa.name.clone(),
                        duration_ms: track.duration_ms,
                        image_url: sa.image_url.clone(),
                        uri: track.uri.clone(),
                        explicit: false,
                    };
                    track_row(
                        &TrackRowSpec {
                            index: track.track_number as usize,
                            title: &track.title,
                            artist: &track.artist,
                            album: None,
                            duration_ms: track.duration_ms,
                            cover: CoverSlot::Hidden,
                            is_current: playing_uri == Some(track.uri.as_str()),
                            dimmed: false,
                            badge: None,
                        },
                        loaded_images,
                        Message::PlayTrack(track.uri.clone()),
                        Message::OpenTrackContextMenu {
                            track: info,
                            from_playlist_id: None,
                            position: iced::Point::new(450.0, 300.0),
                        },
                    )
                }
            });
            Column::new()
                .push(track_table_header(false))
                .push(rows)
                .into()
        };

        let page = Column::new()
            .spacing(20)
            .push(header)
            .push(detail_action_row(
                sa.tracks.first().map(|t| t.uri.clone()),
                accent_tone,
            ))
            .push(body);
        return main_page_frame(page);
    }

    if let Some(artist_state) = selected_artist {
        return view_artist_detail_page(artist_state, loaded_images, accent_tone, playing_uri);
    }

    let header = Text::new("Welcome back")
        .size(30)
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        })
        .color(theme::TEXT_PRIMARY);

    let quick_grid: Element<'a, Message> = if user_playlists.is_empty() && user_albums.is_empty() {
        render_skeleton_quick_grid()
    } else {
        let mut cards: Vec<Element<'a, Message>> = Vec::with_capacity(6);
        for p in user_playlists.iter().take(3) {
            let card = quick_card_with_image(
                &p.name,
                p.image_url.as_deref(),
                loaded_images,
                Icon::MusicNote,
                Message::SelectPlaylist(p.id.clone()),
            );
            cards.push(
                iced::widget::mouse_area(card)
                    .on_right_press(Message::OpenPlaylistContextMenu {
                        playlist: p.clone(),
                        position: iced::Point::new(300.0, 300.0),
                    })
                    .into(),
            );
        }
        for a in user_albums.iter().take(6 - cards.len()) {
            let card = quick_card_with_image(
                &a.name,
                a.image_url.as_deref(),
                loaded_images,
                Icon::Album,
                Message::SelectAlbum(a.id.clone()),
            );
            cards.push(
                iced::widget::mouse_area(card)
                    .on_right_press(Message::OpenAlbumContextMenu {
                        album: a.clone(),
                        position: iced::Point::new(300.0, 300.0),
                    })
                    .into(),
            );
        }
        let mut grid = Column::new().spacing(12);
        let mut cards = cards.into_iter();
        loop {
            let row_cards: Vec<_> = cards.by_ref().take(3).collect();
            if row_cards.is_empty() {
                break;
            }
            let mut row = Row::new().spacing(12);
            let missing = 3 - row_cards.len();
            for card in row_cards {
                row = row.push(Container::new(card).width(Length::FillPortion(1)));
            }
            for _ in 0..missing {
                row = row.push(Space::new().width(Length::FillPortion(1)));
            }
            grid = grid.push(row);
        }
        grid.into()
    };

    let made_for_you: Element<'a, Message> = if featured_playlists.is_empty() {
        render_skeleton_cards(5)
    } else {
        card_shelf(featured_playlists.len().min(10), move |idx| {
            let p = &featured_playlists[idx];
            let card = media_card_with_image(
                &p.name,
                format!("By {}", p.owner_name),
                p.image_url.as_deref(),
                loaded_images,
                Icon::MusicNote,
                Message::SelectPlaylist(p.id.clone()),
            );
            iced::widget::mouse_area(card)
                .on_right_press(Message::OpenPlaylistContextMenu {
                    playlist: p.clone(),
                    position: iced::Point::new(400.0, 550.0),
                })
                .into()
        })
    };

    let top_tracks: Element<'a, Message> = if user_top_tracks.is_empty() {
        render_skeleton_cards(5)
    } else {
        card_shelf(user_top_tracks.len().min(10), move |idx| {
            let track = &user_top_tracks[idx];
            let card = media_card_with_image(
                &track.title,
                track.artist.clone(),
                track.image_url.as_deref(),
                loaded_images,
                Icon::MusicNote,
                Message::PlayTrack(track.uri.clone()),
            );
            iced::widget::mouse_area(card)
                .on_right_press(Message::OpenTrackContextMenu {
                    track: crate::app::TrackInfo {
                        title: track.title.clone(),
                        artist: track.artist.clone(),
                        album: track.album.clone(),
                        duration_ms: track.duration_ms,
                        image_url: track.image_url.clone(),
                        uri: track.uri.clone(),
                        explicit: track.explicit,
                    },
                    from_playlist_id: None,
                    position: iced::Point::new(400.0, 350.0),
                })
                .into()
        })
    };

    let album_shelf = |albums: &'a [crate::api::album::AlbumSummary]| -> Element<'a, Message> {
        card_shelf(albums.len().min(10), move |idx| {
            let a = &albums[idx];
            let card = media_card_with_image(
                &a.name,
                a.artist_name.clone(),
                a.image_url.as_deref(),
                loaded_images,
                Icon::Album,
                Message::SelectAlbum(a.id.clone()),
            );
            iced::widget::mouse_area(card)
                .on_right_press(Message::OpenAlbumContextMenu {
                    album: a.clone(),
                    position: iced::Point::new(400.0, 450.0),
                })
                .into()
        })
    };

    let mut page = Column::new()
        .spacing(16)
        .push(header)
        .push(quick_grid)
        .push(Space::new().height(Length::Fixed(8.0)));

    if !featured_playlists.is_empty() {
        page = page.push(section_title("Made For You")).push(made_for_you);
    }
    page = page.push(section_title("Your Top Tracks")).push(top_tracks);
    if !featured_albums.is_empty() {
        page = page
            .push(section_title("New Releases"))
            .push(album_shelf(featured_albums));
    }
    page = page
        .push(section_title("Saved Albums"))
        .push(if user_albums.is_empty() {
            render_skeleton_cards(5)
        } else {
            album_shelf(user_albums)
        });

    main_page_frame(page)
}

fn section_title(label: &str) -> Element<'_, Message> {
    Container::new(
        Text::new(label)
            .size(22)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .color(theme::TEXT_PRIMARY),
    )
    .padding(iced::Padding {
        top: 8.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    })
    .into()
}

/// Slim banner explaining why audio isn't available yet (connecting, waiting for
/// the one-time pairing approval, or failed). Hidden once the session is ready.
fn view_playback_link_banner<'a>(
    link: &'a crate::app::PlaybackLink,
) -> Option<Element<'a, Message>> {
    use crate::app::PlaybackLink;

    let action = |label: &'static str, message: Message| {
        Button::new(
            Text::new(label)
                .size(13)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(Color::BLACK),
        )
        .padding([6, 16])
        .on_press(message)
        .style(|_t, status| iced::widget::button::Style {
            background: Some(Background::Color(match status {
                iced::widget::button::Status::Hovered => theme::ACCENT_HOVER,
                _ => theme::TEXT_PRIMARY,
            })),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        })
    };

    let (icon, text, button): (Icon, String, Option<Element<'a, Message>>) = match link {
        PlaybackLink::Ready => return None,
        PlaybackLink::Connecting => (
            Icon::Volume,
            "Connecting to Spotify audio…".to_string(),
            None,
        ),
        PlaybackLink::AwaitingApproval { user_code, .. } => (
            Icon::Volume,
            format!(
                "One-time setup: approve Spotifust on spotify.com/pair with code {user_code} to enable playback."
            ),
            Some(action("Open approval page", Message::OpenPlaybackPairingUrl).into()),
        ),
        PlaybackLink::Failed(reason) => (
            Icon::VolumeMute,
            format!("Audio unavailable: {reason}"),
            Some(action("Retry", Message::RetryPlaybackConnection).into()),
        ),
    };

    let mut row = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(icon.view_colored(16.0, theme::ACCENT))
        .push(single_line(text, 13.0, theme::TEXT_PRIMARY, false));
    if let Some(button) = button {
        row = row.push(button);
    }

    Some(
        Container::new(
            Container::new(row)
                .padding([8, 16])
                .width(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(theme::SURFACE_ELEVATED)),
                    border: Border {
                        radius: theme::RADIUS_MD.into(),
                        color: theme::BORDER_SUBTLE,
                        width: 1.0,
                    },
                    ..Default::default()
                }),
        )
        .padding(iced::Padding {
            top: 0.0,
            right: 8.0,
            bottom: 8.0,
            left: 8.0,
        })
        .into(),
    )
}

#[derive(Clone, Copy)]
enum AlbumListItem {
    Disc(u32),
    Track(usize),
}

/// Whether a track row shows a cover column (and which art, if any).
#[derive(Clone, Copy)]
enum CoverSlot<'s> {
    Hidden,
    Shown(Option<&'s str>),
}

struct TrackRowSpec<'s> {
    index: usize,
    title: &'s str,
    artist: &'s str,
    album: Option<&'s str>,
    duration_ms: u32,
    cover: CoverSlot<'s>,
    is_current: bool,
    dimmed: bool,
    badge: Option<&'static str>,
}

/// Wraps a detail page in the main scrollable, reporting the scroll window so long
/// lists can be virtualized, and with a stable id so navigation can reset it.
fn main_page_frame<'a>(page: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let scrollable = thin_scrollable(Container::new(page).padding(iced::Padding {
        top: 0.0,
        right: 16.0,
        bottom: 24.0,
        left: 0.0,
    }))
    .id(MAIN_SCROLL_ID)
    .on_scroll(|viewport| {
        Message::MainContentScrolled(crate::app::ScrollWindow {
            offset_y: viewport.absolute_offset().y,
            viewport_height: viewport.bounds().height,
        })
    })
    .width(Length::Fill)
    .height(Length::Fill);

    Container::new(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            top: 24.0,
            right: 8.0,
            bottom: 0.0,
            left: 24.0,
        })
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_MAIN)),
            border: Border {
                radius: theme::RADIUS_LG.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn empty_state(message: &str) -> Element<'_, Message> {
    Container::new(Text::new(message).size(15).color(theme::TEXT_SECONDARY))
        .padding(32)
        .into()
}

fn format_total_duration(total_ms: u64) -> String {
    let total_min = total_ms / 60_000;
    if total_min >= 60 {
        format!("{} h {} min", total_min / 60, total_min % 60)
    } else {
        format!("{total_min} min")
    }
}

/// Big cover + title block at the top of playlist / album pages. Fixed 230px tall
/// so [`DETAIL_LIST_TOP`] stays exact.
fn detail_header<'a>(
    kind: &'static str,
    title: &'a str,
    subtitle: String,
    cover_url: Option<&str>,
    fallback: Icon,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let info = Column::new()
        .spacing(8)
        .width(Length::Fill)
        .push(
            Text::new(kind)
                .size(11)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_SECONDARY),
        )
        .push(
            Container::new(
                Text::new(title)
                    .size(40)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .max_height(110.0)
            .clip(true),
        )
        .push(single_line(subtitle, 14.0, theme::TEXT_SECONDARY, false));

    Row::new()
        .spacing(24)
        .align_y(Alignment::End)
        .height(Length::Fixed(230.0))
        .push(view_image_or_icon(
            cover_url,
            loaded_images,
            fallback,
            230.0,
            theme::RADIUS_LG,
        ))
        .push(info)
        .into()
}

/// Play button row (always 56px tall, even when empty).
fn detail_action_row<'a>(
    first_uri: Option<String>,
    accent_tone: crate::ui::theme::AccentTone,
) -> Element<'a, Message> {
    let mut row = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .height(Length::Fixed(56.0));
    if let Some(uri) = first_uri {
        row = row.push(big_play_button(Message::PlayTrack(uri), accent_tone));
    }
    row.into()
}

fn big_play_button<'a>(
    on_press: Message,
    accent_tone: crate::ui::theme::AccentTone,
) -> Element<'a, Message> {
    Button::new(
        Container::new(Icon::Play.view_colored(24.0, Color::BLACK))
            .width(Length::Fixed(56.0))
            .height(Length::Fixed(56.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(on_press)
    .style(move |_t, status| {
        let background = match status {
            iced::widget::button::Status::Hovered => accent_tone.hover(),
            iced::widget::button::Status::Pressed => theme::SPOTIFY_GREEN_PRESSED,
            _ => accent_tone.primary(),
        };
        iced::widget::button::Style {
            background: Some(Background::Color(background)),
            border: Border {
                radius: 28.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

/// Column captions above a track list (40px incl. divider, see [`DETAIL_LIST_TOP`]).
fn track_table_header<'a>(with_album: bool) -> Element<'a, Message> {
    let caption = |label: &'static str| Text::new(label).size(12).color(theme::TEXT_SECONDARY);
    let mut row = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .push(
            Container::new(caption("#"))
                .width(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Right),
        )
        .push(Container::new(caption("Title")).width(Length::FillPortion(5)));
    if with_album {
        row = row.push(Container::new(caption("Album")).width(Length::FillPortion(3)));
    }
    row = row.push(
        Container::new(Icon::Clock.view_colored(14.0, theme::TEXT_SECONDARY))
            .width(Length::Fixed(52.0))
            .align_x(iced::alignment::Horizontal::Right),
    );

    Container::new(row)
        .height(Length::Fixed(40.0))
        .padding([0, 12])
        .align_y(iced::alignment::Vertical::Center)
        .style(|_theme| container::Style {
            border: Border {
                color: theme::BORDER_SUBTLE,
                width: 0.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// One Spotify-style track row: number, optional cover, title/artist, album, duration.
#[allow(clippy::too_many_lines)]
fn track_row<'a>(
    spec: &TrackRowSpec<'_>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    on_press: Message,
    on_right_press: Message,
) -> Element<'a, Message> {
    let title_color = if spec.is_current {
        theme::ACCENT
    } else if spec.dimmed {
        theme::TEXT_MUTED
    } else {
        theme::TEXT_PRIMARY
    };

    let index_cell: Element<'a, Message> = if spec.is_current {
        Icon::Volume.view_colored(14.0, theme::ACCENT)
    } else {
        Text::new(spec.index.to_string())
            .size(14)
            .color(theme::TEXT_SECONDARY)
            .into()
    };

    let mut title_line = Row::new()
        .spacing(6)
        .align_y(Alignment::Center)
        .push(single_line(spec.title.to_owned(), 15.0, title_color, false));
    if let Some(label) = spec.badge {
        title_line = title_line.push(
            Container::new(
                Text::new(label)
                    .size(9)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_SECONDARY),
            )
            .padding([1, 5])
            .style(|_theme| container::Style {
                border: Border {
                    radius: 3.0.into(),
                    color: theme::BORDER_STRONG,
                    width: 1.0,
                },
                ..Default::default()
            }),
        );
    }

    let title_block = Column::new()
        .spacing(2)
        .width(Length::Fill)
        .push(title_line)
        .push(single_line(
            spec.artist.to_owned(),
            13.0,
            theme::TEXT_SECONDARY,
            false,
        ));

    let mut title_cell = Row::new().spacing(12).align_y(Alignment::Center);
    if let CoverSlot::Shown(cover) = spec.cover {
        title_cell = title_cell.push(view_image_or_icon(
            cover,
            loaded_images,
            Icon::MusicNote,
            40.0,
            theme::RADIUS_SM,
        ));
    }
    title_cell = title_cell.push(title_block);

    let mut row = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .push(
            Container::new(index_cell)
                .width(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Right),
        )
        .push(Container::new(title_cell).width(Length::FillPortion(5)));
    if let Some(album) = spec.album {
        row = row.push(
            Container::new(single_line(
                album.to_owned(),
                13.0,
                theme::TEXT_SECONDARY,
                false,
            ))
            .width(Length::FillPortion(3)),
        );
    }
    row = row.push(
        Container::new(
            Text::new(format_duration(spec.duration_ms))
                .size(13)
                .color(theme::TEXT_SECONDARY),
        )
        .width(Length::Fixed(52.0))
        .align_x(iced::alignment::Horizontal::Right),
    );

    let button = Button::new(
        Container::new(row)
            .padding([0, 12])
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .width(Length::Fill)
    .height(Length::Fixed(TRACK_ROW_HEIGHT - 4.0))
    .on_press(on_press)
    .style(|_theme, status| iced::widget::button::Style {
        background: match status {
            iced::widget::button::Status::Hovered => Some(Background::Color(theme::SURFACE_HOVER)),
            iced::widget::button::Status::Pressed => Some(Background::Color(theme::SURFACE_ACTIVE)),
            _ => None,
        },
        border: Border {
            radius: theme::RADIUS_SM.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    iced::widget::mouse_area(button)
        .on_right_press(on_right_press)
        .into()
}

#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn view_right_panel<'a>(
    active_tab: Option<RightPanelTab>,
    width: f32,
    playback: &'a PlaybackState,
    user_queue: &'a [crate::app::TrackInfo],
    context_queue: &'a [crate::app::TrackInfo],
    context_index: usize,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    current_lyrics: Option<&'a crate::api::lyrics::LyricsData>,
    is_loading_lyrics: bool,
    current_artist_bio: Option<&'a crate::api::artist::ArtistBio>,
    is_loading_artist_bio: bool,
) -> Element<'a, Message> {
    let Some(tab) = active_tab else {
        return Container::new(Space::new()).into();
    };

    let title_text = match tab {
        RightPanelTab::NowPlaying => "Now Playing",
        RightPanelTab::Queue => "Queue",
        RightPanelTab::Lyrics => "Lyrics",
    };

    let header = Row::new()
        .align_y(Alignment::Center)
        .push(
            Text::new(title_text)
                .size(18)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .push(Space::new().width(Length::Fill))
        .push(icon_button_circle(Icon::X, Message::ToggleRightPanel(tab)));

    let body: Element<'a, Message> = match tab {
        RightPanelTab::Lyrics => {
            let notice = |title: &'a str, detail: &'a str| -> Element<'a, Message> {
                Container::new(
                    Column::new()
                        .spacing(8)
                        .align_x(Alignment::Center)
                        .push(
                            Text::new(title)
                                .size(16)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                                .color(theme::TEXT_PRIMARY),
                        )
                        .push(
                            Text::new(detail)
                                .size(13)
                                .color(theme::TEXT_SECONDARY)
                                .align_x(iced::alignment::Horizontal::Center),
                        ),
                )
                .padding(24)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .into()
            };

            if playback.current_track.is_none() {
                notice("No lyrics yet", "Play a song to see its lyrics here.")
            } else if is_loading_lyrics {
                notice("Loading lyrics…", "Looking this song up on LRCLIB.")
            } else if let Some(lyrics) = current_lyrics.filter(|l| !l.lines.is_empty()) {
                let active_idx = if lyrics.synced {
                    lyrics
                        .lines
                        .iter()
                        .rposition(|l| l.timestamp_ms <= playback.progress_ms)
                } else {
                    None
                };

                let mut lines_col = Column::new().spacing(6).width(Length::Fill);
                for (idx, line) in lyrics.lines.iter().enumerate() {
                    let is_active = Some(idx) == active_idx;
                    let is_past = active_idx.is_some_and(|a| idx < a);
                    let color = if !lyrics.synced || is_active {
                        theme::TEXT_PRIMARY
                    } else if is_past {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_MUTED
                    };
                    // Same size for every line: growing the active line reflowed the
                    // whole column on every tick, which made the text jump around.
                    let text: &str = if line.text.is_empty() {
                        "♪"
                    } else {
                        &line.text
                    };
                    let label = Text::new(text)
                        .size(18)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(color);

                    if lyrics.synced {
                        lines_col = lines_col.push(
                            Button::new(label)
                                .padding([4, 8])
                                .width(Length::Fill)
                                .on_press(Message::SeekToMs(line.timestamp_ms))
                                .style(|_theme, status| iced::widget::button::Style {
                                    background: match status {
                                        iced::widget::button::Status::Hovered => {
                                            Some(Background::Color(theme::SURFACE_HOVER))
                                        }
                                        _ => None,
                                    },
                                    border: Border {
                                        radius: theme::RADIUS_SM.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                        );
                    } else {
                        lines_col = lines_col.push(Container::new(label).padding([4, 8]));
                    }
                }

                let mut col = Column::new().spacing(12);
                if !lyrics.synced {
                    col = col.push(
                        Text::new("These lyrics aren't synced to the song yet.")
                            .size(12)
                            .color(theme::TEXT_MUTED),
                    );
                }
                col.push(lines_col)
                    .push(
                        Text::new("Lyrics provided by LRCLIB")
                            .size(11)
                            .color(theme::TEXT_MUTED),
                    )
                    .into()
            } else {
                notice(
                    "No lyrics available",
                    "We couldn't find lyrics for this song.",
                )
            }
        }
        RightPanelTab::NowPlaying => {
            let (track_title_str, artist_name_str, img_url) =
                if let Some(track) = &playback.current_track {
                    (
                        track.title.as_str(),
                        track.artist.as_str(),
                        track.image_url.as_deref(),
                    )
                } else {
                    ("No track playing", "Select a song to start playback", None)
                };

            let art_size = (width - 44.0).clamp(120.0, 360.0);
            let art = view_image_or_icon(
                img_url,
                loaded_images,
                Icon::Album,
                art_size,
                theme::RADIUS_LG,
            );

            let mut artist_card_col = Column::new().spacing(8).push(
                Text::new("About the artist")
                    .size(14)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            );

            if playback.current_track.is_none() {
                artist_card_col = artist_card_col.push(
                    Text::new("Play something to learn about the artist.")
                        .size(12)
                        .color(theme::TEXT_MUTED),
                );
            } else if is_loading_artist_bio {
                artist_card_col = artist_card_col.push(
                    Text::new("Loading artist biography…")
                        .size(12)
                        .color(theme::TEXT_MUTED),
                );
            } else if let Some(bio) = current_artist_bio {
                artist_card_col = artist_card_col.push(
                    Text::new(&bio.title)
                        .size(16)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(theme::TEXT_PRIMARY),
                );
                if let Some(desc) = &bio.description {
                    artist_card_col =
                        artist_card_col.push(Text::new(desc).size(12).color(theme::ACCENT));
                }
                artist_card_col = artist_card_col
                    .push(
                        Text::new(&bio.extract)
                            .size(13)
                            .line_height(1.4)
                            .color(theme::TEXT_SECONDARY),
                    )
                    .push(
                        Text::new("Source: Wikipedia")
                            .size(11)
                            .color(theme::TEXT_MUTED),
                    );
            } else {
                artist_card_col = artist_card_col.push(
                    Text::new("No biography found for this artist.")
                        .size(12)
                        .color(theme::TEXT_MUTED),
                );
            }

            let artist_card = Container::new(artist_card_col)
                .padding(16)
                .width(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(theme::SURFACE_CARD)),
                    border: Border {
                        radius: theme::RADIUS_MD.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });

            Column::new()
                .spacing(14)
                .push(art)
                .push(
                    Column::new()
                        .spacing(4)
                        .push(single_line(
                            track_title_str,
                            20.0,
                            theme::TEXT_PRIMARY,
                            true,
                        ))
                        .push(single_line(
                            artist_name_str,
                            14.0,
                            theme::TEXT_SECONDARY,
                            false,
                        )),
                )
                .push(artist_card)
                .into()
        }
        RightPanelTab::Queue => {
            let current_header = Text::new("Now Playing")
                .size(14)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY);

            let current_item: Element<'a, Message> = if let Some(track) = &playback.current_track {
                sidebar_item_with_image(
                    &track.title,
                    &track.artist,
                    track.image_url.as_deref(),
                    loaded_images,
                    Icon::MusicNote,
                    true,
                    Message::TogglePlayback,
                )
            } else {
                Container::new(
                    Text::new("No track playing")
                        .size(12)
                        .color(theme::TEXT_SECONDARY),
                )
                .padding(8)
                .into()
            };

            let next_user_header_row = Row::new()
                .align_y(Alignment::Center)
                .push(
                    Text::new("Next in Queue")
                        .size(14)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(theme::TEXT_PRIMARY),
                )
                .push(Space::new().width(Length::Fill));

            let user_queue_section: Element<'a, Message> = if user_queue.is_empty() {
                Space::new().height(Length::Fixed(0.0)).into()
            } else {
                let user_header = next_user_header_row.push(
                    Button::new(
                        Text::new("Clear queue")
                            .size(11)
                            .color(theme::TEXT_SECONDARY),
                    )
                    .padding([4, 8])
                    .on_press(Message::ClearQueue)
                    .style(|_t, _s| iced::widget::button::Style {
                        background: Some(Background::Color(Color::TRANSPARENT)),
                        ..Default::default()
                    }),
                );

                let mut col = Column::new().spacing(6);
                let queue_len = user_queue.len();

                for (idx, track) in user_queue.iter().enumerate() {
                    let t_title = track.title.clone();
                    let t_artist = track.artist.clone();
                    let cover = view_image_or_icon(
                        track.image_url.as_deref(),
                        loaded_images,
                        Icon::MusicNote,
                        40.0,
                        theme::RADIUS_SM,
                    );

                    let play_btn = Button::new(Icon::Play.view_colored(14.0, theme::ACCENT))
                        .padding(4)
                        .on_press(Message::PlayQueueIndex(idx))
                        .style(|_t, _s| iced::widget::button::Style::default());

                    let up_btn: Element<'a, Message> = if idx > 0 {
                        Button::new(Icon::ChevronUp.view_colored(14.0, theme::TEXT_SECONDARY))
                            .padding(2)
                            .on_press(Message::MoveQueueItemUp(idx))
                            .style(|_t, _s| iced::widget::button::Style::default())
                            .into()
                    } else {
                        Space::new().width(Length::Fixed(18.0)).into()
                    };

                    let down_btn: Element<'a, Message> = if idx + 1 < queue_len {
                        Button::new(Icon::ChevronDown.view_colored(14.0, theme::TEXT_SECONDARY))
                            .padding(2)
                            .on_press(Message::MoveQueueItemDown(idx))
                            .style(|_t, _s| iced::widget::button::Style::default())
                            .into()
                    } else {
                        Space::new().width(Length::Fixed(18.0)).into()
                    };

                    let remove_btn =
                        Button::new(Icon::Trash.view_colored(14.0, Color::from_rgb(0.9, 0.3, 0.3)))
                            .padding(4)
                            .on_press(Message::RemoveFromQueue(idx))
                            .style(|_t, _s| iced::widget::button::Style::default());

                    let item_row = Container::new(
                        Row::new()
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .push(play_btn)
                            .push(cover)
                            .push(
                                Column::new()
                                    .spacing(2)
                                    .push(Text::new(t_title).size(13).color(theme::TEXT_PRIMARY))
                                    .push(Text::new(t_artist).size(11).color(theme::TEXT_SECONDARY))
                                    .width(Length::Fill),
                            )
                            .push(up_btn)
                            .push(down_btn)
                            .push(remove_btn),
                    )
                    .padding([6, 10])
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_CARD)),
                        border: Border {
                            radius: theme::RADIUS_SM.into(),
                            color: theme::BORDER_SUBTLE,
                            width: 1.0,
                        },
                        ..Default::default()
                    });

                    col = col.push(item_row);
                }

                Column::new().spacing(8).push(user_header).push(col).into()
            };

            let context_header = Text::new("Next from Context")
                .size(14)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY);

            let context_queue_section: Element<'a, Message> =
                if context_queue.is_empty() || context_index + 1 >= context_queue.len() {
                    if user_queue.is_empty() {
                        Container::new(
                            Column::new()
                                .spacing(8)
                                .align_x(Alignment::Center)
                                .push(Icon::Queue.view_colored(32.0, theme::TEXT_TERTIARY))
                                .push(
                                    Text::new("Queue is empty")
                                        .size(13)
                                        .font(iced::Font {
                                            weight: iced::font::Weight::Bold,
                                            ..Default::default()
                                        })
                                        .color(theme::TEXT_SECONDARY),
                                )
                                .push(
                                    Text::new("Add tracks by right-clicking any song.")
                                        .size(11)
                                        .color(theme::TEXT_TERTIARY),
                                ),
                        )
                        .padding(24)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center)
                        .into()
                    } else {
                        Space::new().height(Length::Fixed(0.0)).into()
                    }
                } else {
                    let mut col = Column::new().spacing(6);
                    for (_idx, track) in context_queue
                        .iter()
                        .enumerate()
                        .skip(context_index + 1)
                        .take(15)
                    {
                        let t_uri = track.uri.clone();
                        let item = sidebar_item_with_image(
                            &track.title,
                            &track.artist,
                            track.image_url.as_deref(),
                            loaded_images,
                            Icon::MusicNote,
                            false,
                            Message::PlayTrack(t_uri),
                        );
                        col = col.push(item);
                    }
                    Column::new()
                        .spacing(8)
                        .push(context_header)
                        .push(col)
                        .into()
                };

            Column::new()
                .spacing(16)
                .push(current_header)
                .push(current_item)
                .push(user_queue_section)
                .push(context_queue_section)
                .into()
        }
    };

    let content = Column::new().spacing(16).push(header).push(
        thin_scrollable(Container::new(body).padding(iced::Padding {
            top: 0.0,
            right: 10.0,
            bottom: 16.0,
            left: 0.0,
        }))
        .id(RIGHT_PANEL_SCROLL_ID)
        .height(Length::Fill),
    );

    Container::new(content)
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .padding(16)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_MAIN)),
            border: Border {
                radius: theme::RADIUS_LG.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn view_playback_bar<'a>(
    playback: &'a PlaybackState,
    active_right_panel: Option<RightPanelTab>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let (track_name, artist_name, image_url) = if let Some(track) = &playback.current_track {
        (
            track.title.as_str(),
            track.artist.as_str(),
            track.image_url.as_deref(),
        )
    } else {
        ("No track playing", "Spotifust", None)
    };

    let track_cover = view_image_or_icon(
        image_url,
        loaded_images,
        Icon::MusicNote,
        56.0,
        theme::RADIUS_MD,
    );

    let track_info = Row::new()
        .align_y(Alignment::Center)
        .spacing(12)
        .push(track_cover)
        .push(
            Column::new()
                .spacing(3)
                .width(Length::Fill)
                .push(single_line(track_name, 14.0, theme::TEXT_PRIMARY, true))
                .push(single_line(artist_name, 12.0, theme::TEXT_SECONDARY, false)),
        );

    let play_pause_icon = if playback.is_playing {
        Icon::Pause
    } else {
        Icon::Play
    };

    let controls = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .push(icon_button_plain_active(
            Icon::Shuffle,
            Message::ToggleShuffle,
            playback.is_shuffled,
        ))
        .push(icon_button_plain(Icon::SkipPrev, Message::SkipPrev))
        .push(icon_button_circle_accent(
            play_pause_icon,
            Message::TogglePlayback,
        ))
        .push(icon_button_plain(Icon::SkipNext, Message::SkipNext))
        .push(icon_button_plain_active(
            Icon::Repeat,
            Message::ToggleRepeat,
            playback.repeat_mode != crate::app::RepeatMode::Off,
        ));

    let duration_ms = playback.current_track.as_ref().map_or(0, |t| t.duration_ms);
    let progress_percent = calculate_progress_ratio(playback.progress_ms, duration_ms);

    let seek_bar = slider(0.0..=1.0, progress_percent, Message::SeekTo)
        .step(0.001_f32)
        .width(Length::Fill)
        .style(|_theme, status| {
            let base = iced::widget::slider::Style {
                rail: iced::widget::slider::Rail {
                    backgrounds: (
                        Background::Color(theme::ACCENT),
                        Background::Color(theme::SURFACE_CARD),
                    ),
                    width: 4.0,
                    border: Border {
                        radius: theme::RADIUS_PILL.into(),
                        ..Default::default()
                    },
                },
                handle: iced::widget::slider::Handle {
                    shape: iced::widget::slider::HandleShape::Circle { radius: 6.0 },
                    background: Background::Color(theme::TEXT_PRIMARY),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            };
            match status {
                iced::widget::slider::Status::Hovered | iced::widget::slider::Status::Dragged => {
                    iced::widget::slider::Style {
                        handle: iced::widget::slider::Handle {
                            shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 },
                            background: Background::Color(theme::TEXT_PRIMARY),
                            border_width: 0.0,
                            border_color: Color::TRANSPARENT,
                        },
                        ..base
                    }
                }
                iced::widget::slider::Status::Active => base,
            }
        });

    let current_time_str = format_duration(playback.progress_ms);
    let total_time_str = format_duration(duration_ms);

    let progress_row = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(
            Text::new(current_time_str)
                .size(11)
                .color(theme::TEXT_SECONDARY),
        )
        .push(seek_bar)
        .push(
            Text::new(total_time_str)
                .size(11)
                .color(theme::TEXT_SECONDARY),
        );

    let center_controls = Column::new()
        .spacing(6)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .max_width(640.0)
        .push(controls)
        .push(progress_row);

    let now_playing_active = active_right_panel == Some(RightPanelTab::NowPlaying);
    let lyrics_active = active_right_panel == Some(RightPanelTab::Lyrics);
    let queue_active = active_right_panel == Some(RightPanelTab::Queue);

    let now_playing_btn = icon_button_plain_active(
        Icon::Album,
        Message::ToggleRightPanel(RightPanelTab::NowPlaying),
        now_playing_active,
    );
    let lyrics_btn = icon_button_plain_active(
        Icon::MusicNote,
        Message::ToggleRightPanel(RightPanelTab::Lyrics),
        lyrics_active,
    );
    let queue_btn = icon_button_plain_active(
        Icon::Queue,
        Message::ToggleRightPanel(RightPanelTab::Queue),
        queue_active,
    );

    let volume_slider = slider(0.0..=1.0, playback.volume, Message::VolumeChanged)
        .step(0.01_f32)
        .width(Length::Fixed(90.0))
        .style(|_theme, status| {
            let base = iced::widget::slider::Style {
                rail: iced::widget::slider::Rail {
                    backgrounds: (
                        Background::Color(theme::TEXT_PRIMARY),
                        Background::Color(theme::SURFACE_CARD),
                    ),
                    width: 4.0,
                    border: Border {
                        radius: theme::RADIUS_PILL.into(),
                        ..Default::default()
                    },
                },
                handle: iced::widget::slider::Handle {
                    shape: iced::widget::slider::HandleShape::Circle { radius: 5.0 },
                    background: Background::Color(theme::TEXT_PRIMARY),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            };
            match status {
                iced::widget::slider::Status::Hovered | iced::widget::slider::Status::Dragged => {
                    iced::widget::slider::Style {
                        handle: iced::widget::slider::Handle {
                            shape: iced::widget::slider::HandleShape::Circle { radius: 7.0 },
                            background: Background::Color(theme::TEXT_PRIMARY),
                            border_width: 0.0,
                            border_color: Color::TRANSPARENT,
                        },
                        ..base
                    }
                }
                iced::widget::slider::Status::Active => base,
            }
        });

    let volume_icon = if playback.is_muted || playback.volume == 0.0 {
        Icon::VolumeMute
    } else {
        Icon::Volume
    };

    let volume_btn = icon_button_plain(volume_icon, Message::ToggleMute);

    let volume_controls = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(volume_btn)
        .push(volume_slider);

    let right_utility_controls = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(now_playing_btn)
        .push(lyrics_btn)
        .push(queue_btn)
        .push(volume_controls);

    // Proportional columns instead of fixed 300/500/300 px, which overlapped on
    // windows narrower than ~1150 px.
    Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .spacing(16)
            .push(Container::new(track_info).width(Length::FillPortion(3)))
            .push(
                Container::new(center_controls)
                    .width(Length::FillPortion(4))
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .push(
                Container::new(right_utility_controls)
                    .width(Length::FillPortion(3))
                    .align_x(iced::alignment::Horizontal::Right),
            ),
    )
    .width(Length::Fill)
    .height(Length::Fixed(84.0))
    .padding(iced::Padding {
        top: 8.0,
        right: 20.0,
        bottom: 8.0,
        left: 20.0,
    })
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::BG_BASE)),
        border: Border {
            color: theme::BORDER_SUBTLE,
            width: 1.0,
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn view_drag_handle<'a>(is_left: bool) -> Element<'a, Message> {
    let start_msg = if is_left {
        Message::StartSidebarDrag
    } else {
        Message::StartRightPanelDrag
    };

    let inner_bar = Container::new(Space::new())
        .width(Length::Fixed(2.0))
        .height(Length::Fixed(40.0))
        .style(|_theme| container::Style {
            background: Some(Background::Color(theme::BORDER_SUBTLE)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let container_widget = Container::new(inner_bar)
        .width(Length::Fixed(8.0))
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        });

    iced::widget::mouse_area(container_widget)
        .on_press(start_msg)
        .interaction(iced::mouse::Interaction::ResizingHorizontally)
        .into()
}

fn icon_button_circle<'a>(icon: Icon, message: Message) -> Element<'a, Message> {
    Button::new(
        Container::new(icon.view_colored(16.0, theme::TEXT_SECONDARY))
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(32.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(message)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                ..base
            },
            _ => base,
        }
    })
    .into()
}

fn icon_button_circle_top_bar<'a>(icon: Icon, message: Message) -> Element<'a, Message> {
    Button::new(
        Container::new(icon.view_colored(18.0, theme::TEXT_SECONDARY))
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(message)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                color: theme::BORDER_SUBTLE,
                width: 1.0,
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    color: theme::TEXT_SECONDARY,
                    width: 1.0,
                },
                ..base
            },
            _ => base,
        }
    })
    .into()
}

fn icon_button_circle_disabled_top_bar<'a>(
    icon: Icon,
    message: Message,
    enabled: bool,
) -> Element<'a, Message> {
    if enabled {
        icon_button_circle_top_bar(icon, message)
    } else {
        Container::new(icon.view_colored(18.0, theme::TEXT_MUTED))
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(|_theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.12, 0.12, 0.12, 0.5))),
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    color: theme::BORDER_SUBTLE,
                    width: 1.0,
                },
                ..Default::default()
            })
            .into()
    }
}

fn icon_button_circle_active<'a>(
    icon: Icon,
    message: Message,
    active: bool,
) -> Element<'a, Message> {
    let icon_color = if active {
        Color::WHITE
    } else {
        theme::TEXT_SECONDARY
    };
    let bg_color = if active {
        theme::SURFACE_ACTIVE
    } else {
        theme::SURFACE_CARD
    };

    Button::new(
        Container::new(icon.view_colored(18.0, icon_color))
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(message)
    .style(move |_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(bg_color)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                ..base
            },
            _ => base,
        }
    })
    .into()
}

fn icon_button_circle_accent<'a>(icon: Icon, message: Message) -> Element<'a, Message> {
    Button::new(
        Container::new(icon.view_colored(18.0, Color::BLACK))
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(message)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(theme::ACCENT)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::ACCENT_HOVER)),
                ..base
            },
            _ => base,
        }
    })
    .into()
}

fn icon_button_plain<'a>(icon: Icon, message: Message) -> Element<'a, Message> {
    Button::new(
        Container::new(icon.view_colored(18.0, theme::TEXT_SECONDARY))
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(32.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(message)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    ..Default::default()
                },
                ..base
            },
            _ => base,
        }
    })
    .into()
}

fn icon_button_plain_active<'a>(
    icon: Icon,
    message: Message,
    active: bool,
) -> Element<'a, Message> {
    let color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    };

    Button::new(
        Container::new(icon.view_colored(18.0, color))
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(32.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(message)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    ..Default::default()
                },
                ..base
            },
            _ => base,
        }
    })
    .into()
}

fn filter_chip<'a>(label: &'static str, active: bool, on_press: Message) -> Element<'a, Message> {
    let bg = if active {
        theme::TEXT_PRIMARY
    } else {
        theme::SURFACE_CARD
    };
    let fg = if active {
        Color::BLACK
    } else {
        theme::TEXT_PRIMARY
    };

    Button::new(
        Container::new(
            Text::new(label)
                .size(13)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(fg),
        )
        .padding([6, 14])
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(on_press)
    .style(move |_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(if active {
                    theme::ACCENT_HOVER
                } else {
                    theme::SURFACE_HOVER
                })),
                ..base
            },
            _ => base,
        }
    })
    .into()
}

fn sidebar_item_with_image<'a>(
    title: &str,
    subtitle: &str,
    image_url: Option<&str>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    fallback_icon: Icon,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let icon_box = view_image_or_icon(
        image_url,
        loaded_images,
        fallback_icon,
        44.0,
        theme::RADIUS_MD,
    );

    let title_color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_PRIMARY
    };

    let details = Column::new()
        .spacing(3)
        .width(Length::Fill)
        .push(single_line(title.to_owned(), 14.0, title_color, true))
        .push(single_line(
            subtitle.to_owned(),
            12.0,
            theme::TEXT_SECONDARY,
            false,
        ));

    let content = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(icon_box)
        .push(details);

    Button::new(content)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fixed(60.0))
        .on_press(on_press)
        .style(move |_theme, status| {
            let background = match status {
                iced::widget::button::Status::Hovered => theme::SURFACE_HOVER,
                iced::widget::button::Status::Pressed => theme::SURFACE_ACTIVE,
                _ if active => theme::SURFACE_ACTIVE,
                _ => Color::TRANSPARENT,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(background)),
                border: Border {
                    radius: theme::RADIUS_MD.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn quick_card_with_image<'a>(
    title: &str,
    image_url: Option<&str>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    fallback_icon: Icon,
    on_press: Message,
) -> Element<'a, Message> {
    let cover = view_image_or_icon(
        image_url,
        loaded_images,
        fallback_icon,
        56.0,
        theme::RADIUS_SM,
    );

    let content = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(cover)
        .push(
            Container::new(single_line(
                title.to_owned(),
                14.0,
                theme::TEXT_PRIMARY,
                true,
            ))
            .padding(iced::Padding {
                top: 0.0,
                right: 12.0,
                bottom: 0.0,
                left: 0.0,
            }),
        );

    Button::new(content)
        .padding(0)
        .width(Length::Fill)
        .height(Length::Fixed(56.0))
        .on_press(on_press)
        .style(|_theme, status| {
            let background = match status {
                iced::widget::button::Status::Hovered => theme::SURFACE_HOVER,
                iced::widget::button::Status::Pressed => theme::SURFACE_ACTIVE,
                _ => theme::SURFACE_CARD,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(background)),
                border: Border {
                    radius: theme::RADIUS_MD.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn media_card_with_image<'a>(
    title: &str,
    subtitle: String,
    image_url: Option<&str>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    fallback_icon: Icon,
    on_press: Message,
) -> Element<'a, Message> {
    let cover = view_image_or_icon(
        image_url,
        loaded_images,
        fallback_icon,
        MEDIA_CARD_WIDTH - 24.0,
        theme::RADIUS_MD,
    );

    let text_col = Column::new()
        .spacing(4)
        .push(single_line(
            title.to_owned(),
            15.0,
            theme::TEXT_PRIMARY,
            true,
        ))
        .push(single_line(subtitle, 13.0, theme::TEXT_SECONDARY, false));

    let content = Column::new().spacing(12).push(cover).push(text_col);

    Button::new(content)
        .padding(12)
        .width(Length::Fixed(MEDIA_CARD_WIDTH))
        .height(Length::Fixed(MEDIA_CARD_HEIGHT))
        .on_press(on_press)
        .style(|_theme, status| {
            let background = match status {
                iced::widget::button::Status::Hovered => theme::SURFACE_HOVER,
                iced::widget::button::Status::Pressed => theme::SURFACE_ACTIVE,
                _ => Color::TRANSPARENT,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(background)),
                border: Border {
                    radius: theme::RADIUS_LG.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn format_duration(ms: u32) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}

fn format_followers(n: u32) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.into_iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

fn capitalize_words(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[allow(clippy::too_many_lines)]
fn view_artist_detail_page<'a>(
    artist: &'a crate::app::SelectedArtistState,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    accent_tone: crate::ui::theme::AccentTone,
    playing_uri: Option<&str>,
) -> Element<'a, Message> {
    let mut info = Column::new()
        .spacing(8)
        .width(Length::Fill)
        .push(
            Text::new("ARTIST")
                .size(11)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_SECONDARY),
        )
        .push(single_line(
            artist.name.as_str(),
            48.0,
            theme::TEXT_PRIMARY,
            true,
        ));

    // Spotify stopped returning follower counts and genres to development-mode
    // apps; only show them when they're actually present.
    let mut facts: Vec<String> = Vec::new();
    if artist.followers > 0 {
        facts.push(format!("{} followers", format_followers(artist.followers)));
    }
    if !artist.genres.is_empty() {
        facts.push(
            artist
                .genres
                .iter()
                .take(3)
                .map(|g| capitalize_words(g))
                .collect::<Vec<_>>()
                .join(" • "),
        );
    }
    if !facts.is_empty() {
        info = info.push(single_line(
            facts.join("  ·  "),
            14.0,
            theme::TEXT_SECONDARY,
            false,
        ));
    }

    let header = Row::new()
        .spacing(28)
        .align_y(Alignment::End)
        .height(Length::Fixed(230.0))
        .push(view_image_or_icon(
            artist.image_url.as_deref(),
            loaded_images,
            Icon::User,
            230.0,
            theme::RADIUS_PILL,
        ))
        .push(info);

    let follow_btn = Button::new(
        Text::new("Follow")
            .size(13)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .color(theme::TEXT_PRIMARY),
    )
    .padding([8, 20])
    .on_press(Message::FollowArtistToggle(artist.id.clone(), false))
    .style(|_t, status| iced::widget::button::Style {
        background: None,
        border: Border {
            color: match status {
                iced::widget::button::Status::Hovered => theme::TEXT_PRIMARY,
                _ => theme::BORDER_STRONG,
            },
            width: 1.0,
            radius: 16.0.into(),
        },
        ..Default::default()
    });

    let mut actions = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .height(Length::Fixed(56.0));
    if let Some(first) = artist.top_tracks.first() {
        actions = actions.push(big_play_button(
            Message::PlayTrack(first.uri.clone()),
            accent_tone,
        ));
    }
    actions = actions.push(follow_btn);

    let body: Element<'a, Message> = if artist.is_loading {
        render_skeleton_rows(6)
    } else {
        let mut sections = Column::new().spacing(16);
        if !artist.top_tracks.is_empty() {
            let mut rows = Column::new();
            for (idx, track) in artist.top_tracks.iter().take(10).enumerate() {
                let image_url = track.image_url.as_ref().or(artist.image_url.as_ref());
                rows = rows.push(
                    Container::new(track_row(
                        &TrackRowSpec {
                            index: idx + 1,
                            title: &track.title,
                            artist: &track.album,
                            album: None,
                            duration_ms: track.duration_ms,
                            cover: CoverSlot::Shown(image_url.map(String::as_str)),
                            is_current: playing_uri == Some(track.uri.as_str()),
                            dimmed: false,
                            badge: None,
                        },
                        loaded_images,
                        Message::PlayTrack(track.uri.clone()),
                        Message::OpenTrackContextMenu {
                            track: crate::app::TrackInfo {
                                title: track.title.clone(),
                                artist: artist.name.clone(),
                                album: track.album.clone(),
                                duration_ms: track.duration_ms,
                                image_url: image_url.cloned(),
                                uri: track.uri.clone(),
                                explicit: false,
                            },
                            from_playlist_id: None,
                            position: iced::Point::new(450.0, 300.0),
                        },
                    ))
                    .height(Length::Fixed(TRACK_ROW_HEIGHT))
                    .align_y(iced::alignment::Vertical::Center),
                );
            }
            sections = sections.push(section_title("Popular")).push(rows);
        }

        if !artist.albums.is_empty() {
            sections = sections.push(section_title("Discography")).push(card_shelf(
                artist.albums.len(),
                move |idx| {
                    let album = &artist.albums[idx];
                    let year = album.release_date.get(..4).unwrap_or("Album");
                    media_card_with_image(
                        &album.name,
                        year.to_string(),
                        album.image_url.as_deref(),
                        loaded_images,
                        Icon::Album,
                        Message::SelectAlbum(album.id.clone()),
                    )
                },
            ));
        }

        if artist.top_tracks.is_empty() && artist.albums.is_empty() {
            sections = sections.push(empty_state("Nothing to show for this artist yet."));
        }
        sections.into()
    };

    main_page_frame(
        Column::new()
            .spacing(20)
            .push(header)
            .push(actions)
            .push(body),
    )
}

#[allow(clippy::too_many_lines)]
fn view_search_results<'a>(
    results: &'a crate::api::search::SearchResults,
    is_searching: bool,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    category_filter: SearchCategoryFilter,
    allow_explicit_content: bool,
    playing_uri: Option<&str>,
) -> Element<'a, Message> {
    let centered_note = |note: &'a str| -> Element<'a, Message> {
        Container::new(Text::new(note).size(16).color(theme::TEXT_SECONDARY))
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 120.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            })
            .align_x(iced::alignment::Horizontal::Center)
            .into()
    };

    let nothing =
        results.tracks.is_empty() && results.albums.is_empty() && results.artists.is_empty();
    if nothing && is_searching {
        // First query only: later queries keep the previous results on screen.
        return main_page_frame(render_skeleton_rows(6));
    }
    if nothing {
        return main_page_frame(centered_note("Type to search for songs, artists or albums"));
    }

    let tracks: Vec<&'a crate::api::search::SearchResultTrack> = results
        .tracks
        .iter()
        .filter(|t| allow_explicit_content || !t.explicit)
        .collect();

    let pills = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(filter_chip(
            "All",
            category_filter == SearchCategoryFilter::All,
            Message::SearchCategoryFilterSelected(SearchCategoryFilter::All),
        ))
        .push(filter_chip(
            "Songs",
            category_filter == SearchCategoryFilter::Tracks,
            Message::SearchCategoryFilterSelected(SearchCategoryFilter::Tracks),
        ))
        .push(filter_chip(
            "Artists",
            category_filter == SearchCategoryFilter::Artists,
            Message::SearchCategoryFilterSelected(SearchCategoryFilter::Artists),
        ))
        .push(filter_chip(
            "Albums",
            category_filter == SearchCategoryFilter::Albums,
            Message::SearchCategoryFilterSelected(SearchCategoryFilter::Albums),
        ))
        .push(Space::new().width(Length::Fill))
        .push(if is_searching {
            Text::new("Updating…").size(12).color(theme::TEXT_MUTED)
        } else {
            Text::new("")
        });

    let song_row = move |idx: usize, track: &'a crate::api::search::SearchResultTrack| {
        Container::new(track_row(
            &TrackRowSpec {
                index: idx + 1,
                title: &track.title,
                artist: &track.artist,
                album: Some(&track.album),
                duration_ms: track.duration_ms,
                cover: CoverSlot::Shown(track.image_url.as_deref()),
                is_current: playing_uri == Some(track.uri.as_str()),
                dimmed: false,
                badge: track.explicit.then_some("E"),
            },
            loaded_images,
            Message::PlayTrack(track.uri.clone()),
            Message::OpenTrackContextMenu {
                track: crate::app::TrackInfo {
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                    duration_ms: track.duration_ms,
                    image_url: track.image_url.clone(),
                    uri: track.uri.clone(),
                    explicit: track.explicit,
                },
                from_playlist_id: None,
                position: iced::Point::new(450.0, 300.0),
            },
        ))
        .height(Length::Fixed(TRACK_ROW_HEIGHT))
        .align_y(iced::alignment::Vertical::Center)
    };

    let artist_card = move |idx: usize| -> Element<'a, Message> {
        let artist = &results.artists[idx];
        media_card_with_image(
            &artist.name,
            "Artist".to_string(),
            artist.image_url.as_deref(),
            loaded_images,
            Icon::User,
            Message::SelectArtist(artist.id.clone()),
        )
    };
    let album_card = move |idx: usize| -> Element<'a, Message> {
        let album = &results.albums[idx];
        media_card_with_image(
            &album.name,
            album.artist_name.clone(),
            album.image_url.as_deref(),
            loaded_images,
            Icon::Album,
            Message::SelectAlbum(album.id.clone()),
        )
    };
    let card_grid = |count: usize, build: &dyn Fn(usize) -> Element<'a, Message>| {
        let mut row = Row::new().spacing(CARD_SPACING);
        for idx in 0..count {
            row = row.push(build(idx));
        }
        row.wrap().vertical_spacing(CARD_SPACING)
    };

    let mut content = Column::new().spacing(16).push(pills);

    match category_filter {
        SearchCategoryFilter::All => {
            if let Some(top) = tracks.first() {
                let top_card = Button::new(
                    Column::new()
                        .spacing(16)
                        .push(view_image_or_icon(
                            top.image_url.as_deref(),
                            loaded_images,
                            Icon::MusicNote,
                            96.0,
                            theme::RADIUS_MD,
                        ))
                        .push(single_line(
                            top.title.as_str(),
                            28.0,
                            theme::TEXT_PRIMARY,
                            true,
                        ))
                        .push(single_line(
                            format!("Song • {}", top.artist),
                            14.0,
                            theme::TEXT_SECONDARY,
                            false,
                        )),
                )
                .padding(20)
                .width(Length::Fixed(380.0))
                .height(Length::Fixed(4.0 * TRACK_ROW_HEIGHT))
                .on_press(Message::PlayTrack(top.uri.clone()))
                .style(|_theme, status| iced::widget::button::Style {
                    background: Some(Background::Color(match status {
                        iced::widget::button::Status::Hovered => theme::SURFACE_HOVER,
                        _ => theme::SURFACE_CARD,
                    })),
                    border: Border {
                        radius: theme::RADIUS_LG.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });

                let mut songs = Column::new();
                for (idx, track) in tracks.iter().take(4).enumerate() {
                    songs = songs.push(song_row(idx, track));
                }

                content = content.push(
                    Row::new()
                        .spacing(24)
                        .push(
                            Column::new()
                                .spacing(12)
                                .push(section_title("Top result"))
                                .push(top_card),
                        )
                        .push(
                            Column::new()
                                .spacing(12)
                                .width(Length::Fill)
                                .push(section_title("Songs"))
                                .push(songs),
                        ),
                );
            }
            if !results.artists.is_empty() {
                content = content
                    .push(section_title("Artists"))
                    .push(card_shelf(results.artists.len(), artist_card));
            }
            if !results.albums.is_empty() {
                content = content
                    .push(section_title("Albums"))
                    .push(card_shelf(results.albums.len(), album_card));
            }
        }
        SearchCategoryFilter::Tracks => {
            let mut songs = Column::new();
            for (idx, track) in tracks.iter().enumerate() {
                songs = songs.push(song_row(idx, track));
            }
            content = content.push(track_table_header(true)).push(songs);
        }
        SearchCategoryFilter::Artists => {
            content = content.push(card_grid(results.artists.len(), &artist_card));
        }
        SearchCategoryFilter::Albums => {
            content = content.push(card_grid(results.albums.len(), &album_card));
        }
    }

    main_page_frame(content)
}

#[allow(clippy::cast_precision_loss)]
fn render_skeleton_rows<'a>(count: usize) -> Element<'a, Message> {
    let mut col = Column::new().spacing(12);
    for i in 0..count {
        let title_width = 120.0 + (((i * 37) % 140) as f32);
        let artist_width = 80.0 + (((i * 23) % 90) as f32);

        let row = Row::new()
            .spacing(16)
            .align_y(Alignment::Center)
            .push(
                Container::new(Space::new())
                    .width(Length::Fixed(24.0))
                    .height(Length::Fixed(14.0))
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_HOVER)),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .push(
                Container::new(Space::new())
                    .width(Length::Fixed(title_width))
                    .height(Length::Fixed(14.0))
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_CARD)),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .push(
                Container::new(Space::new())
                    .width(Length::Fixed(artist_width))
                    .height(Length::Fixed(14.0))
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_HOVER)),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            );

        col = col.push(
            Container::new(row)
                .padding([10, 12])
                .width(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(theme::SURFACE_MAIN)),
                    border: Border {
                        radius: theme::RADIUS_MD.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }
    col.into()
}

#[allow(clippy::cast_precision_loss)]
fn render_skeleton_cards<'a>(count: usize) -> Element<'a, Message> {
    let mut row = Row::new().spacing(16);
    for i in 0..count {
        let title_width = 90.0 + (((i * 17) % 40) as f32);
        let sub_width = 60.0 + (((i * 13) % 30) as f32);

        let card_inner = Column::new()
            .spacing(10)
            .push(
                Container::new(Space::new())
                    .width(Length::Fixed(MEDIA_CARD_WIDTH - 24.0))
                    .height(Length::Fixed(MEDIA_CARD_WIDTH - 24.0))
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_HOVER)),
                        border: Border {
                            radius: theme::RADIUS_MD.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .push(
                Container::new(Space::new())
                    .width(Length::Fixed(title_width))
                    .height(Length::Fixed(14.0))
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_CARD)),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .push(
                Container::new(Space::new())
                    .width(Length::Fixed(sub_width))
                    .height(Length::Fixed(12.0))
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_HOVER)),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            );

        let card = Container::new(card_inner)
            .padding(12)
            .width(Length::Fixed(MEDIA_CARD_WIDTH))
            .height(Length::Fixed(MEDIA_CARD_HEIGHT));

        row = row.push(card);
    }
    Container::new(row)
        .width(Length::Fill)
        .height(Length::Fixed(MEDIA_CARD_HEIGHT))
        .clip(true)
        .into()
}

fn render_skeleton_quick_grid<'a>() -> Element<'a, Message> {
    fn make_skeleton_card<'a>() -> Element<'a, Message> {
        Container::new(
            Row::new()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(
                    Container::new(Space::new())
                        .width(Length::Fixed(48.0))
                        .height(Length::Fixed(48.0))
                        .style(|_theme| container::Style {
                            background: Some(Background::Color(theme::SURFACE_HOVER)),
                            border: Border {
                                radius: theme::RADIUS_SM.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                )
                .push(
                    Container::new(Space::new())
                        .width(Length::Fixed(110.0))
                        .height(Length::Fixed(14.0))
                        .style(|_theme| container::Style {
                            background: Some(Background::Color(theme::SURFACE_CARD)),
                            border: Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                ),
        )
        .padding(0)
        .width(Length::FillPortion(1))
        .height(Length::Fixed(48.0))
        .style(|_theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_MAIN)),
            border: Border {
                radius: theme::RADIUS_MD.into(),
                color: theme::BORDER_SUBTLE,
                width: 1.0,
            },
            ..Default::default()
        })
        .into()
    }

    let mut row_1 = Row::new().spacing(12);
    let mut row_2 = Row::new().spacing(12);

    for _ in 0..3 {
        row_1 = row_1.push(make_skeleton_card());
        row_2 = row_2.push(make_skeleton_card());
    }

    Column::new().spacing(12).push(row_1).push(row_2).into()
}

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    clippy::items_after_statements,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn view_settings_page<'a>(
    autoplay_enabled: bool,
    cache_size_bytes: u64,
    allow_explicit_content: bool,
    ui_scale: f32,
    accent_tone: crate::ui::theme::AccentTone,
    user_profile: Option<&'a crate::api::user::UserProfile>,
    ui_language: crate::app::UiLanguage,
    audio_bitrate: crate::audio::session::AudioBitrate,
    audio_normalization: bool,
    gapless_playback: bool,
) -> Element<'a, Message> {
    fn setting_row<'a>(
        title: &'static str,
        desc: impl iced::advanced::text::IntoFragment<'a>,
        control: Element<'a, Message>,
    ) -> Element<'a, Message> {
        Row::new()
            .spacing(16)
            .align_y(Alignment::Center)
            .push(
                Column::new()
                    .spacing(4)
                    .width(Length::FillPortion(3))
                    .push(
                        Text::new(title)
                            .size(15)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_PRIMARY),
                    )
                    .push(Text::new(desc).size(13).color(theme::TEXT_SECONDARY)),
            )
            .push(Container::new(control).width(Length::FillPortion(2)))
            .into()
    }

    let section_title = move |title: &'static str| -> Element<'a, Message> {
        Text::new(title)
            .size(18)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .color(accent_tone.primary())
            .into()
    };

    let header = Text::new("Settings")
        .size(32)
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        })
        .color(theme::TEXT_PRIMARY);

    fn make_badge_enabled<'a>() -> Element<'a, Message> {
        Container::new(
            Text::new("Enabled")
                .size(12)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::COLOR_SUCCESS),
        )
        .padding([6, 12])
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color {
                r: theme::COLOR_SUCCESS.r,
                g: theme::COLOR_SUCCESS.g,
                b: theme::COLOR_SUCCESS.b,
                a: 0.15,
            })),
            border: Border {
                color: theme::COLOR_SUCCESS,
                width: 1.0,
                radius: theme::RADIUS_PILL.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn make_toggle_badge<'a>(enabled: bool, msg: Message) -> Element<'a, Message> {
        let (label, bg_color, text_color) = if enabled {
            (
                "Enabled",
                Color {
                    r: theme::COLOR_SUCCESS.r,
                    g: theme::COLOR_SUCCESS.g,
                    b: theme::COLOR_SUCCESS.b,
                    a: 0.15,
                },
                theme::COLOR_SUCCESS,
            )
        } else {
            (
                "Disabled",
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.05,
                },
                theme::TEXT_MUTED,
            )
        };
        Button::new(
            Text::new(label)
                .size(12)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(text_color),
        )
        .padding([6, 12])
        .on_press(msg)
        .style(move |_theme, _status| iced::widget::button::Style {
            background: Some(Background::Color(bg_color)),
            border: Border {
                color: text_color,
                width: 1.0,
                radius: theme::RADIUS_PILL.into(),
            },
            ..Default::default()
        })
        .into()
    }

    let path_box = Container::new(
        Text::new("/home/user/music")
            .size(13)
            .color(theme::TEXT_PRIMARY),
    )
    .padding([8, 14])
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::SURFACE_HOVER)),
        border: Border {
            color: theme::BORDER_SUBTLE,
            width: 1.0,
            radius: theme::RADIUS_MD.into(),
        },
        ..Default::default()
    });

    let (account_name, plan_desc) = match user_profile {
        Some(p) => (
            p.display_name.as_str(),
            p.product.as_deref().unwrap_or("Spotify Account"),
        ),
        None => ("Connected User", "Spotify Account"),
    };

    let manage_btn = Button::new(
        Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(
                Text::new("Manage on Spotify")
                    .size(13)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(Color::WHITE),
            )
            .push(Icon::ChevronRight.view_colored(14.0, Color::WHITE)),
    )
    .padding([8, 16])
    .on_press(Message::OpenSpotifyAccount)
    .style(move |_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                color: theme::BORDER_SUBTLE,
                width: 1.0,
                radius: theme::RADIUS_PILL.into(),
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                border: Border {
                    color: Color::WHITE,
                    width: 1.0,
                    radius: theme::RADIUS_PILL.into(),
                },
                ..base
            },
            _ => base,
        }
    });

    let mut lang_picker = Row::new().spacing(8).align_y(Alignment::Center);
    for lang in crate::app::UiLanguage::ALL {
        let is_selected = lang == ui_language;
        let lang_btn = Button::new(
            Text::new(lang.name())
                .size(12)
                .font(iced::Font {
                    weight: if is_selected {
                        iced::font::Weight::Bold
                    } else {
                        iced::font::Weight::Normal
                    },
                    ..Default::default()
                })
                .color(if is_selected {
                    Color::WHITE
                } else {
                    theme::TEXT_SECONDARY
                }),
        )
        .padding([6, 14])
        .on_press(Message::SetUiLanguage(lang))
        .style(move |_theme, status| {
            let bg_color = if is_selected {
                Color {
                    r: accent_tone.primary().r,
                    g: accent_tone.primary().g,
                    b: accent_tone.primary().b,
                    a: 0.20,
                }
            } else {
                Color::TRANSPARENT
            };
            let border_color = if is_selected {
                accent_tone.primary()
            } else {
                theme::BORDER_SUBTLE
            };
            let base = iced::widget::button::Style {
                background: Some(Background::Color(bg_color)),
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: theme::RADIUS_PILL.into(),
                },
                ..Default::default()
            };
            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Background::Color(Color {
                        r: accent_tone.primary().r,
                        g: accent_tone.primary().g,
                        b: accent_tone.primary().b,
                        a: 0.30,
                    })),
                    border: Border {
                        color: accent_tone.primary(),
                        width: 1.0,
                        radius: theme::RADIUS_PILL.into(),
                    },
                    ..base
                },
                _ => base,
            }
        });
        lang_picker = lang_picker.push(lang_btn);
    }

    let mut bitrate_picker = Row::new().spacing(8).align_y(Alignment::Center);
    for rate in crate::audio::session::AudioBitrate::ALL {
        let is_selected = rate == audio_bitrate;
        let rate_btn = Button::new(
            Text::new(rate.label())
                .size(12)
                .font(iced::Font {
                    weight: if is_selected {
                        iced::font::Weight::Bold
                    } else {
                        iced::font::Weight::Normal
                    },
                    ..Default::default()
                })
                .color(if is_selected {
                    Color::WHITE
                } else {
                    theme::TEXT_SECONDARY
                }),
        )
        .padding([6, 12])
        .on_press(Message::SetAudioBitrate(rate))
        .style(move |_theme, status| {
            let bg_color = if is_selected {
                Color {
                    r: accent_tone.primary().r,
                    g: accent_tone.primary().g,
                    b: accent_tone.primary().b,
                    a: 0.25,
                }
            } else {
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.04,
                }
            };
            let base = iced::widget::button::Style {
                background: Some(Background::Color(bg_color)),
                border: Border {
                    color: if is_selected {
                        accent_tone.primary()
                    } else {
                        theme::BORDER_SUBTLE
                    },
                    width: 1.0,
                    radius: theme::RADIUS_PILL.into(),
                },
                ..Default::default()
            };
            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Background::Color(Color {
                        r: accent_tone.primary().r,
                        g: accent_tone.primary().g,
                        b: accent_tone.primary().b,
                        a: 0.35,
                    })),
                    border: Border {
                        color: accent_tone.primary(),
                        width: 1.0,
                        radius: theme::RADIUS_PILL.into(),
                    },
                    ..base
                },
                _ => base,
            }
        });
        bitrate_picker = bitrate_picker.push(rate_btn);
    }

    let account_summary = format!("{account_name} • {plan_desc}");
    let main_col = Column::new()
        .spacing(24)
        .push(header)
        .push(section_title("Account & Profile"))
        .push(setting_row(
            "Spotify Account",
            account_summary,
            manage_btn.into(),
        ))
        .push(section_title("Language & Region"))
        .push(setting_row(
            "Interface Language",
            "Choose your display language for application menus and controls.",
            lang_picker.into(),
        ))
        .push(section_title("Explicit Content"))
        .push(setting_row(
            "Allow Explicit Content",
            "Turn on to see and play music with the explicit content tag [E].",
            make_toggle_badge(allow_explicit_content, Message::ToggleExplicitContent),
        ))
        .push(section_title("Autoplay & Recommendations"))
        .push(setting_row(
            "Autoplay Similar Songs",
            "Keep listening to similar recommended songs when your music or queue ends.",
            make_toggle_badge(autoplay_enabled, Message::ToggleAutoplay),
        ))
        .push(section_title("Audio & Streaming Quality"))
        .push(setting_row(
            "Streaming Quality",
            "Highest quality audio streaming available (320 kbps Vorbis for Premium).",
            bitrate_picker.into(),
        ))
        .push(setting_row(
            "Audio Normalization",
            "Set the same volume level for all tracks during playback.",
            make_toggle_badge(audio_normalization, Message::ToggleAudioNormalization),
        ))
        .push(section_title("Audio Effects & Crossfade"))
        .push(setting_row(
            "Gapless Playback",
            "Allows playback to transition between continuous album tracks without silence.",
            make_toggle_badge(gapless_playback, Message::ToggleGaplessPlayback),
        ))
        .push(setting_row(
            "Crossfade",
            "Allows tracks to crossfade into each other seamlessly.",
            make_badge_enabled(),
        ))
        .push(section_title("Local Files"))
        .push(setting_row(
            "Show Local Files",
            "Scan and display audio files from your local computer storage.",
            make_badge_enabled(),
        ))
        .push(setting_row(
            "Local Music Directory",
            "Folder path where your local music files (.mp3, .flac, .ogg, .wav) are located.",
            path_box.into(),
        ))
        .push(section_title("Spotify Connect & Devices"))
        .push(setting_row(
            "Spotify Connect",
            "Control playback across your phone, tablet, and web player.",
            make_badge_enabled(),
        ));

    let cache_control = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(
            Container::new(
                Text::new(crate::api::cache::format_bytes(cache_size_bytes))
                    .size(13)
                    .color(theme::TEXT_SECONDARY),
            )
            .padding([6, 12])
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                border: Border {
                    color: theme::BORDER_SUBTLE,
                    width: 1.0,
                    radius: theme::RADIUS_MD.into(),
                },
                ..Default::default()
            }),
        )
        .push(
            Button::new(
                Text::new("Clear Cache")
                    .size(12)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .padding([6, 14])
            .on_press(Message::ClearCacheRequested)
            .style(|_theme, status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => theme::SURFACE_HOVER,
                    _ => theme::SURFACE_CARD,
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(bg)),
                    border: Border {
                        color: theme::BORDER_SUBTLE,
                        width: 1.0,
                        radius: theme::RADIUS_MD.into(),
                    },
                    ..Default::default()
                }
            }),
        );

    let scale_pct = (ui_scale * 100.0).round() as u32;
    let scale_control = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(
            Button::new(
                Text::new("-")
                    .size(14)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .padding([6, 12])
            .on_press(Message::AdjustUiScale(-0.05))
            .style(|_theme, status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => theme::SURFACE_HOVER,
                    _ => theme::SURFACE_CARD,
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(bg)),
                    border: Border {
                        color: theme::BORDER_SUBTLE,
                        width: 1.0,
                        radius: theme::RADIUS_MD.into(),
                    },
                    ..Default::default()
                }
            }),
        )
        .push(
            Container::new(
                Text::new(format!("{scale_pct}%"))
                    .size(13)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .padding([6, 12])
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                border: Border {
                    color: theme::BORDER_SUBTLE,
                    width: 1.0,
                    radius: theme::RADIUS_MD.into(),
                },
                ..Default::default()
            }),
        )
        .push(
            Button::new(
                Text::new("+")
                    .size(14)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .padding([6, 12])
            .on_press(Message::AdjustUiScale(0.05))
            .style(|_theme, status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => theme::SURFACE_HOVER,
                    _ => theme::SURFACE_CARD,
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(bg)),
                    border: Border {
                        color: theme::BORDER_SUBTLE,
                        width: 1.0,
                        radius: theme::RADIUS_MD.into(),
                    },
                    ..Default::default()
                }
            }),
        )
        .push(
            Button::new(
                Text::new("Reset")
                    .size(12)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_MUTED),
            )
            .padding([6, 12])
            .on_press(Message::ResetUiScale)
            .style(|_theme, status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => theme::SURFACE_HOVER,
                    _ => theme::SURFACE_CARD,
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(bg)),
                    border: Border {
                        color: theme::BORDER_SUBTLE,
                        width: 1.0,
                        radius: theme::RADIUS_MD.into(),
                    },
                    ..Default::default()
                }
            }),
        );

    let mut accent_picker = Row::new().spacing(8).align_y(Alignment::Center);
    for tone in crate::ui::theme::AccentTone::ALL {
        let is_selected = tone == accent_tone;
        let dot = Container::new(Space::new())
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(move |_theme| container::Style {
                background: Some(Background::Color(tone.primary())),
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let content = Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(dot)
            .push(
                Text::new(tone.name())
                    .size(12)
                    .font(iced::Font {
                        weight: if is_selected {
                            iced::font::Weight::Bold
                        } else {
                            iced::font::Weight::Normal
                        },
                        ..Default::default()
                    })
                    .color(if is_selected {
                        Color::WHITE
                    } else {
                        theme::TEXT_SECONDARY
                    }),
            );

        let tone_btn = Button::new(content)
            .padding([6, 12])
            .on_press(Message::SetAccentTone(tone))
            .style(move |_theme, status| {
                let bg_color = if is_selected {
                    Color {
                        r: tone.primary().r,
                        g: tone.primary().g,
                        b: tone.primary().b,
                        a: 0.20,
                    }
                } else {
                    Color::TRANSPARENT
                };
                let border_color = if is_selected {
                    tone.primary()
                } else {
                    theme::BORDER_SUBTLE
                };
                let base = iced::widget::button::Style {
                    background: Some(Background::Color(bg_color)),
                    border: Border {
                        color: border_color,
                        width: 1.0,
                        radius: theme::RADIUS_PILL.into(),
                    },
                    ..Default::default()
                };
                match status {
                    iced::widget::button::Status::Hovered => iced::widget::button::Style {
                        background: Some(Background::Color(Color {
                            r: tone.primary().r,
                            g: tone.primary().g,
                            b: tone.primary().b,
                            a: 0.30,
                        })),
                        border: Border {
                            color: tone.primary(),
                            width: 1.0,
                            radius: theme::RADIUS_PILL.into(),
                        },
                        ..base
                    },
                    _ => base,
                }
            });

        accent_picker = accent_picker.push(tone_btn);
    }

    let main_col = main_col
        .push(section_title("Theme & Accent Color Tone"))
        .push(setting_row(
            "Accent Color Tone",
            "Choose your preferred accent color for highlights, badges, and controls.",
            accent_picker.into(),
        ))
        .push(section_title("UI Scaling & Accessibility"))
        .push(setting_row(
            "Interface Zoom",
            "Adjust the interface scale from 70% to 130% (shortcuts: Ctrl +, Ctrl -, Ctrl 0).",
            scale_control.into(),
        ))
        .push(section_title("Storage & Cache"))
        .push(setting_row(
            "Cache Storage",
            "Local disk space used for cached album artwork, fragments, and metadata.",
            cache_control.into(),
        ));

    Scrollable::new(
        Container::new(main_col)
            .width(Length::Fill)
            .padding(32)
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(theme::SURFACE_MAIN)),
                border: Border {
                    radius: theme::RADIUS_LG.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .into()
}

fn view_mini_player<'a>(
    playback: &'a PlaybackState,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let (track_name, artist_name, image_url) = if let Some(track) = &playback.current_track {
        (
            track.title.as_str(),
            track.artist.as_str(),
            track.image_url.as_deref(),
        )
    } else {
        ("Synthetic Horizon", "Spotifust Audio Engine", None)
    };

    let track_cover = view_image_or_icon(
        image_url,
        loaded_images,
        Icon::MusicNote,
        48.0,
        theme::RADIUS_MD,
    );

    let play_pause_icon = if playback.is_playing {
        Icon::Pause
    } else {
        Icon::Play
    };

    let content = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(track_cover)
        .push(
            Column::new()
                .spacing(2)
                .width(Length::Fill)
                .push(
                    Text::new(track_name)
                        .size(13)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(theme::TEXT_PRIMARY),
                )
                .push(Text::new(artist_name).size(11).color(theme::TEXT_SECONDARY)),
        )
        .push(icon_button_plain(Icon::SkipPrev, Message::SkipPrev))
        .push(icon_button_circle_accent(
            play_pause_icon,
            Message::TogglePlayback,
        ))
        .push(icon_button_plain(Icon::SkipNext, Message::SkipNext));

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_MAIN)),
            ..Default::default()
        })
        .into()
}

pub fn thin_scrollable<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Scrollable<'a, Message> {
    Scrollable::new(content)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(6.0)
                .margin(2.0)
                .scroller_width(6.0),
        ))
        .style(|theme, status| {
            let mut s = scrollable::default(theme, status);
            s.vertical_rail.background = None;
            s.vertical_rail.scroller.background = Background::Color(Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.25,
            });
            s.vertical_rail.scroller.border = Border {
                radius: 3.0.into(),
                ..Border::default()
            };
            s
        })
}

#[must_use]
pub fn calculate_progress_ratio(progress_ms: u32, duration_ms: u32) -> f32 {
    if duration_ms > 0 {
        #[allow(clippy::cast_precision_loss)]
        (progress_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(0), "0:00");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(45_000), "0:45");
    }

    #[test]
    fn test_format_duration_minutes_and_seconds() {
        assert_eq!(format_duration(225_000), "3:45");
    }

    #[test]
    fn test_calculate_progress_ratio_zero_duration() {
        assert_eq!(calculate_progress_ratio(5000, 0), 0.0);
    }

    #[test]
    fn test_calculate_progress_ratio_halfway() {
        assert_eq!(calculate_progress_ratio(50_000, 100_000), 0.5);
    }

    #[test]
    fn test_calculate_progress_ratio_clamped() {
        assert_eq!(calculate_progress_ratio(150_000, 100_000), 1.0);
    }
}
