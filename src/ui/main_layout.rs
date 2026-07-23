use crate::app::{Message, NavigationItem, PlaybackState, RightPanelTab};
use crate::ui::icons::Icon;
use crate::ui::theme;
use iced::widget::mouse_area;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Theme,
    widget::{Button, Column, Container, Image, Row, Scrollable, Space, Text, container, slider},
};

const LOGO_BYTES: &[u8] = include_bytes!("../../assets/spotifust.png");

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
    selected_playlist: Option<&'a crate::app::SelectedPlaylistState>,
) -> Element<'a, Message> {
    let top_bar = view_top_bar(*nav_item, user_profile);
    let sidebar = view_sidebar_panel(sidebar_width, user_playlists, user_albums);
    let main_content = view_main_content(*nav_item, selected_playlist, user_albums, user_top_tracks);
    let right_panel = view_right_panel(active_right_panel, right_panel_width);
    let playback_bar = view_playback_bar(playback, active_right_panel);

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
fn view_top_bar(
    current_nav: NavigationItem,
    user_profile: Option<&crate::api::user::UserProfile>,
) -> Element<'static, Message> {
    let initial_letter = user_profile
        .and_then(|p| p.display_name.chars().next())
        .map_or("G".to_string(), |c| c.to_uppercase().to_string());

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

    let search_bar = Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .spacing(10)
            .push(Icon::Search.view_colored(18.0, theme::TEXT_SECONDARY))
            .push(
                Text::new("What do you want to play?")
                    .color(theme::TEXT_SECONDARY)
                    .size(14),
            ),
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

    let explore_premium_btn = Button::new(
        Container::new(
            Text::new("Explore Premium")
                .size(13)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .height(Length::Fixed(40.0))
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding([0, 18])
    .height(Length::Fixed(40.0))
    .style(|_theme: &Theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            text_color: theme::TEXT_PRIMARY,
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
                    color: theme::BORDER_STRONG,
                    width: 1.0,
                    radius: theme::RADIUS_PILL.into(),
                },
                ..base
            },
            _ => base,
        }
    })
    .on_press(Message::MockAction);

    let user_avatar_btn = Button::new(
        Container::new(
            Text::new(initial_letter)
                .size(14)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(Color::BLACK),
        )
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(40.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|_theme| container::Style {
            background: Some(Background::Color(theme::ACCENT)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
    )
    .padding(0)
    .on_press(Message::MockAction)
    .style(|_theme, _status| iced::widget::button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        ..Default::default()
    });

    let right_controls = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(explore_premium_btn)
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
fn view_sidebar_panel(
    width: f32,
    playlists: &[crate::api::playlist::PlaylistSummary],
    albums: &[crate::api::album::AlbumSummary],
) -> Element<'static, Message> {
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
            .on_press(Message::MockAction)
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
            Icon::Album,
            Icon::User,
            Icon::MusicNote,
        ];

        for icon in library_items {
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
                .on_press(Message::MockAction)
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
            .on_press(Message::MockAction)
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
        .push(filter_chip("Playlists", true))
        .push(filter_chip("Artists", false))
        .push(filter_chip("Albums", false));

    let mut list = Column::new().spacing(4);

    let liked_item = sidebar_item(
        "Liked Songs",
        "Playlist • 142 songs",
        Icon::Heart,
        true,
        true,
        Message::MockAction,
    );
    list = list.push(liked_item);

    if playlists.is_empty() && albums.is_empty() {
        let items = [
            ("Synthwave Architect", "Album • The Midnight", Icon::Album, false),
            ("Rustaceans Unite", "Playlist • Spotifust", Icon::MusicNote, false),
            ("Chill Lofi Beats", "Playlist • Spotifust", Icon::Queue, false),
            ("Gunship", "Artist", Icon::User, false),
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
    } else {
        for p in playlists {
            let sub = format!("Playlist • {} tracks", p.total_tracks);
            let p_id = p.id.clone();
            list = list.push(sidebar_item(
                p.name.clone(),
                sub,
                Icon::MusicNote,
                false,
                false,
                Message::SelectPlaylist(p_id),
            ));
        }
        for a in albums {
            let sub = format!("Album • {}", a.artist_name);
            list = list.push(sidebar_item(
                a.name.clone(),
                sub,
                Icon::Album,
                false,
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

#[allow(clippy::too_many_lines)]
fn view_main_content<'a>(
    current_nav: NavigationItem,
    selected_playlist: Option<&'a crate::app::SelectedPlaylistState>,
    user_albums: &'a [crate::api::album::AlbumSummary],
    user_top_tracks: &'a [crate::api::tracks::TopTrack],
) -> Element<'a, Message> {
    if let Some(sp) = selected_playlist {
        let playlist_header = Column::new()
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
            );

        let content_body: Element<'a, Message> = if sp.is_loading {
            Container::new(
                Text::new("Loading playlist tracks...")
                    .size(15)
                    .color(theme::TEXT_SECONDARY),
            )
            .padding(32)
            .into()
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

            // Table Header
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
                .on_press(Message::MockAction)
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

        let scrollable = Scrollable::new(
            Container::new(page_column).padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
                left: 0.0,
            }),
        )
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

    let quick_grid = Column::new()
        .spacing(12)
        .push(
            Row::new()
                .spacing(12)
                .push(quick_card("Liked Songs", Icon::Heart, true))
                .push(quick_card("Synthwave Architect", Icon::Album, false))
                .push(quick_card("Rustaceans Unite", Icon::MusicNote, false)),
        )
        .push(
            Row::new()
                .spacing(12)
                .push(quick_card("Chill Lofi Beats", Icon::Queue, false))
                .push(quick_card("Deep Focus", Icon::Search, false))
                .push(quick_card("Top Gaming Tracks", Icon::Play, false)),
        );

    let section_1_header = Row::new()
        .align_y(Alignment::Center)
        .push(
            Text::new("Made For You")
                .size(22)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .push(Space::new().width(Length::Fill))
        .push(
            Text::new("Show all")
                .size(13)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_SECONDARY),
        );

    let section_1_cards = if user_top_tracks.is_empty() {
        Row::new()
            .spacing(16)
            .push(media_card("Daily Mix 1", "Gunship, The Midnight, Carpenter Brut", Icon::MusicNote))
            .push(media_card("Discover Weekly", "Your weekly mixtape of fresh music.", Icon::Search))
            .push(media_card("Release Radar", "Catch all the latest music from artists you follow.", Icon::Album))
            .push(media_card("Chill Mix", "Lofi and ambient beats to keep you focused.", Icon::Queue))
    } else {
        let mut row = Row::new().spacing(16);
        for track in user_top_tracks.iter().take(4) {
            let subtitle = format!("{} • Track", track.artist);
            row = row.push(media_card(&track.title, &subtitle, Icon::MusicNote));
        }
        row
    };

    let section_2_header = Row::new()
        .align_y(Alignment::Center)
        .push(
            Text::new("Recently Played")
                .size(22)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .push(Space::new().width(Length::Fill))
        .push(
            Text::new("Show all")
                .size(13)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_SECONDARY),
        );

    let section_2_cards = if user_albums.is_empty() {
        Row::new()
            .spacing(16)
            .push(media_card("Endless Summer", "The Midnight • Album", Icon::Album))
            .push(media_card("Dark All Day", "GUNSHIP • Album", Icon::MusicNote))
            .push(media_card("Techno Bunker", "Hard hitting synth and techno tracks.", Icon::Queue))
            .push(media_card("Coding Mode", "Zero distractions, pure synthwave.", Icon::Play))
    } else {
        let mut row = Row::new().spacing(16);
        for a in user_albums.iter().take(4) {
            let subtitle = format!("{} • Album", a.artist_name);
            row = row.push(media_card(&a.name, &subtitle, Icon::Album));
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
        .push(section_1_cards)
        .push(section_2_header)
        .push(section_2_cards);

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

#[allow(clippy::too_many_lines)]
fn view_right_panel(
    active_tab: Option<RightPanelTab>,
    width: f32,
) -> Element<'static, Message> {
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

    let body: Element<'static, Message> = match tab {
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

            Column::new()
                .spacing(16)
                .push(lyrics_card)
                .into()
        }
        RightPanelTab::NowPlaying => {
            let art_placeholder = Container::new(Icon::Album.view_colored(64.0, theme::TEXT_SECONDARY))
                .width(Length::Fill)
                .height(Length::Fixed(240.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(theme::SURFACE_CARD)),
                    border: Border {
                        radius: theme::RADIUS_LG.into(),
                        color: theme::BORDER_SUBTLE,
                        width: 1.0,
                    },
                    ..Default::default()
                });

            let track_title = Text::new("Synthetic Horizon")
                .size(20)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY);

            let artist_name = Text::new("Spotifust Audio Engine")
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

            let current_item = sidebar_item(
                "Synthetic Horizon",
                "Spotifust Audio Engine",
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
fn view_playback_bar(
    playback: &PlaybackState,
    active_right_panel: Option<RightPanelTab>,
) -> Element<'static, Message> {
    let (track_name, artist_name) = if let Some(track) = &playback.current_track {
        (track.title.clone(), track.artist.clone())
    } else {
        ("No track playing".to_string(), "Spotifust".to_string())
    };

    let track_info = Row::new()
        .align_y(Alignment::Center)
        .spacing(12)
        .push(
            Container::new(Icon::MusicNote.view_colored(20.0, theme::TEXT_SECONDARY))
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(48.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(theme::SURFACE_CARD)),
                    border: Border {
                        radius: theme::RADIUS_MD.into(),
                        color: theme::BORDER_SUBTLE,
                        width: 1.0,
                    },
                    ..Default::default()
                }),
        )
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
                .push(
                    Text::new(artist_name)
                        .size(11)
                        .color(theme::TEXT_SECONDARY),
                ),
        )
        .push(icon_button_active(Icon::Heart, Message::MockAction, true));

    let play_icon = if playback.is_playing {
        Icon::Pause
    } else {
        Icon::Play
    };

    let play_btn = Button::new(
        Container::new(play_icon.view_colored(16.0, Color::BLACK))
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(Message::TogglePlayback)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(theme::ACCENT)),
            text_color: Color::BLACK,
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
            iced::widget::button::Status::Pressed => iced::widget::button::Style {
                background: Some(Background::Color(theme::ACCENT_PRESSED)),
                ..base
            },
            _ => base,
        }
    });

    let shuffle = icon_button(Icon::Shuffle, Message::MockAction);
    let skip_prev = icon_button(Icon::SkipPrev, Message::SkipPrev);
    let skip_next = icon_button(Icon::SkipNext, Message::SkipNext);
    let repeat = icon_button(Icon::Repeat, Message::MockAction);

    let progress_percent = if let Some(track) = &playback.current_track {
        if track.duration_ms > 0 {
            ((playback.progress_ms as f32) / (track.duration_ms as f32)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    } else {
        0.0
    };

    let format_time = |ms: u32| {
        let secs = ms / 1000;
        let mins = secs / 60;
        let rem_secs = secs % 60;
        format!("{mins}:{rem_secs:02}")
    };

    let display_progress_ms = if let Some(track) = &playback.current_track {
        if track.duration_ms > 0 {
            playback.progress_ms.min(track.duration_ms)
        } else {
            playback.progress_ms
        }
    } else {
        playback.progress_ms
    };

    let current_time = format_time(display_progress_ms);
    let total_time = format_time(playback.current_track.as_ref().map_or(0, |t| t.duration_ms));

    let playback_controls = Column::new()
        .align_x(Alignment::Center)
        .spacing(6)
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(20)
                .push(shuffle)
                .push(skip_prev)
                .push(play_btn)
                .push(skip_next)
                .push(repeat),
        )
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(8)
                .push(
                    Text::new(current_time)
                        .size(11)
                        .color(theme::TEXT_SECONDARY),
                )
                .push(
                    slider(0.0..=1.0, progress_percent, Message::SeekTo)
                        .step(0.001_f32)
                        .width(Length::Fixed(460.0))
                        .style(|_theme: &Theme, status| iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    Background::Color(
                                        if status == iced::widget::slider::Status::Hovered {
                                            theme::ACCENT_HOVER
                                        } else {
                                            theme::ACCENT
                                        },
                                    ),
                                    Background::Color(Color {
                                        r: 0.25,
                                        g: 0.25,
                                        b: 0.25,
                                        a: 1.0,
                                    }),
                                ),
                                width: 4.0,
                                border: Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle {
                                    radius: if status == iced::widget::slider::Status::Hovered {
                                        6.0
                                    } else {
                                        0.0
                                    },
                                },
                                background: Background::Color(theme::TEXT_PRIMARY),
                                border_width: 0.0,
                                border_color: Color::TRANSPARENT,
                            },
                        }),
                )
                .push(Text::new(total_time).size(11).color(theme::TEXT_SECONDARY)),
        );

    let now_playing_active = active_right_panel == Some(RightPanelTab::NowPlaying);
    let queue_active = active_right_panel == Some(RightPanelTab::Queue);

    let extra_controls = Row::new()
        .align_y(Alignment::Center)
        .spacing(12)
        .push(icon_button_active(
            Icon::Album,
            Message::ToggleRightPanel(RightPanelTab::NowPlaying),
            now_playing_active,
        ))
        .push(icon_button_active(
            Icon::Queue,
            Message::ToggleRightPanel(RightPanelTab::Queue),
            queue_active,
        ))
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(8)
                .push(
                    Container::new(Icon::Volume.view_colored(16.0, theme::TEXT_SECONDARY)),
                )
                .push(
                    slider(0.0..=1.0, playback.volume, Message::VolumeChanged)
                        .step(0.01_f32)
                        .width(Length::Fixed(96.0))
                        .style(|_theme: &Theme, status| iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    Background::Color(
                                        if status == iced::widget::slider::Status::Hovered {
                                            theme::ACCENT_HOVER
                                        } else {
                                            theme::TEXT_PRIMARY
                                        },
                                    ),
                                    Background::Color(Color {
                                        r: 0.25,
                                        g: 0.25,
                                        b: 0.25,
                                        a: 1.0,
                                    }),
                                ),
                                width: 4.0,
                                border: Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle {
                                    radius: if status == iced::widget::slider::Status::Hovered {
                                        5.0
                                    } else {
                                        0.0
                                    },
                                },
                                background: Background::Color(theme::TEXT_PRIMARY),
                                border_width: 0.0,
                                border_color: Color::TRANSPARENT,
                            },
                        }),
                ),
        );

    Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .push(Container::new(track_info).width(Length::Fixed(280.0)))
            .push(Space::new().width(Length::Fill))
            .push(playback_controls)
            .push(Space::new().width(Length::Fill))
            .push(Container::new(extra_controls).width(Length::Fixed(240.0))),
    )
    .width(Length::Fill)
    .height(Length::Fixed(80.0))
    .padding([0, 24])
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::BG_BASE)),
        ..Default::default()
    })
    .into()
}

