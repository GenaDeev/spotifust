use crate::app::{Message, NavigationItem, PlaybackState};
use crate::ui::icons::Icon;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Element, Length, Shadow, Theme, Vector,
    widget::{Button, Column, Container, Row, Space, Text, container, slider},
};

pub fn view<'a>(nav_item: &'a NavigationItem, playback: &'a PlaybackState) -> Element<'a, Message> {
    let sidebar = view_sidebar(*nav_item);
    let main_content = view_main_content(*nav_item);
    let playback_bar = view_playback_bar(playback);

    let top_section = Row::new()
        .push(sidebar)
        .push(main_content)
        .height(Length::Fill);

    let layout = Column::new().push(top_section).push(playback_bar);

    Container::new(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::BG_BASE)),
            ..Default::default()
        })
        .into()
}

fn view_sidebar(current_nav: NavigationItem) -> Element<'static, Message> {
    let logo = Row::new()
        .align_y(Alignment::Center)
        .spacing(12)
        .push(
            Container::new(Icon::Album.view(32.0)).style(|_theme: &Theme| container::Style {
                text_color: Some(theme::ACCENT),
                ..Default::default()
            }),
        )
        .push(
            Text::new("Spotifust")
                .size(24)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::ACCENT),
        );

    let nav_links = Column::new()
        .spacing(8)
        .push(nav_button(
            "Home",
            Icon::Home,
            current_nav == NavigationItem::Home,
            NavigationItem::Home,
        ))
        .push(nav_button(
            "Search",
            Icon::Search,
            current_nav == NavigationItem::Search,
            NavigationItem::Search,
        ))
        .push(nav_button(
            "Library",
            Icon::Library,
            current_nav == NavigationItem::Library,
            NavigationItem::Library,
        ));

    Container::new(
        Column::new()
            .push(logo)
            .push(Space::new().height(Length::Fixed(40.0)))
            .push(nav_links)
            .spacing(24)
            .padding(24),
    )
    .width(Length::Fixed(240.0))
    .height(Length::Fill)
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::SURFACE_0)),
        border: Border {
            color: theme::BORDER_SUBTLE,
            width: 1.0,
            radius: iced::border::Radius::default(),
        },
        ..Default::default()
    })
    .into()
}

fn nav_button(
    label: &str,
    icon: Icon,
    is_active: bool,
    target: NavigationItem,
) -> Element<'_, Message> {
    let text_color = if is_active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };

    let content = Row::new()
        .align_y(Alignment::Center)
        .spacing(16)
        .push(
            Container::new(icon.view(20.0)).style(move |_theme: &Theme| container::Style {
                text_color: Some(text_color),
                ..Default::default()
            }),
        )
        .push(
            Text::new(label)
                .size(15)
                .color(text_color)
                .font(iced::Font {
                    weight: if is_active {
                        iced::font::Weight::Bold
                    } else {
                        iced::font::Weight::Normal
                    },
                    ..Default::default()
                }),
        );

    Button::new(content)
        .width(Length::Fill)
        .padding(12)
        .on_press(Message::NavigationSelected(target))
        .style(move |_theme: &Theme, status| {
            let base = iced::widget::button::Style {
                text_color,
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            };

            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Background::Color(theme::SURFACE_2)),
                    text_color: theme::TEXT_PRIMARY,
                    ..base
                },
                _ => {
                    if is_active {
                        iced::widget::button::Style {
                            background: Some(Background::Color(theme::SURFACE_1)),
                            ..base
                        }
                    } else {
                        base
                    }
                }
            }
        })
        .into()
}

fn view_main_content<'a>(_nav: NavigationItem) -> Element<'a, Message> {
    // Placeholder for actual content views
    Container::new(
        Column::new()
            .push(
                Text::new("Good evening")
                    .size(32)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(theme::TEXT_PRIMARY),
            )
            .push(Space::new().height(Length::Fixed(24.0)))
            .push(
                Row::new()
                    .spacing(16)
                    .push(mock_card("Liked Songs", theme::ACCENT_DEEP))
                    .push(mock_card("Daily Mix 1", theme::SURFACE_2))
                    .push(mock_card("Discover Weekly", theme::SURFACE_2)),
            )
            .padding(32),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::BG_BASE)),
        ..Default::default()
    })
    .into()
}

