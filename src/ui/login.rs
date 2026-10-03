use crate::app::Message;
use crate::ui::icons::Icon;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Theme,
    widget::{Button, Column, Container, Image, Row, Text},
};

#[allow(clippy::too_many_lines)]
pub fn view(is_loading: bool, error: Option<&str>) -> Element<'_, Message> {
    let logo_handle = crate::ui::logo_handle();
    let logo = Image::new(logo_handle)
        .width(Length::Fixed(84.0))
        .height(Length::Fixed(84.0))
        .filter_method(iced::widget::image::FilterMethod::Linear);

    let title = Text::new("Spotifust")
        .size(40)
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        })
        .color(theme::TEXT_PRIMARY);

    let badge = Container::new(
        Text::new("HIGH FIDELITY")
            .size(10)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .color(theme::SPOTIFY_GREEN),
    )
    .padding([3, 8])
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(Color {
            r: theme::SPOTIFY_GREEN.r,
            g: theme::SPOTIFY_GREEN.g,
            b: theme::SPOTIFY_GREEN.b,
            a: 0.15,
        })),
        border: Border {
            color: Color {
                r: theme::SPOTIFY_GREEN.r,
                g: theme::SPOTIFY_GREEN.g,
                b: theme::SPOTIFY_GREEN.b,
                a: 0.4,
            },
            width: 1.0,
            radius: theme::RADIUS_PILL.into(),
        },
        ..Default::default()
    });

    let header_row = Row::new()
        .align_y(Alignment::Center)
        .spacing(12)
        .push(title)
        .push(badge);

    let subtitle = Text::new("Ultra-fast native Spotify client • Pure Rust • No web bloat")
        .size(14)
        .color(theme::TEXT_SECONDARY);

    let mut inner_col = Column::new()
        .spacing(20)
        .align_x(Alignment::Center)
        .push(logo)
        .push(header_row)
        .push(subtitle);

    if let Some(err) = error {
        let is_session_expired = err.to_lowercase().contains("expired")
            || err.to_lowercase().contains("expiró")
            || err.to_lowercase().contains("session");

        let (alert_bg, alert_border, alert_color) = if is_session_expired {
            (
                Color {
                    r: 0.961,
                    g: 0.620,
                    b: 0.043,
                    a: 0.12,
                },
                Color {
                    r: 0.961,
                    g: 0.620,
                    b: 0.043,
                    a: 0.45,
                },
                Color {
                    r: 0.984,
                    g: 0.749,
                    b: 0.141,
                    a: 1.0,
                },
            )
        } else {
            (
                Color {
                    r: theme::COLOR_ERROR.r,
                    g: theme::COLOR_ERROR.g,
                    b: theme::COLOR_ERROR.b,
                    a: 0.12,
                },
                Color {
                    r: theme::COLOR_ERROR.r,
                    g: theme::COLOR_ERROR.g,
                    b: theme::COLOR_ERROR.b,
                    a: 0.45,
                },
                Color {
                    r: 0.973,
                    g: 0.443,
                    b: 0.443,
                    a: 1.0,
                },
            )
        };

        let alert_icon = if is_session_expired {
            Icon::Clock
        } else {
            Icon::X
        };

        inner_col = inner_col.push(
            Container::new(
                Row::new()
                    .align_y(Alignment::Center)
                    .spacing(10)
                    .push(alert_icon.view_colored(16.0, alert_color))
                    .push(Text::new(err).color(alert_color).size(13).font(iced::Font {
                        weight: iced::font::Weight::Medium,
                        ..Default::default()
                    })),
            )
            .padding([12, 18])
            .width(Length::Fixed(420.0))
            .style(move |_theme: &Theme| iced::widget::container::Style {
                background: Some(Background::Color(alert_bg)),
                border: Border {
                    color: alert_border,
                    width: 1.0,
                    radius: theme::RADIUS_MD.into(),
                },
                ..Default::default()
            }),
        );
    }

    if is_loading {
        let loading_block = Column::new()
            .spacing(14)
            .align_x(Alignment::Center)
            .push(
                Row::new()
                    .align_y(Alignment::Center)
                    .spacing(10)
                    .push(Icon::Devices.view_colored(20.0, theme::SPOTIFY_GREEN))
                    .push(
                        Text::new("Awaiting browser authorization...")
                            .size(15)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .color(theme::TEXT_PRIMARY),
                    ),
            )
            .push(
                Text::new("Authorize Spotifust in your opened default web browser.")
                    .size(12)
                    .color(theme::TEXT_SECONDARY),
            )
            .push(
                Button::new(
                    Text::new("Cancel")
                        .size(12)
                        .font(iced::Font {
                            weight: iced::font::Weight::Medium,
                            ..Default::default()
                        })
                        .color(theme::TEXT_SECONDARY),
                )
                .on_press(Message::CancelLogin)
                .padding([6, 20])
                .style(|_theme: &Theme, status| {
                    let base = iced::widget::button::Style {
                        background: Some(Background::Color(theme::SURFACE_HOVER)),
                        border: Border {
                            radius: theme::RADIUS_PILL.into(),
                            color: theme::BORDER_SUBTLE,
                            width: 1.0,
                        },
                        ..Default::default()
                    };
                    match status {
                        iced::widget::button::Status::Hovered => iced::widget::button::Style {
                            background: Some(Background::Color(theme::SURFACE_ACTIVE)),
                            ..base
                        },
                        _ => base,
                    }
                }),
            );

        inner_col = inner_col.push(Container::new(loading_block).padding([10, 0]));
    } else {
        let login_btn = Button::new(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(10)
                .push(Icon::Play.view_colored(16.0, Color::BLACK))
                .push(
                    Text::new("LOG IN WITH SPOTIFY")
                        .size(14)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(Color::BLACK),
                ),
        )
        .on_press(Message::LoginRequested)
        .padding([16, 42])
        .style(|_theme: &Theme, status| {
            let base = iced::widget::button::Style {
                background: Some(Background::Color(theme::SPOTIFY_GREEN)),
                text_color: Color::BLACK,
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    ..Default::default()
                },
                ..Default::default()
            };

            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Background::Color(theme::SPOTIFY_GREEN_HOVER)),
                    ..base
                },
                iced::widget::button::Status::Pressed => iced::widget::button::Style {
                    background: Some(Background::Color(theme::SPOTIFY_GREEN_PRESSED)),
                    ..base
                },
                _ => base,
            }
        });

        inner_col = inner_col.push(Container::new(login_btn).padding([10, 0]));
    }

    let feature_badge = |icon: Icon, title: &'static str, subtitle: &'static str| {
        Container::new(
            Column::new()
                .spacing(3)
                .align_x(Alignment::Center)
                .push(icon.view_colored(18.0, theme::SPOTIFY_GREEN))
                .push(
                    Text::new(title)
                        .size(11)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(theme::TEXT_PRIMARY),
                )
                .push(Text::new(subtitle).size(10).color(theme::TEXT_TERTIARY)),
        )
        .padding([10, 14])
        .width(Length::Fixed(130.0))
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.02,
            })),
            border: Border {
                radius: theme::RADIUS_MD.into(),
                color: theme::BORDER_SUBTLE,
                width: 1.0,
            },
            ..Default::default()
        })
    };

    let features_row = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(feature_badge(Icon::MusicNote, "320 kbps", "Bitrate Audio"))
        .push(feature_badge(Icon::Devices, "< 25 MB", "Memory Usage"))
        .push(feature_badge(Icon::Lock, "PKCE Flow", "Keyring Vault"));

    inner_col = inner_col.push(Container::new(features_row).padding([10, 0]));

    let footer = Text::new("Powered by Librespot & RSpotify • Zero browser engine overhead")
        .size(11)
        .color(theme::TEXT_TERTIARY);

    inner_col = inner_col.push(footer);

    Container::new(inner_col)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BG_BASE)),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_view_idle_renders_without_panic() {
        let _ = view(false, None);
    }

    #[test]
    fn test_login_view_loading_renders_without_panic() {
        let _ = view(true, None);
    }

    #[test]
    fn test_login_view_error_renders_without_panic() {
        let _ = view(false, Some("Session expired. Please log in again."));
    }
}
