use iced::{
    widget::{Button, Column, Container, Text},
    Length, Element, Alignment, Background, Color, Theme, Border, Shadow, Vector,
};
use crate::app::Message;

pub fn view<'a>(
    _username: &'a str,
    _password: &'a str,
    is_loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    let title = Text::new("Spotifust")
        .size(48)
        .color(Color::from_rgb8(29, 185, 84));

    let subtitle = Text::new("Connect your Spotify account to continue")
        .size(16)
        .color(Color::from_rgb8(179, 179, 179));

    let mut col = Column::new()
        .spacing(30)
        .align_x(Alignment::Center)
        .push(title)
        .push(subtitle);

    if let Some(err) = error {
        col = col.push(Text::new(err).color(iced::color!(0x00FF_5555)));
    }

    if is_loading {
        col = col.push(Text::new("Awaiting browser login...").color(Color::WHITE));
    } else {
        let login_btn = Button::new(
            Text::new("Login with Spotify")
                .size(18)
                .align_x(iced::alignment::Horizontal::Center)
        )
        .on_press(Message::LoginRequested)
        .padding([15, 40])
        .style(|_theme: &Theme, status| {
            let base = iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgb8(29, 185, 84))),
                text_color: Color::WHITE,
                border: Border {
                    radius: 25.0.into(),
                    ..Default::default()
                },
                shadow: Shadow {
                    color: Color::from_rgba8(29, 185, 84, 0.4),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 10.0,
                },
                ..Default::default()
            };
            
            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Background::Color(Color::from_rgb8(30, 215, 96))),
                    shadow: Shadow {
                        color: Color::from_rgba8(29, 185, 84, 0.6),
                        offset: Vector::new(0.0, 6.0),
                        blur_radius: 15.0,
                    },
                    ..base
                },
                iced::widget::button::Status::Pressed => iced::widget::button::Style {
                    background: Some(Background::Color(Color::from_rgb8(20, 131, 59))),
                    shadow: Shadow {
                        color: Color::from_rgba8(29, 185, 84, 0.2),
                        offset: Vector::new(0.0, 2.0),
                        blur_radius: 5.0,
                    },
                    ..base
                },
                _ => base,
            }
        });

        col = col.push(login_btn);
    }

    Container::new(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgb8(10, 10, 10))),
            ..Default::default()
        })
        .into()
}
