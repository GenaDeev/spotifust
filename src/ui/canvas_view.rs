use crate::app::{CardState, Message};
use iced::{
    Color, Point, Rectangle, Theme, mouse,
    widget::canvas::{Action, Cache, Event, Frame, Geometry, Path, Program, Stroke},
};

#[allow(dead_code)]
pub struct CanvasView<'a> {
    pub cards: &'a [CardState],
    pub cache: &'a Cache,
}

impl Program<Message> for CanvasView<'_> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let cursor_position = cursor.position_in(bounds)?;

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                Some(Action::publish(Message::CursorMoved(cursor_position)))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                Some(Action::publish(Message::CursorPressed(cursor_position)))
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(Action::publish(Message::CursorReleased))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        // Static cached layout geometry
        let background = self.cache.draw(renderer, bounds.size(), |frame| {
            // Dark theme background
            frame.fill_rectangle(
                Point::ORIGIN,
                bounds.size(),
                Color::from_rgb8(0x12, 0x12, 0x12),
            );

            for card in self.cards {
                let rect =
                    Path::rectangle(Point::new(card.bounds.x, card.bounds.y), card.bounds.size());

                // Base card color (Spotify's #181818 surface color)
                frame.fill(&rect, Color::from_rgb8(0x18, 0x18, 0x18));

                // Draw title
                frame.fill_text(canvas::Text {
                    content: card.title.clone(),
                    position: Point::new(card.bounds.x + 15.0, card.bounds.y + 15.0),
                    color: Color::WHITE,
                    size: iced::Pixels(16.0),
                    ..Default::default()
                });
            }
        });

        // Dynamic hover/drag borders (NOT cached to avoid invalidating heavy layout renders)
        let mut dynamic = Frame::new(renderer, bounds.size());

        for card in self.cards {
            if card.is_hovered || card.is_dragging {
                let rect =
                    Path::rectangle(Point::new(card.bounds.x, card.bounds.y), card.bounds.size());

                // Accent color: Rust's #f48264 orange
                let border_color = if card.is_dragging {
                    Color::from_rgb8(0xF4, 0x82, 0x64)
                } else {
                    // Dimmer on hover
                    Color::from_rgba8(0xF4, 0x82, 0x64, 0.7)
                };

                dynamic.stroke(
                    &rect,
                    Stroke::default().with_color(border_color).with_width(2.0),
                );
            }
        }

        vec![background, dynamic.into_geometry()]
    }
}