// --- Helper UI Widgets ---

fn view_drag_handle(is_left: bool) -> Element<'static, Message> {
    let handle = Container::new(Space::new())
        .width(Length::Fixed(4.0))
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        });

    mouse_area(handle)
        .on_press(if is_left {
            Message::StartSidebarDrag
        } else {
            Message::StartRightPanelDrag
        })
        .interaction(iced::mouse::Interaction::ResizingHorizontally)
        .into()
}

fn icon_button(icon: Icon, on_press: Message) -> Element<'static, Message> {
    Button::new(icon.view_colored(18.0, theme::TEXT_SECONDARY))
        .padding(8)
        .on_press(on_press)
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
        })
        .into()
}

fn icon_button_active(icon: Icon, on_press: Message, active: bool) -> Element<'static, Message> {
    let color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    };
    Button::new(icon.view_colored(18.0, color))
        .padding(8)
        .on_press(on_press)
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
        })
        .into()
}

fn icon_button_circle(icon: Icon, on_press: Message) -> Element<'static, Message> {
    Button::new(
        Container::new(icon.view_colored(16.0, theme::TEXT_SECONDARY))
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(on_press)
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

fn icon_button_circle_active(icon: Icon, on_press: Message, active: bool) -> Element<'static, Message> {
    let (bg, icon_color) = if active {
        (theme::SURFACE_ACTIVE, theme::TEXT_PRIMARY)
    } else {
        (theme::SURFACE_CARD, theme::TEXT_SECONDARY)
    };

    Button::new(
        Container::new(icon.view_colored(18.0, icon_color))
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .align_x(iced::alignment::Horizontal::Center)
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
                background: Some(Background::Color(theme::SURFACE_HOVER)),
                ..base
            },
            _ => base,
        }
    })
    .into()
}