fn mock_card(title: &str, bg: iced::Color) -> Element<'_, Message> {
    Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .push(
                Container::new(Icon::Heart.view(24.0))
                    .width(Length::Fixed(60.0))
                    .height(Length::Fixed(60.0))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(move |_theme: &Theme| container::Style {
                        background: Some(Background::Color(iced::Color::from_rgba8(
                            255, 255, 255, 0.1,
                        ))),
                        text_color: Some(theme::TEXT_PRIMARY),
                        ..Default::default()
                    }),
            )
            .push(Space::new().width(Length::Fixed(16.0)))
            .push(
                Text::new(title)
                    .size(16)
                    .color(theme::TEXT_PRIMARY)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    }),
            ),
    )
    .width(Length::Fixed(260.0))
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        shadow: Shadow {
            color: iced::Color::from_rgba8(0, 0, 0, 0.3),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    })
    .into()
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn view_playback_bar(playback: &PlaybackState) -> Element<'_, Message> {
    let play_icon = if playback.is_playing {
        Icon::Pause
    } else {
        Icon::Play
    };

    let play_btn = Button::new(play_icon.view(20.0))
        .padding(12)
        .on_press(Message::TogglePlayback)
        .style(|_theme: &Theme, status| {
            let base = iced::widget::button::Style {
                background: Some(Background::Color(theme::TEXT_PRIMARY)),
                text_color: theme::BG_BASE,
                border: Border {
                    radius: 22.0.into(),
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
    let shuffle = icon_button(Icon::Shuffle, Message::SkipPrev); // Mock action
    let repeat = icon_button(Icon::Repeat, Message::SkipNext); // Mock action

    let track_info = if let Some(track) = &playback.current_track {
        Row::new()
            .align_y(Alignment::Center)
            .spacing(16)
            .push(
                Container::new(Icon::MusicNote.view(24.0))
                    .width(Length::Fixed(56.0))
                    .height(Length::Fixed(56.0))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
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
                    .push(Text::new(&track.title).size(14).color(theme::TEXT_PRIMARY))
                    .push(
                        Text::new(&track.artist)
                            .size(12)
                            .color(theme::TEXT_SECONDARY),
                    ),
            )
            .push(icon_button(Icon::Heart, Message::SkipNext)) // Mock action
    } else {
        Row::new()
    };

    let progress_percent = if let Some(track) = &playback.current_track {
        (playback.progress_ms as f32) / (track.duration_ms as f32)
    } else {
        0.0
    };

    let format_time = |ms: u32| {
        let secs = ms / 1000;
        let mins = secs / 60;
        let rem_secs = secs % 60;
        format!("{mins}:{rem_secs:02}")
    };

    let current_time = format_time(playback.progress_ms);
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
                        .width(Length::Fixed(400.0))
                        .style(|_theme: &Theme, _status| iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    Background::Color(theme::TEXT_PRIMARY),
                                    Background::Color(theme::SURFACE_2),
                                ),
                                width: 4.0,
                                border: Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle { radius: 6.0 },
                                background: Background::Color(theme::TEXT_PRIMARY),
                                border_width: 0.0,
                                border_color: iced::Color::TRANSPARENT,
                            },
                        }),
                )
                .push(Text::new(total_time).size(11).color(theme::TEXT_SECONDARY)),
        );

    let extra_controls = Row::new()
        .align_y(Alignment::Center)
        .spacing(16)
        .push(icon_button(Icon::Queue, Message::SkipNext))
        .push(icon_button(Icon::Devices, Message::SkipNext))
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
                        .width(Length::Fixed(100.0))
                        .style(|_theme: &Theme, _status| iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    Background::Color(theme::TEXT_PRIMARY),
                                    Background::Color(theme::SURFACE_2),
                                ),
                                width: 4.0,
                                border: Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle { radius: 6.0 },
                                background: Background::Color(theme::TEXT_PRIMARY),
                                border_width: 0.0,
                                border_color: iced::Color::TRANSPARENT,
                            },
                        }),
                ),
        );

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
            .padding([15, 24]),
    )
    .width(Length::Fill)
    .height(Length::Fixed(90.0))
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(theme::SURFACE_0)),
        border: Border {
            color: theme::BORDER_SUBTLE,
            width: 1.0,
            radius: iced::border::Radius::default(),
        },
        shadow: Shadow {
            color: iced::Color::from_rgba8(0, 0, 0, 0.4),
            offset: Vector::new(0.0, -4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    })
    .into()
}

fn icon_button<'a>(icon: Icon, message: Message) -> Element<'a, Message> {
    Button::new(
        Container::new(icon.view(16.0)).style(|_theme: &Theme| container::Style {
            text_color: Some(theme::TEXT_SECONDARY),
            ..Default::default()
        }),
    )
    .padding(8)
    .on_press(message)
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
    })
    .into()
}
