use crate::app::{Message, NavigationItem, PlaybackState, RightPanelTab};
use crate::ui::icons::Icon;
use crate::ui::theme;
use iced::widget::mouse_area;
use iced::{
    Alignment, Background, Border, Element, Length, Theme,
    widget::{Button, Column, Container, Row, Scrollable, Space, Text, container, slider},
};

pub fn view<'a>(
    nav_item: &'a NavigationItem,
    playback: &'a PlaybackState,
    sidebar_width: f32,
    right_panel_width: f32,
    active_right_panel: Option<RightPanelTab>,
) -> Element<'a, Message> {
    let top_bar = view_top_bar(*nav_item);
    let sidebar = view_sidebar_panel(sidebar_width);
    let main_content = view_main_content(*nav_item);
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
            background: Some(Background::Color(theme::SURFACE_0)),
            ..Default::default()
        })
        .into()
}

#[allow(clippy::too_many_lines)]
fn view_top_bar(current_nav: NavigationItem) -> Element<'static, Message> {
    let logo_section = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(Icon::Play.view_colored(28.0, theme::ACCENT))
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
            .spacing(12)
            .push(Icon::Search.view_colored(20.0, theme::TEXT_SECONDARY))
            .push(
                Text::new("What do you want to play?")
                    .color(theme::TEXT_SECONDARY)
                    .size(14),
            ),
    )
    .height(Length::Fixed(48.0))
    .padding([0, 16])
    .width(Length::Fixed(360.0))
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::SURFACE_2)),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        text_color: Some(theme::TEXT_SECONDARY),
        ..Default::default()
    });

    let right_controls = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .push(
            Button::new(Text::new("Explore Premium").size(12).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            }))
            .padding([8, 16])
            .style(|_theme: &Theme, status| {
                let base = iced::widget::button::Style {
                    background: Some(Background::Color(iced::Color::TRANSPARENT)),
                    text_color: theme::TEXT_PRIMARY,
                    border: Border {
                        color: theme::TEXT_PRIMARY,
                        width: 1.0,
                        radius: 999.0.into(),
                    },
                    ..Default::default()
                };
                match status {
                    iced::widget::button::Status::Hovered => iced::widget::button::Style {
                        background: Some(Background::Color(theme::SURFACE_2)),
                        ..base
                    },
                    _ => base,
                }
            })
            .on_press(Message::MockAction),
        )
        .push(icon_button(Icon::Album, Message::MockAction)) // mock download/install
        .push(icon_button(Icon::Plus, Message::MockAction)) // mock bell
        .push(icon_button(Icon::User, Message::MockAction)) // mock friends
        .push(
            Button::new(
                Container::new(Text::new("G").size(14).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                }))
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(48.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            )
            .padding(0)
            .on_press(Message::MockAction)
            .style(|_theme: &Theme, status| {
                let base = iced::widget::button::Style {
                    background: Some(Background::Color(theme::SURFACE_2)),
                    text_color: theme::TEXT_PRIMARY,
                    border: Border {
                        radius: 16.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                match status {
                    iced::widget::button::Status::Hovered => iced::widget::button::Style {
                        background: Some(Background::Color(theme::SURFACE_1)),
                        ..base
                    },
                    _ => base,
                }
            }),
        );

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
    .height(Length::Fixed(64.0))
    .padding([0, 16])
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::SURFACE_0)),
        ..Default::default()
    })
    .into()
}