fn filter_chip(label: &'static str, active: bool) -> Element<'static, Message> {
    let (bg, text_color) = if active {
        (theme::ACCENT, Color::BLACK)
    } else {
        (theme::SURFACE_CARD, theme::TEXT_PRIMARY)
    };

    Button::new(
        Text::new(label)
            .size(13)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .color(text_color),
    )
    .padding([8, 16])
    .on_press(Message::MockAction)
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
                background: Some(Background::Color(if active { theme::ACCENT_HOVER } else { theme::SURFACE_HOVER })),
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

fn quick_card(title: &'static str, icon: Icon, is_liked: bool) -> Element<'static, Message> {
    let icon_bg = if is_liked {
        theme::ACCENT
    } else {
        theme::SURFACE_CARD
    };

    let icon_box = Container::new(icon.view_colored(20.0, Color::WHITE))
        .width(Length::Fixed(56.0))
        .height(Length::Fixed(56.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(icon_bg)),
            border: Border {
                radius: theme::RADIUS_SM.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let content = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(icon_box)
        .push(
            Text::new(title)
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
        .on_press(Message::MockAction)
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

fn media_card<'a>(
    title: impl Into<String>,
    subtitle: impl Into<String>,
    icon: Icon,
) -> Element<'a, Message> {
    let title_str = title.into();
    let subtitle_str = subtitle.into();

    let cover = Container::new(icon.view_colored(36.0, theme::TEXT_SECONDARY))
        .width(Length::Fill)
        .height(Length::Fixed(150.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|_theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: theme::RADIUS_MD.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let text_col = Column::new()
        .spacing(4)
        .push(
            Text::new(title_str)
                .size(15)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .push(
            Text::new(subtitle_str)
                .size(12)
                .color(theme::TEXT_SECONDARY),
        );

    let content = Column::new()
        .spacing(12)
        .push(cover)
        .push(text_col);

    Button::new(content)
        .padding(12)
        .width(Length::Fixed(180.0))
        .on_press(Message::MockAction)
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
