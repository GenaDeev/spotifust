use crate::app::{Message, NavigationItem, PlaybackState, RightPanelTab, SidebarFilter};
use crate::ui::icons::Icon;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Theme,
    widget::{
        Button, Column, Container, Image, Row, Scrollable, Space, Text, TextInput, container,
        slider, text_input,
    },
};

const LOGO_BYTES: &[u8] = include_bytes!("../../assets/spotifust.png");

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

#[allow(clippy::too_many_arguments)]
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
    search_query: &'a str,
    search_results: &'a crate::api::search::SearchResults,
    is_searching: bool,
    sidebar_filter: SidebarFilter,
    selected_playlist: Option<&'a crate::app::SelectedPlaylistState>,
    selected_album: Option<&'a crate::app::SelectedAlbumState>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let top_bar = view_top_bar(*nav_item, user_profile, search_query, loaded_images);
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
        user_playlists,
        user_albums,
        user_top_tracks,
        search_results,
        is_searching,
        loaded_images,
    );
    let right_panel = view_right_panel(
        active_right_panel,
        right_panel_width,
        playback,
        loaded_images,
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

    let layout = Column::new()
        .push(top_bar)
        .push(middle_section)
        .push(playback_bar);

    Container::new(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::BG_BASE)),
            ..Default::default()
        })
        .into()
}

#[allow(clippy::too_many_lines)]
fn view_top_bar<'a>(
    current_nav: NavigationItem,
    user_profile: Option<&'a crate::api::user::UserProfile>,
    search_query: &'a str,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let logo_handle = iced::widget::image::Handle::from_bytes(LOGO_BYTES);
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

    let user_avatar_btn = Button::new(user_avatar_content)
        .padding(0)
        .on_press(Message::MockAction)
        .style(|_theme, _status| iced::widget::button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        });

    let right_controls = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(icon_button_circle(Icon::Plus, Message::MockAction))
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

        let scrollable_list = Scrollable::new(list).height(Length::Fill);

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
        .push(Space::new().width(Length::Fill))
        .push(icon_button_circle(Icon::Plus, Message::MockAction));

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
            list = list.push(sidebar_item_with_image(
                &p.name,
                &sub,
                p.image_url.as_deref(),
                loaded_images,
                Icon::MusicNote,
                is_active,
                Message::SelectPlaylist(p_id),
            ));
        }
    }

    if show_albums {
        for a in albums {
            let is_active = selected_album.is_some_and(|sa| sa.id == a.id);
            let sub = format!("Album • {}", a.artist_name);
            let a_id = a.id.clone();
            list = list.push(sidebar_item_with_image(
                &a.name,
                &sub,
                a.image_url.as_deref(),
                loaded_images,
                Icon::Album,
                is_active,
                Message::SelectAlbum(a_id),
            ));
        }
    }

    if playlists.is_empty() && albums.is_empty() {
        let items = [
            (
                "Synthwave Architect",
                "Album • The Midnight",
                Icon::Album,
                false,
            ),
            (
                "Rustaceans Unite",
                "Playlist • Spotifust",
                Icon::MusicNote,
                false,
            ),
            (
                "Chill Lofi Beats",
                "Playlist • Spotifust",
                Icon::Queue,
                false,
            ),
        ];

        for (title, sub, icon, active) in items {
            list = list.push(sidebar_item(
                title,
                sub,
                icon,
                active,
                false,
                Message::MockAction,
            ));
        }
    }

    let scrollable_list = Scrollable::new(list).height(Length::Fill);

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

