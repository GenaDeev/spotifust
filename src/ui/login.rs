use crate::app::Message;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Theme,
    widget::{Button, Column, Container, Image, Row, Text},
};

const LOGO_BYTES: &[u8] = include_bytes!("../../assets/spotifust.png");

#[allow(clippy::too_many_lines)]
pub fn view<'a>(
    _username: &'a str,
    _password: &'a str,
    is_loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    let logo_handle = iced::widget::image::Handle::from_bytes(LOGO_BYTES);
    let logo = Image::new(logo_handle)
        .width(Length::Fixed(96.0))
        .height(Length::Fixed(96.0))
        .filter_method(iced::widget::image::FilterMethod::Linear);

    let title = Text::new("Spotifust")
        .size(48)
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        })
        .color(theme::TEXT_PRIMARY);

    let badge = Container::new(
        Text::new("DESKTOP")
            .size(10)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .color(theme::ACCENT),
    )
    .padding([3, 8])
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(Color {
            r: theme::ACCENT.r,
            g: theme::ACCENT.g,
            b: theme::ACCENT.b,
            a: 0.15,
        })),
        border: Border {
            color: theme::ACCENT,
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

    let subtitle = Text::new("Listen to millions of songs without audio limits.")
        .size(15)
        .color(theme::TEXT_SECONDARY);

    let mut inner_col = Column::new()
        .spacing(16)
        .align_x(Alignment::Center)
        .push(logo)
        .push(header_row)
        .push(subtitle);

    if let Some(err) = error {
        inner_col = inner_col.push(
            Container::new(
                Row::new()
                    .align_y(Alignment::Center)
                    .spacing(8)
                    .push(Text::new("⚠").size(14).color(Color::from_rgb8(255, 100, 100)))
                    .push(
                        Text::new(err)
                            .color(Color::from_rgb8(255, 120, 120))
                            .size(13),
                    ),
            )
            .padding([12, 20])
            .style(|_theme: &Theme| iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba8(255, 80, 80, 0.12))),
                border: Border {
                    color: Color::from_rgba8(255, 80, 80, 0.4),
                    width: 1.0,
                    radius: theme::RADIUS_MD.into(),
                },
                ..Default::default()
            }),
        );
    }

    if is_loading {
        let loading_badge = Container::new(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(10)
                .push(Text::new("⏳").size(16))
                .push(
                    Text::new("Awaiting OAuth authentication in your browser...")
                        .size(14)
                        .color(theme::TEXT_SECONDARY),
                ),
        )
        .padding([14, 24])
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::SURFACE_HOVER)),
            border: Border {
                color: theme::BORDER_SUBTLE,
                width: 1.0,
                radius: theme::RADIUS_PILL.into(),
            },
            ..Default::default()
        });

        inner_col = inner_col.push(Container::new(loading_badge).padding(Padding {
            top: 16.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }));
    } else {
        let login_btn = Button::new(
            Text::new("Log in with Spotify")
                .size(16)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::LoginRequested)
        .padding([16, 44])
        .style(|_theme: &Theme, status| {
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

        inner_col = inner_col.push(Container::new(login_btn).padding(Padding {
            top: 16.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }));
    }

    let card = Container::new(inner_col)
        .padding(48)
        .max_width(460.0)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: theme::RADIUS_XL.into(),
                color: theme::BORDER_SUBTLE,
                width: 1.0,
            },
            ..Default::default()
        });

    Container::new(card)
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