#[allow(clippy::too_many_lines)]
fn view_sidebar_panel(width: f32) -> Element<'static, Message> {
    let is_compact = width < 120.0;

    if is_compact {
        let mut list = Column::new().spacing(12).align_x(Alignment::Center);
        // Liked Songs compact icon
        list = list.push(
            Button::new(
                Container::new(Icon::Heart.view_colored(20.0, theme::BG_BASE))
                    .width(Length::Fixed(40.0))
                    .height(Length::Fixed(40.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::ACCENT)),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .padding(0)
            .on_press(Message::MockAction)
            .style(|_theme, status| {
                let base = iced::widget::button::Style {
                    background: Some(Background::Color(iced::Color::TRANSPARENT)),
                    ..Default::default()
                };
                match status {
                    iced::widget::button::Status::Hovered => iced::widget::button::Style {
                        background: Some(Background::Color(theme::SURFACE_1)),
                        ..base
                    },
                    _ => base,
                }
            }),
        );

        let library_items = [
            Icon::Album,     // Synthwave Architect
            Icon::User,      // The Midnight
            Icon::MusicNote, // Rustaceans Unite
        ];

        for icon in library_items {
            list = list.push(
                Button::new(
                    Container::new(icon.view_colored(20.0, theme::TEXT_SECONDARY))
                        .width(Length::Fixed(40.0))
                        .height(Length::Fixed(40.0))
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                        .style(|_theme| container::Style {
                            background: Some(Background::Color(theme::SURFACE_2)),
                            border: Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                )
                .padding(0)
                .on_press(Message::MockAction)
                .style(|_theme, status| {
                    let base = iced::widget::button::Style {
                        background: Some(Background::Color(iced::Color::TRANSPARENT)),
                        ..Default::default()
                    };
                    match status {
                        iced::widget::button::Status::Hovered => iced::widget::button::Style {
                            background: Some(Background::Color(theme::SURFACE_1)),
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
                .push(Icon::Library.view_colored(24.0, theme::TEXT_SECONDARY))
                .push(scrollable_list),
        )
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .padding([16, 0])
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::BG_BASE)),
            border: Border {
                radius: 8.0.into(),
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
                    .push(Icon::Library.view(24.0))
                    .push(Text::new("Your Library").size(16).font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })),
            )
            .padding([8, 12])
            .on_press(Message::MockAction)
            .style(|_theme: &Theme, status| {
                let base = iced::widget::button::Style {
                    text_color: theme::TEXT_SECONDARY,
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
        .push(icon_button(Icon::Plus, Message::MockAction))
        .push(icon_button(Icon::ChevronRight, Message::MockAction));

    let filters = Scrollable::new(
        Row::new()
            .spacing(8)
            .push(filter_chip("Playlists"))
            .push(filter_chip("Artists"))
            .push(filter_chip("Albums")),
    )
    .direction(iced::widget::scrollable::Direction::Horizontal(
        iced::widget::scrollable::Scrollbar::new(),
    ));

    let search_sort = Row::new()
        .align_y(Alignment::Center)
        .push(icon_button(Icon::Search, Message::MockAction))
        .push(Space::new().width(Length::Fill))
        .push(
            Button::new(
                Row::new()
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .push(Text::new("Recents").size(12).color(theme::TEXT_SECONDARY))
                    .push(Icon::ChevronDown.view(16.0)),
            )
            .padding(0)
            .on_press(Message::MockAction)
            .style(|_theme: &Theme, _status| iced::widget::button::Style {
                text_color: theme::TEXT_SECONDARY,
                ..Default::default()
            }),
        );

    let mut list = Column::new().spacing(0);
    list = list.push(library_row(
        "Liked Songs",
        "Playlist • 120 songs",
        true,
        true,
    ));

    let library_items = [
        ("Synthwave Architect", "Playlist • GenaDeev", false),
        ("The Midnight", "Artist", false),
        ("Rustaceans Unite", "Playlist • 45 songs", false),
        ("Daily Mix 1", "Made for you", false),
        ("Discover Weekly", "Playlist • Spotify", false),
    ];

    for (title, sub, is_pinned) in library_items {
        list = list.push(library_row(title, sub, is_pinned, false));
    }

    let scrollable_list = Scrollable::new(list).height(Length::Fill);

    Container::new(
        Column::new()
            .push(header)
            .push(
                Space::new()
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(12.0)),
            )
            .push(Container::new(filters).padding([0, 16]))
            .push(
                Space::new()
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(12.0)),
            )
            .push(Container::new(search_sort).padding([0, 16]))
            .push(
                Space::new()
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(8.0)),
            )
            .push(scrollable_list),
    )
    .width(Length::Fixed(width))
    .height(Length::Fill)
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::BG_BASE)),
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn filter_chip(label: &str) -> Element<'_, Message> {
    Button::new(Text::new(label).size(13))
        .padding([6, 12])
        .on_press(Message::MockAction)
        .style(|_theme: &Theme, status| {
            let base = iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_2)),
                text_color: theme::TEXT_PRIMARY,
                border: Border {
                    radius: 999.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            };
            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Background::Color(iced::Color {
                        r: 0.2,
                        g: 0.2,
                        b: 0.2,
                        a: 1.0,
                    })),
                    ..base
                },
                _ => base,
            }
        })
        .into()
}