#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn view_main_content<'a>(
    current_nav: NavigationItem,
    selected_playlist: Option<&'a crate::app::SelectedPlaylistState>,
    selected_album: Option<&'a crate::app::SelectedAlbumState>,
    user_playlists: &'a [crate::api::playlist::PlaylistSummary],
    user_albums: &'a [crate::api::album::AlbumSummary],
    user_top_tracks: &'a [crate::api::tracks::TopTrack],
    search_results: &'a crate::api::search::SearchResults,
    is_searching: bool,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    if current_nav == NavigationItem::Search {
        return view_search_results(search_results, is_searching, loaded_images);
    }

    if let Some(sp) = selected_playlist {
        let playlist_cover_url = sp
            .image_url
            .as_deref()
            .or_else(|| sp.tracks.first().and_then(|t| t.image_url.as_deref()));

        let playlist_header = Row::new()
            .spacing(24)
            .align_y(Alignment::Center)
            .push(view_image_or_icon(
                playlist_cover_url,
                loaded_images,
                Icon::MusicNote,
                230.0,
                theme::RADIUS_LG,
            ))
            .push(
                Column::new()
                    .spacing(6)
                    .push(
                        Text::new("PLAYLIST")
                            .size(11)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::ACCENT),
                    )
                    .push(
                        Text::new(&sp.name)
                            .size(32)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_PRIMARY),
                    )
                    .push(
                        Text::new(format!("{} tracks loaded", sp.tracks.len()))
                            .size(13)
                            .color(theme::TEXT_SECONDARY),
                    ),
            );

        let content_body: Element<'a, Message> = if sp.is_loading {
            render_skeleton_rows(8)
        } else if sp.tracks.is_empty() {
            Container::new(
                Text::new("No tracks found in this playlist.")
                    .size(15)
                    .color(theme::TEXT_SECONDARY),
            )
            .padding(32)
            .into()
        } else {
            let mut tracks_column = Column::new().spacing(6);

            let table_header = Row::new()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(
                    Text::new("#")
                        .size(13)
                        .color(theme::TEXT_SECONDARY)
                        .width(Length::Fixed(24.0)),
                )
                .push(
                    Text::new("Title")
                        .size(13)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(theme::TEXT_SECONDARY)
                        .width(Length::FillPortion(3)),
                )
                .push(
                    Text::new("Artist")
                        .size(13)
                        .color(theme::TEXT_SECONDARY)
                        .width(Length::FillPortion(2)),
                )
                .push(
                    Text::new("Album")
                        .size(13)
                        .color(theme::TEXT_SECONDARY)
                        .width(Length::FillPortion(2)),
                )
                .push(
                    Text::new("Duration")
                        .size(13)
                        .color(theme::TEXT_SECONDARY)
                        .width(Length::Fixed(60.0)),
                );

            tracks_column = tracks_column.push(
                Container::new(table_header)
                    .padding([8, 12])
                    .style(|_theme| container::Style {
                        border: Border {
                            color: theme::BORDER_SUBTLE,
                            width: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            );

            for (idx, track) in sp.tracks.iter().enumerate() {
                let track_num = (idx + 1).to_string();
                let dur_str = format_duration(track.duration_ms);
                let uri = track.uri.clone();

                let title_text = Text::new(&track.title)
                    .size(14)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(if track.is_local {
                        theme::TEXT_MUTED
                    } else {
                        theme::TEXT_PRIMARY
                    });

                let mut title_row = Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(title_text);

                if track.is_local {
                    let local_badge = Container::new(
                        Text::new("LOCAL")
                            .size(9)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_MUTED),
                    )
                    .padding([2, 6])
                    .style(|_theme: &Theme| container::Style {
                        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    });

                    title_row = title_row.push(local_badge);
                }

                let track_row = Row::new()
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(
                        Text::new(track_num)
                            .size(13)
                            .color(theme::TEXT_SECONDARY)
                            .width(Length::Fixed(24.0)),
                    )
                    .push(Container::new(title_row).width(Length::FillPortion(3)))
                    .push(
                        Text::new(&track.artist)
                            .size(13)
                            .color(theme::TEXT_SECONDARY)
                            .width(Length::FillPortion(2)),
                    )
                    .push(
                        Text::new(&track.album)
                            .size(13)
                            .color(theme::TEXT_SECONDARY)
                            .width(Length::FillPortion(2)),
                    )
                    .push(
                        Text::new(dur_str)
                            .size(13)
                            .color(theme::TEXT_SECONDARY)
                            .width(Length::Fixed(60.0)),
                    );

                let track_item = Button::new(
                    Container::new(track_row)
                        .padding([8, 12])
                        .width(Length::Fill),
                )
                .padding(0)
                .on_press(Message::PlayTrack(uri))
                .style(|_theme, status| {
                    let base = iced::widget::button::Style {
                        background: Some(Background::Color(Color::TRANSPARENT)),
                        border: Border {
                            radius: theme::RADIUS_MD.into(),
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
                });

                tracks_column = tracks_column.push(track_item);
            }

            tracks_column.into()
        };

        let page_column = Column::new()
            .spacing(20)
            .push(playlist_header)
            .push(content_body);

        let scrollable = Scrollable::new(Container::new(page_column).padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(6.0)
                .margin(2.0)
                .scroller_width(6.0),
        ))
        .height(Length::Fill);

        return Container::new(scrollable)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
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

    if let Some(sa) = selected_album {
        let cover = view_image_or_icon(
            sa.image_url.as_deref(),
            loaded_images,
            Icon::Album,
            230.0,
            theme::RADIUS_LG,
        );

        let album_header = Row::new()
            .spacing(24)
            .align_y(Alignment::Center)
            .push(cover)
            .push(
                Column::new()
                    .spacing(6)
                    .push(
                        Text::new("ALBUM")
                            .size(11)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::ACCENT),
                    )
                    .push(
                        Text::new(&sa.name)
                            .size(32)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_PRIMARY),
                    )
                    .push(
                        Text::new(format!("{} • {}", sa.artist_name, sa.release_date))
                            .size(13)
                            .color(theme::TEXT_SECONDARY),
                    ),
            );

        let content_body: Element<'a, Message> = if sa.is_loading {
            render_skeleton_rows(8)
        } else if sa.tracks.is_empty() {
            Container::new(
                Text::new("No tracks found in this album.")
                    .size(15)
                    .color(theme::TEXT_SECONDARY),
            )
            .padding(32)
            .into()
        } else {
            let mut tracks_column = Column::new().spacing(6);

            for track in &sa.tracks {
                let track_num = track.track_number.to_string();
                let dur_str = format_duration(track.duration_ms);
                let uri = track.uri.clone();

                let track_row = Row::new()
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(
                        Text::new(track_num)
                            .size(13)
                            .color(theme::TEXT_SECONDARY)
                            .width(Length::Fixed(24.0)),
                    )
                    .push(
                        Text::new(&track.title)
                            .size(14)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_PRIMARY)
                            .width(Length::FillPortion(3)),
                    )
                    .push(
                        Text::new(&track.artist)
                            .size(13)
                            .color(theme::TEXT_SECONDARY)
                            .width(Length::FillPortion(2)),
                    )
                    .push(
                        Text::new(dur_str)
                            .size(13)
                            .color(theme::TEXT_SECONDARY)
                            .width(Length::Fixed(60.0)),
                    );

                let track_item = Button::new(
                    Container::new(track_row)
                        .padding([8, 12])
                        .width(Length::Fill),
                )
                .padding(0)
                .on_press(Message::PlayTrack(uri))
                .style(|_theme, status| {
                    let base = iced::widget::button::Style {
                        background: Some(Background::Color(Color::TRANSPARENT)),
                        border: Border {
                            radius: theme::RADIUS_MD.into(),
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
                });

                tracks_column = tracks_column.push(track_item);
            }

            tracks_column.into()
        };

        let page_column = Column::new()
            .spacing(20)
            .push(album_header)
            .push(content_body);

        let scrollable = Scrollable::new(Container::new(page_column).padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(6.0)
                .margin(2.0)
                .scroller_width(6.0),
        ))
        .height(Length::Fill);

        return Container::new(scrollable)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
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

    let title_text = match current_nav {
        NavigationItem::Home => "Good evening",
        NavigationItem::Search => "Search",
        NavigationItem::Library => "Your Library",
    };

    let header = Text::new(title_text)
        .size(30)
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        })
        .color(theme::TEXT_PRIMARY);

    let mut row_1 = Row::new().spacing(12);
    let mut row_2 = Row::new().spacing(12);

    let items_source: Vec<(&str, Option<&str>, Icon, Message)> =
        if !user_playlists.is_empty() || !user_albums.is_empty() {
            let mut list = Vec::new();
            for p in user_playlists.iter().take(3) {
                list.push((
                    p.name.as_str(),
                    p.image_url.as_deref(),
                    Icon::MusicNote,
                    Message::SelectPlaylist(p.id.clone()),
                ));
            }
            for a in user_albums.iter().take(3) {
                list.push((
                    a.name.as_str(),
                    a.image_url.as_deref(),
                    Icon::Album,
                    Message::SelectAlbum(a.id.clone()),
                ));
            }
            list
        } else {
            Vec::new()
        };

    for (idx, (title, img_url, icon, msg)) in items_source.into_iter().enumerate() {
        let card = quick_card_with_image(title, img_url, loaded_images, icon, msg);
        if idx < 3 {
            row_1 = row_1.push(card);
        } else {
            row_2 = row_2.push(card);
        }
    }

    let quick_grid = Column::new().spacing(12).push(row_1).push(row_2);

    let section_1_header = Row::new()
        .align_y(Alignment::Center)
        .push(
            Text::new("Top Tracks For You")
                .size(22)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .push(Space::new().width(Length::Fill));

    let section_1_cards = if user_top_tracks.is_empty() {
        Row::new().push(
            Text::new("No top tracks available")
                .size(13)
                .color(theme::TEXT_SECONDARY),
        )
    } else {
        let mut row = Row::new().spacing(16);
        for track in user_top_tracks.iter().take(5) {
            let subtitle = format!("{} • Track", track.artist);
            let uri = track.uri.clone();
            row = row.push(media_card_with_image(
                &track.title,
                &subtitle,
                track.image_url.as_deref(),
                loaded_images,
                Icon::MusicNote,
                Message::PlayTrack(uri),
            ));
        }
        row
    };

    let section_2_header = Row::new()
        .align_y(Alignment::Center)
        .push(
            Text::new("Saved Albums")
                .size(22)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .push(Space::new().width(Length::Fill));

    let section_2_cards = if user_albums.is_empty() {
        Row::new().push(
            Text::new("No saved albums available")
                .size(13)
                .color(theme::TEXT_SECONDARY),
        )
    } else {
        let mut row = Row::new().spacing(16);
        for a in user_albums.iter().take(5) {
            let subtitle = format!("{} • Album", a.artist_name);
            let a_id = a.id.clone();
            row = row.push(media_card_with_image(
                &a.name,
                &subtitle,
                a.image_url.as_deref(),
                loaded_images,
                Icon::Album,
                Message::SelectAlbum(a_id),
            ));
        }
        row
    };

    let scroll_content = Column::new()
        .spacing(24)
        .padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 0.0,
        })
        .push(header)
        .push(quick_grid)
        .push(section_1_header)
        .push(scroll_row(section_1_cards))
        .push(section_2_header)
        .push(scroll_row(section_2_cards));

    let scrollable = Scrollable::new(scroll_content)
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(6.0)
                .margin(2.0)
                .scroller_width(6.0),
        ))
        .style(|theme: &Theme, status| {
            let mut s = iced::widget::scrollable::default(theme, status);
            s.vertical_rail.background = None;
            s.vertical_rail.scroller.background = Background::Color(Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.18,
            });
            s.vertical_rail.scroller.border = Border {
                radius: theme::RADIUS_PILL.into(),
                ..Border::default()
            };
            s
        })
        .width(Length::Fill)
        .height(Length::Fill);

    Container::new(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            top: 20.0,
            right: 12.0,
            bottom: 20.0,
            left: 20.0,
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

fn scroll_row(content: Row<'_, Message>) -> Element<'_, Message> {
    Scrollable::new(content)
        .direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new()
                .width(4.0)
                .margin(2.0)
                .scroller_width(4.0),
        ))
        .width(Length::Fill)
        .into()
}

