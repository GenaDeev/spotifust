use crate::app::Message;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector,
    widget::{Button, Column, Container, Image, Text},
};

// We use the same logo byte array used for the window icon
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
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(120.0));

    let title = Text::new("Spotifust")
        .size(56)
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        })
        .color(Color::WHITE);

    let subtitle = Text::new("Connect your Spotify account to continue")
        .size(18)
        .color(Color::from_rgb8(170, 170, 170));

    let mut inner_col = Column::new()
        .spacing(15)
        .align_x(Alignment::Center)
        .push(logo)
        .push(title)
        .push(subtitle);

    if let Some(err) = error {
        inner_col = inner_col.push(
            Container::new(Text::new(err).color(iced::color!(0x00FF_5555)).size(14))
                .padding([10, 20])
                .style(|_theme: &Theme| iced::widget::container::Style {
                    background: Some(Background::Color(Color::from_rgba8(255, 85, 85, 0.1))),
                    border: Border {
                        color: iced::color!(0x00FF_5555),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
        );
    }

    if is_loading {
        inner_col = inner_col.push(
            Text::new("Awaiting browser login...")
                .size(16)
                .color(Color::from_rgb8(150, 150, 150)),
        );
    } else {
        let login_btn = Button::new(
            Text::new("Login with Spotify")
                .size(18)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::LoginRequested)
        .padding([16, 48])
        .style(|_theme: &Theme, status| {
            let base = iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgb8(29, 185, 84))),
                text_color: Color::WHITE,
                border: Border {
                    radius: 30.0.into(),
                    ..Default::default()
                },
                shadow: Shadow {
                    color: Color::from_rgba8(29, 185, 84, 0.3),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 16.0,
                },
                ..Default::default()
            };

            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Background::Color(Color::from_rgb8(30, 215, 96))),
                    shadow: Shadow {
                        color: Color::from_rgba8(29, 185, 84, 0.5),
                        offset: Vector::new(0.0, 12.0),
                        blur_radius: 20.0,
                    },
                    ..base
                },
                iced::widget::button::Status::Pressed => iced::widget::button::Style {
                    background: Some(Background::Color(Color::from_rgb8(20, 131, 59))),
                    shadow: Shadow {
                        color: Color::from_rgba8(29, 185, 84, 0.1),
                        offset: Vector::new(0.0, 4.0),
                        blur_radius: 8.0,
                    },
                    ..base
                },
                _ => base,
            }
        });

        inner_col = inner_col.push(Container::new(login_btn).padding(iced::Padding {
            top: 20.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }));
    }

    let card = Container::new(inner_col)
        .padding(50)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgb8(24, 24, 24))),
            border: Border {
                radius: 24.0.into(),
                color: Color::from_rgb8(40, 40, 40),
                width: 1.0,
            },
            shadow: Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.5),
                offset: Vector::new(0.0, 20.0),
                blur_radius: 40.0,
            },
            ..Default::default()
        });

    Container::new(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgb8(9, 9, 9))),
            ..Default::default()
        })
        .into()
}