fn library_row<'a>(
    title: &'a str,
    subtitle: &'a str,
    is_pinned: bool,
    is_liked: bool,
) -> Element<'a, Message> {
    let cover = Container::new(if is_liked {
        Icon::Heart.view_colored(24.0, theme::BG_BASE)
    } else {
        Icon::Album.view_colored(24.0, theme::TEXT_SECONDARY)
    })
    .width(Length::Fixed(48.0))
    .height(Length::Fixed(48.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(if is_liked {
            theme::ACCENT
        } else {
            theme::SURFACE_2
        })),
        text_color: Some(if is_liked {
            theme::BG_BASE
        } else {
            theme::TEXT_SECONDARY
        }),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let title_text = Text::new(title)
        .size(16)
        .color(if is_liked {
            theme::ACCENT
        } else {
            theme::TEXT_PRIMARY
        })
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        });

    let pin = if is_pinned {
        Icon::Pin.view(12.0)
    } else {
        Space::new()
            .width(Length::Fixed(0.0))
            .height(Length::Fixed(0.0))
            .into()
    };

    let content = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(cover)
        .push(
            Column::new().push(title_text).push(
                Row::new()
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .push(
                        Container::new(pin).style(|_theme: &Theme| container::Style {
                            text_color: Some(theme::ACCENT),
                            ..Default::default()
                        }),
                    )
                    .push(Text::new(subtitle).size(13).color(theme::TEXT_SECONDARY)),
            ),
        );

    Button::new(content)
        .width(Length::Fill)
        .padding([8, 16])
        .on_press(Message::MockAction)
        .style(|_theme: &Theme, status| {
            let base = iced::widget::button::Style {
                background: Some(Background::Color(iced::Color::TRANSPARENT)),
                ..Default::default()
            };
            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Background::Color(theme::SURFACE_1)),
                    ..base
                },
                _ => base,
            }
        })
        .into()
}

fn view_main_content<'a>(_nav: NavigationItem) -> Element<'a, Message> {
    let chips = Row::new()
        .spacing(8)
        .push(filter_chip("All"))
        .push(filter_chip("Music"))
        .push(filter_chip("Podcasts"));

    let featured_title = Text::new("Welcome back")
        .size(24)
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        })
        .color(theme::TEXT_PRIMARY);

    let main_layout = Column::new()
        .spacing(24)
        .push(chips)
        .push(featured_title)
        .push(
            Container::new(
                Column::new()
                    .spacing(12)
                    .push(
                        Text::new("Spotifust Native Desktop")
                            .size(18)
                            .color(theme::ACCENT)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            }),
                    )
                    .push(
                        Text::new(
                            "This is a modular, ultra-lightweight client built from scratch in Rust. Drag the borders of the panels to resize the layout just like Spotify!",
                        )
                        .size(14)
                        .color(theme::TEXT_SECONDARY),
                    ),
            )
            .padding(24)
            .width(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Background::Color(theme::SURFACE_1)),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );

    Container::new(main_layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::BG_BASE)),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .padding(24)
        .into()
}