#[allow(clippy::too_many_lines)]
fn view_right_panel<'a>(
    active_tab: Option<RightPanelTab>,
    width: f32,
    playback: &'a PlaybackState,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
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
            let lyrics_card = Container::new(
                Column::new()
                    .spacing(12)
                    .push(
                        Text::new("♫ Synchronized Lyrics")
                            .size(16)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::ACCENT),
                    )
                    .push(
                        Text::new("Lyrics provider connected.")
                            .size(14)
                            .color(theme::TEXT_SECONDARY),
                    ),
            )
            .padding(20)
            .style(|_theme| container::Style {
                background: Some(Background::Color(theme::SURFACE_CARD)),
                border: Border {
                    radius: theme::RADIUS_MD.into(),
                    color: theme::BORDER_SUBTLE,
                    width: 1.0,
                },
                ..Default::default()
            });

            Column::new().spacing(16).push(lyrics_card).into()
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

            let art_placeholder =
                view_image_or_icon(img_url, loaded_images, Icon::Album, 240.0, theme::RADIUS_LG);

            let track_title = Text::new(track_title_str)
                .size(20)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY);

            let artist_name = Text::new(artist_name_str)
                .size(14)
                .color(theme::TEXT_SECONDARY);

            let artist_card = Container::new(
                Column::new()
                    .spacing(8)
                    .push(
                        Text::new("About the artist")
                            .size(14)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_PRIMARY),
                    )
                    .push(
                        Text::new("Spotifust is a high-performance, single-binary Rust client built for extreme speed and low RAM footprint.")
                            .size(12)
                            .color(theme::TEXT_SECONDARY),
                    ),
            )
            .padding(16)
            .style(|_theme| container::Style {
                background: Some(Background::Color(theme::SURFACE_CARD)),
                border: Border {
                    radius: theme::RADIUS_MD.into(),
                    color: theme::BORDER_SUBTLE,
                    width: 1.0,
                },
                ..Default::default()
            });

            Column::new()
                .spacing(16)
                .push(art_placeholder)
                .push(track_title)
                .push(artist_name)
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

            let (curr_title, curr_artist) = if let Some(track) = &playback.current_track {
                (track.title.as_str(), track.artist.as_str())
            } else {
                ("Synthetic Horizon", "Spotifust Audio Engine")
            };

            let current_item = sidebar_item(
                curr_title,
                curr_artist,
                Icon::Play,
                true,
                false,
                Message::MockAction,
            );

            let next_header = Text::new("Next in Queue")
                .size(14)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY);

            let queue_list = Column::new()
                .spacing(4)
                .push(sidebar_item(
                    "Sunset Drive",
                    "The Midnight",
                    Icon::MusicNote,
                    false,
                    false,
                    Message::MockAction,
                ))
                .push(sidebar_item(
                    "Tech Noir",
                    "GUNSHIP",
                    Icon::Album,
                    false,
                    false,
                    Message::MockAction,
                ))
                .push(sidebar_item(
                    "Resonance",
                    "HOME",
                    Icon::Queue,
                    false,
                    false,
                    Message::MockAction,
                ));

            Column::new()
                .spacing(16)
                .push(current_header)
                .push(current_item)
                .push(next_header)
                .push(queue_list)
                .into()
        }
    };

    let content = Column::new()
        .spacing(16)
        .push(header)
        .push(Scrollable::new(body).height(Length::Fill));

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
            track.title.clone(),
            track.artist.clone(),
            track.image_url.as_deref(),
        )
    } else {
        (
            "No track playing".to_string(),
            "Spotifust".to_string(),
            None,
        )
    };

    let track_cover = view_image_or_icon(
        image_url,
        loaded_images,
        Icon::MusicNote,
        48.0,
        theme::RADIUS_MD,
    );

    let track_info = Row::new()
        .align_y(Alignment::Center)
        .spacing(12)
        .push(track_cover)
        .push(
            Column::new()
                .spacing(2)
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
        .push(icon_button_circle(Icon::Heart, Message::MockAction));

    let play_pause_icon = if playback.is_playing {
        Icon::Pause
    } else {
        Icon::Play
    };

    let controls = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .push(icon_button_plain(Icon::Shuffle, Message::MockAction))
        .push(icon_button_plain(Icon::SkipPrev, Message::SkipPrev))
        .push(icon_button_circle_accent(
            play_pause_icon,
            Message::TogglePlayback,
        ))
        .push(icon_button_plain(Icon::SkipNext, Message::SkipNext))
        .push(icon_button_plain(Icon::Repeat, Message::MockAction));

    let duration_ms = playback
        .current_track
        .as_ref()
        .map_or(225_000, |t| t.duration_ms);
    let progress_percent = if duration_ms > 0 {
        (playback.progress_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let seek_bar = slider(0.0..=1.0, progress_percent, Message::SeekTo)
        .step(0.001)
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
        .width(Length::Fixed(500.0))
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
        .step(0.01)
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
        Icon::X
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

    Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .push(
                Container::new(track_info)
                    .width(Length::Fixed(300.0))
                    .align_x(iced::alignment::Horizontal::Left),
            )
            .push(Space::new().width(Length::Fill))
            .push(center_controls)
            .push(Space::new().width(Length::Fill))
            .push(
                Container::new(right_utility_controls)
                    .width(Length::Fixed(300.0))
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

fn sidebar_item<'a>(
    title: impl Into<String>,
    subtitle: impl Into<String>,
    icon: Icon,
    active: bool,
    is_liked: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let title_str = title.into();
    let subtitle_str = subtitle.into();
    let icon_bg = if is_liked {
        theme::ACCENT
    } else {
        theme::SURFACE_CARD
    };

    let icon_color = if is_liked {
        Color::WHITE
    } else {
        theme::TEXT_SECONDARY
    };

    let icon_box = Container::new(icon.view_colored(18.0, icon_color))
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(44.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(icon_bg)),
            border: Border {
                radius: theme::RADIUS_MD.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let title_color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_PRIMARY
    };

    let details = Column::new()
        .spacing(2)
        .push(
            Text::new(title_str)
                .size(14)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(title_color),
        )
        .push(
            Text::new(subtitle_str)
                .size(12)
                .color(theme::TEXT_SECONDARY),
        );

    let content = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(icon_box)
        .push(details);

    Button::new(content)
        .padding(8)
        .width(Length::Fill)
        .on_press(on_press)
        .style(move |_theme, status| {
            let bg = if active {
                theme::SURFACE_ACTIVE
            } else {
                Color::TRANSPARENT
            };
            let base = iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    radius: theme::RADIUS_MD.into(),
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
        .spacing(2)
        .push(
            Text::new(title.to_string())
                .size(14)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(title_color),
        )
        .push(
            Text::new(subtitle.to_string())
                .size(12)
                .color(theme::TEXT_SECONDARY),
        );

    let content = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(icon_box)
        .push(details);

    Button::new(content)
        .padding(8)
        .width(Length::Fill)
        .on_press(on_press)
        .style(move |_theme, status| {
            let bg = if active {
                theme::SURFACE_ACTIVE
            } else {
                Color::TRANSPARENT
            };
            let base = iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    radius: theme::RADIUS_MD.into(),
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
            Text::new(title.to_string())
                .size(14)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        );

    Button::new(content)
        .padding(0)
        .width(Length::Fill)
        .height(Length::Fixed(56.0))
        .on_press(on_press)
        .style(|_theme, status| {
            let base = iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_CARD)),
                border: Border {
                    radius: theme::RADIUS_MD.into(),
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

fn media_card_with_image<'a>(
    title: &str,
    subtitle: &str,
    image_url: Option<&str>,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
    fallback_icon: Icon,
    on_press: Message,
) -> Element<'a, Message> {
    let cover = view_image_or_icon(
        image_url,
        loaded_images,
        fallback_icon,
        150.0,
        theme::RADIUS_MD,
    );

    let text_col = Column::new()
        .spacing(4)
        .push(
            Text::new(title.to_string())
                .size(15)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .push(
            Text::new(subtitle.to_string())
                .size(12)
                .color(theme::TEXT_SECONDARY),
        );

    let content = Column::new().spacing(12).push(cover).push(text_col);

    Button::new(content)
        .padding(12)
        .width(Length::Fixed(174.0))
        .on_press(on_press)
        .style(|_theme, status| {
            let base = iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_MAIN)),
                border: Border {
                    radius: theme::RADIUS_LG.into(),
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

fn format_duration(ms: u32) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}

#[allow(clippy::too_many_lines)]
fn view_search_results<'a>(
    results: &'a crate::api::search::SearchResults,
    is_searching: bool,
    loaded_images: &'a std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Element<'a, Message> {
    if is_searching {
        return Container::new(
            Text::new("Searching...")
                .size(16)
                .color(theme::TEXT_SECONDARY),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into();
    }

    if results.tracks.is_empty() && results.albums.is_empty() && results.artists.is_empty() {
        return Container::new(
            Text::new("Type to search for tracks, albums, or artists")
                .size(16)
                .color(theme::TEXT_SECONDARY),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into();
    }

    let mut tracks_col = Column::new().spacing(8);
    for (idx, track) in results.tracks.iter().enumerate() {
        let formatted_dur = format_duration(track.duration_ms);
        let uri = track.uri.clone();

        let track_cover = view_image_or_icon(
            track.image_url.as_deref(),
            loaded_images,
            Icon::MusicNote,
            40.0,
            theme::RADIUS_SM,
        );

        let row = Row::new()
            .align_y(Alignment::Center)
            .spacing(16)
            .push(
                Text::new(format!("{}", idx + 1))
                    .size(13)
                    .color(theme::TEXT_SECONDARY)
                    .width(Length::Fixed(24.0)),
            )
            .push(track_cover)
            .push(
                Column::new()
                    .spacing(2)
                    .push(
                        Text::new(&track.title)
                            .size(14)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_PRIMARY),
                    )
                    .push(
                        Text::new(format!("{} • {}", track.artist, track.album))
                            .size(12)
                            .color(theme::TEXT_SECONDARY),
                    ),
            )
            .push(Space::new().width(Length::Fill))
            .push(
                Text::new(formatted_dur)
                    .size(13)
                    .color(theme::TEXT_SECONDARY),
            );

        let track_btn = Button::new(Container::new(row).padding([6, 10]).width(Length::Fill))
            .padding(0)
            .on_press(Message::PlayTrack(uri))
            .style(|_theme, status| {
                let base = iced::widget::button::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    border: Border {
                        radius: theme::RADIUS_MD.into(),
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
            });

        tracks_col = tracks_col.push(track_btn);
    }

    let mut albums_row = Row::new().spacing(16);
    for album in results.albums.iter().take(5) {
        let subtitle = format!("{} • Album", album.artist_name);
        let a_id = album.id.clone();
        albums_row = albums_row.push(media_card_with_image(
            &album.name,
            &subtitle,
            album.image_url.as_deref(),
            loaded_images,
            Icon::Album,
            Message::SelectAlbum(a_id),
        ));
    }

    let mut content = Column::new().spacing(24);

    if !results.tracks.is_empty() {
        content = content
            .push(
                Text::new("Songs")
                    .size(22)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .push(tracks_col);
    }

    if !results.albums.is_empty() {
        content = content
            .push(
                Text::new("Albums")
                    .size(22)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .push(scroll_row(albums_row));
    }

    Scrollable::new(Container::new(content).width(Length::Fill).padding(24)).into()
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
