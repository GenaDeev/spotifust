use iced::{
    widget::{Column, Container, Row, Text, Space, canvas},
    Length, Element, Alignment, Background, Color, Theme, Border, Shadow, Vector,
};
use crate::app::{CardState, Message};
use crate::ui::canvas_view::CanvasView;

pub fn view<'a>(
    cards: &'a [CardState],
    canvas_cache: &'a canvas::Cache,
) -> Element<'a, Message> {
    // 1. Sidebar (Darker, rounded floating style)
    let sidebar = Container::new(
        Column::new()
            .push(Text::new("Spotifust").size(26).color(Color::from_rgb8(29, 185, 84)))
            .push(Space::new().height(Length::Fixed(40.0)))
            .push(Text::new("Home").size(16).color(Color::WHITE))
            .push(Text::new("Search").size(16).color(Color::from_rgb8(179, 179, 179)))
            .push(Text::new("Library").size(16).color(Color::from_rgb8(179, 179, 179)))
            .spacing(24)
            .padding(25),
    )
    .width(Length::Fixed(260.0))
    .height(Length::Fill)
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(5, 5, 5))), // Ultra dark
        border: Border {
            color: Color::from_rgba8(255, 255, 255, 0.05),
            width: 1.0,
            radius: iced::border::Radius { top_left: 0.0, top_right: 15.0, bottom_right: 15.0, bottom_left: 0.0 },
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.8),
            offset: Vector::new(10.0, 0.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    });

    // 2. Main Content (Canvas Cards)
    let canvas = iced::widget::canvas(CanvasView {
        cards,
        cache: canvas_cache,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    let main_content = Container::new(canvas)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgb8(10, 10, 10))), // Deep dark grey
            ..Default::default()
        });

    // 3. Playback Bar (Bottom) - Glassy/Floating look
    let playback_bar = Container::new(
        Row::new()
            .align_y(Alignment::Center)
            .push(Text::new("Now Playing - Artist").size(14).color(Color::WHITE))
            .push(Space::new().width(Length::Fill))
            .push(
                Row::new()
                    .spacing(20)
                    .push(Text::new("⏮").size(22).color(Color::from_rgb8(179, 179, 179)))
                    .push(Text::new("▶").size(28).color(Color::WHITE))
                    .push(Text::new("⏭").size(22).color(Color::from_rgb8(179, 179, 179)))
            )
            .push(Space::new().width(Length::Fill))
            .push(Text::new("0:00 / 3:45").size(12).color(Color::from_rgb8(179, 179, 179)))
            .padding([15, 30]),
    )
    .width(Length::Fill)
    .height(Length::Fixed(90.0))
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(15, 15, 15))), // Slightly lighter than base
        border: Border {
            color: Color::from_rgba8(255, 255, 255, 0.08),
            width: 1.0,
            radius: iced::border::Radius { top_left: 20.0, top_right: 20.0, bottom_right: 0.0, bottom_left: 0.0 },
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.9),
            offset: Vector::new(0.0, -10.0),
            blur_radius: 30.0,
        },
        text_color: Some(Color::WHITE),
        ..Default::default()
    });

    // 4. Assemble the layout
    let top_section = Row::new()
        .push(sidebar)
        .push(main_content)
        .height(Length::Fill);

    let layout = Column::new()
        .push(top_section)
        .push(playback_bar);

    Container::new(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgb8(10, 10, 10))),
            ..Default::default()
        })
        .into()
}