#[allow(dead_code)]
fn icon_button_circle<'a>(icon: Icon, message: Message, disabled: bool) -> Element<'a, Message> {
    Button::new(
        Container::new(icon.view_colored(
            16.0,
            if disabled {
                theme::TEXT_TERTIARY
            } else {
                theme::TEXT_PRIMARY
            },
        ))
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .padding(0)
    .on_press(message)
    .style(move |_theme: &Theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(iced::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.7,
            })),
            border: Border {
                radius: 16.0.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        if disabled {
            return base;
        }
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(theme::SURFACE_2)),
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
    is_active: bool,
) -> Element<'a, Message> {
    Button::new(
        Container::new(icon.view_colored(
            24.0,
            if is_active {
                theme::BG_BASE
            } else {
                theme::TEXT_PRIMARY
            },
        ))
        .width(Length::Fixed(48.0))
        .height(Length::Fixed(48.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(message)
    .style(move |_theme: &Theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(if is_active {
                theme::TEXT_PRIMARY
            } else {
                theme::SURFACE_2
            })),
            border: Border {
                radius: 24.0.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(if is_active {
                    theme::TEXT_PRIMARY
                } else {
                    iced::Color {
                        r: 0.2,
                        g: 0.2,
                        b: 0.2,
                        a: 1.0,
                    }
                })),
                ..base
            },
            _ => base,
        }
    })
    .into()
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn view_playback_bar(
    playback: &PlaybackState,
    active_right_panel: Option<RightPanelTab>,
) -> Element<'_, Message> {
    let play_icon = if playback.is_playing {
        Icon::Pause
    } else {
        Icon::Play
    };

    let play_btn = Button::new(
        Container::new(play_icon.view_colored(16.0, theme::BG_BASE))
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(32.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(Message::TogglePlayback)
    .style(|_theme: &Theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(theme::TEXT_PRIMARY)),
            text_color: theme::BG_BASE,
            border: Border {
                radius: 16.0.into(),
                ..Default::default()
            },
            ..Default::default()
        };

        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(iced::Color::WHITE)),
                ..base
            },
            _ => base,
        }
    });

    let skip_prev = icon_button(Icon::SkipPrev, Message::SkipPrev);
    let skip_next = icon_button(Icon::SkipNext, Message::SkipNext);
    let shuffle = icon_button_active(Icon::Shuffle, Message::SkipPrev, false);
    let repeat = icon_button_active(Icon::Repeat, Message::SkipNext, false);

    let track_info = if let Some(track) = &playback.current_track {
        Row::new()
            .align_y(Alignment::Center)
            .spacing(16)
            .push(
                Container::new(Icon::MusicNote.view_colored(24.0, theme::TEXT_SECONDARY))
                    .width(Length::Fixed(56.0))
                    .height(Length::Fixed(56.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_2)),
                        text_color: Some(theme::TEXT_SECONDARY),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .push(
                Column::new()
                    .spacing(2)
                    .push(
                        Text::new(&track.title)
                            .size(14)
                            .color(theme::TEXT_PRIMARY)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            }),
                    )
                    .push(
                        Text::new(if track.album.is_empty() {
                            track.artist.clone()
                        } else {
                            format!("{} — {}", track.artist, track.album)
                        })
                        .size(12)
                        .color(theme::TEXT_SECONDARY),
                    ),
            )
            .push(icon_button_active(Icon::Heart, Message::MockAction, true)) // Mock active liked
    } else {
        Row::new()
    };

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
        .spacing(8)
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(24)
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
                        .width(Length::Fill)
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
                                    Background::Color(iced::Color {
                                        r: 0.3,
                                        g: 0.3,
                                        b: 0.3,
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
                                border_color: iced::Color::TRANSPARENT,
                            },
                        }),
                )
                .push(Text::new(total_time).size(11).color(theme::TEXT_SECONDARY)),
        );

    let now_playing_active = active_right_panel == Some(RightPanelTab::NowPlaying);
    let queue_active = active_right_panel == Some(RightPanelTab::Queue);

    let extra_controls = Row::new()
        .align_y(Alignment::Center)
        .spacing(16)
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
        .push(icon_button(Icon::Devices, Message::MockAction))
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(8)
                .push(
                    Container::new(Icon::Volume.view(16.0)).style(|_theme: &Theme| {
                        container::Style {
                            text_color: Some(theme::TEXT_SECONDARY),
                            ..Default::default()
                        }
                    }),
                )
                .push(
                    slider(0.0..=1.0, playback.volume, Message::VolumeChanged)
                        .step(0.01_f32)
                        .width(Length::Fixed(100.0))
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
                                    Background::Color(iced::Color {
                                        r: 0.3,
                                        g: 0.3,
                                        b: 0.3,
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
                                border_color: iced::Color::TRANSPARENT,
                            },
                        }),
                ),
        )
        .push(icon_button(Icon::ChevronRight, Message::MockAction)); // Fullscreen

    Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .push(Container::new(track_info).width(Length::FillPortion(1)))
            .push(
                Container::new(playback_controls)
                    .width(Length::FillPortion(2))
                    .center_x(Length::Fill),
            )
            .push(
                Container::new(extra_controls)
                    .width(Length::FillPortion(1))
                    .align_x(iced::alignment::Horizontal::Right),
            )
            .padding([0, 16]),
    )
    .width(Length::Fill)
    .height(Length::Fixed(90.0))
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::SURFACE_0)),
        ..Default::default()
    })
    .into()
}

fn icon_button<'a>(icon: Icon, message: Message) -> Element<'a, Message> {
    icon_button_active(icon, message, false)
}

fn icon_button_active<'a>(icon: Icon, message: Message, is_active: bool) -> Element<'a, Message> {
    let color = if is_active {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    };
    Button::new(Container::new(icon.view_colored(16.0, color)))
        .padding(8)
        .on_press(message)
        .style(move |_theme: &Theme, status| {
            let base = iced::widget::button::Style {
                text_color: color,
                background: Some(Background::Color(iced::Color::TRANSPARENT)),
                ..Default::default()
            };

            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    text_color: if is_active {
                        theme::ACCENT_HOVER
                    } else {
                        theme::TEXT_PRIMARY
                    },
                    ..base
                },
                _ => base,
            }
        })
        .into()
}

fn view_drag_handle<'a>(is_sidebar: bool) -> Element<'a, Message> {
    let msg = if is_sidebar {
        Message::StartSidebarDrag
    } else {
        Message::StartRightPanelDrag
    };

    let handle_line = Container::new(Space::new())
        .width(Length::Fixed(1.0))
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_2)),
            ..Default::default()
        });

    let area = mouse_area(
        Container::new(handle_line)
            .width(Length::Fixed(6.0))
            .height(Length::Fill)
            .center_x(Length::Fill),
    )
    .on_press(msg)
    .interaction(iced::mouse::Interaction::ResizingHorizontally);

    area.into()
}

#[allow(clippy::too_many_lines)]
fn view_right_panel<'a>(view: Option<RightPanelTab>, width: f32) -> Element<'a, Message> {
    let Some(tab) = view else {
        return Space::new()
            .width(Length::Fixed(0.0))
            .height(Length::Fixed(0.0))
            .into();
    };

    let title = match tab {
        RightPanelTab::NowPlaying => "Now Playing",
        RightPanelTab::Queue => "Play Queue",
        RightPanelTab::Lyrics => "Lyrics",
    };

    let header = Row::new()
        .align_y(Alignment::Center)
        .push(
            Text::new(title)
                .size(16)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::TEXT_PRIMARY),
        )
        .push(Space::new().width(Length::Fill))
        .push(
            Button::new(Icon::X.view_colored(14.0, theme::TEXT_SECONDARY))
                .padding(6)
                .on_press(Message::ToggleRightPanel(tab))
                .style(|_theme, status| {
                    let base = iced::widget::button::Style {
                        background: Some(Background::Color(iced::Color::TRANSPARENT)),
                        ..Default::default()
                    };
                    match status {
                        iced::widget::button::Status::Hovered => iced::widget::button::Style {
                            background: Some(Background::Color(theme::SURFACE_2)),
                            ..base
                        },
                        _ => base,
                    }
                }),
        );

    let content: Element<'_, Message> = match tab {
        RightPanelTab::NowPlaying => Column::new()
            .spacing(16)
            .push(
                Container::new(Icon::MusicNote.view_colored(48.0, theme::TEXT_SECONDARY))
                    .width(Length::Fill)
                    .height(Length::Fixed(240.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(theme::SURFACE_2)),
                        border: Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .push(
                Text::new("Neon Nights")
                    .size(18)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .push(
                Text::new("Synthwave Architect")
                    .size(14)
                    .color(theme::TEXT_SECONDARY),
            )
            .into(),
        RightPanelTab::Queue => Column::new()
            .spacing(12)
            .push(
                Text::new("Now Playing")
                    .size(12)
                    .color(theme::TEXT_SECONDARY),
            )
            .push(
                Row::new()
                    .spacing(8)
                    .push(Icon::MusicNote.view_colored(16.0, theme::ACCENT))
                    .push(Text::new("Neon Nights").size(14).color(theme::ACCENT)),
            )
            .push(Space::new().height(Length::Fixed(8.0)))
            .push(
                Text::new("Next In Queue")
                    .size(12)
                    .color(theme::TEXT_SECONDARY),
            )
            .push(
                Text::new("1. Retro Wave — Miami Nights")
                    .size(13)
                    .color(theme::TEXT_PRIMARY),
            )
            .push(
                Text::new("2. Laser Diodes — Grid Runner")
                    .size(13)
                    .color(theme::TEXT_PRIMARY),
            )
            .into(),
        RightPanelTab::Lyrics => Scrollable::new(
            Column::new()
                .spacing(16)
                .push(
                    Text::new("Driving through the neon glow...")
                        .size(16)
                        .color(theme::TEXT_PRIMARY),
                )
                .push(
                    Text::new("Synth waves moving very slow...")
                        .size(16)
                        .color(theme::TEXT_PRIMARY),
                )
                .push(
                    Text::new("Gridlines stretching to the sky...")
                        .size(16)
                        .color(theme::TEXT_SECONDARY),
                )
                .push(
                    Text::new("Underneath the purple sky...")
                        .size(16)
                        .color(theme::TEXT_SECONDARY),
                ),
        )
        .into(),
    };

    let panel_layout = Column::new().spacing(16).push(header).push(content);

    Container::new(panel_layout)
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .padding(16)
        .style(|_theme| container::Style {
            background: Some(Background::Color(theme::BG_BASE)),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
