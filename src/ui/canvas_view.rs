use iced::{
    Rectangle, Renderer, Theme, Point, Size, Pixels,
    widget::canvas::{self, Event, Geometry, Path, Program, Stroke, Text},
    mouse,
};
use crate::app::{Message, Card};
use crate::ui::theme;

pub struct CardCanvas<'a> {
    pub cards: &'a [Card],
    pub layout_cache: &'a canvas::Cache,
}

impl<'a> CardCanvas<'a> {
    pub fn new(cards: &'a [Card], layout_cache: &'a canvas::Cache) -> Self {
        Self { cards, layout_cache }
    }
}

impl Program<Message> for CardCanvas<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        // 1. Static Geometry (Cached) - Card backgrounds, titles, layout structures
        let layout_geom = self.layout_cache.draw(renderer, bounds.size(), |frame| {
            for card in self.cards {
                // Background
                let rect_path = Path::rounded_rectangle(
                    Point::new(card.x, card.y),
                    Size::new(card.width, card.height),
                    8.0.into(),
                );
                frame.fill(&rect_path, theme::SURFACE_1);

                // Subtle Border
                frame.stroke(
                    &rect_path,
                    Stroke {
                        style: theme::BG_BASE.into(),
                        width: 1.0,
                        ..Default::default()
                    },
                );

                // Header Divider
                let divider_path = Path::line(
                    Point::new(card.x, card.y + 40.0),
                    Point::new(card.x + card.width, card.y + 40.0),
                );
                frame.stroke(
                    &divider_path,
                    Stroke {
                        style: theme::BG_BASE.into(),
                        width: 1.0,
                        ..Default::default()
                    },
                );

                // Title Text
                frame.fill_text(Text {
                    content: card.title.clone(),
                    position: Point::new(card.x + 12.0, card.y + 12.0),
                    color: theme::TEXT_PRIMARY,
                    size: Pixels(14.0),
                    font: iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    },
                    ..Default::default()
                });

                // Subtitle Text
                frame.fill_text(Text {
                    content: card.subtitle.clone(),
                    position: Point::new(card.x + 12.0, card.y + 48.0),
                    color: theme::TEXT_SECONDARY,
                    size: Pixels(12.0),
                    ..Default::default()
                });

                // Draw resize grip dots/lines in the bottom-right corner
                let grip_path = Path::new(|builder| {
                    builder.move_to(Point::new(card.x + card.width - 12.0, card.y + card.height - 4.0));
                    builder.line_to(Point::new(card.x + card.width - 4.0, card.y + card.height - 12.0));

                    builder.move_to(Point::new(card.x + card.width - 8.0, card.y + card.height - 4.0));
                    builder.line_to(Point::new(card.x + card.width - 4.0, card.y + card.height - 8.0));
                });
                frame.stroke(
                    &grip_path,
                    Stroke {
                        style: theme::TEXT_SECONDARY.into(),
                        width: 1.5,
                        ..Default::default()
                    },
                );
            }
        });

        // 2. Dynamic Geometry (Uncached) - Hover and dragging visual indicators
        let mut interaction_frame = canvas::Frame::new(renderer, bounds.size());
        for card in self.cards {
            if card.hovered || card.dragging {
                let rect_path = Path::rounded_rectangle(
                    Point::new(card.x, card.y),
                    Size::new(card.width, card.height),
                    8.0.into(),
                );

                let highlight_color = if card.dragging {
                    theme::ACCENT
                } else {
                    theme::TEXT_SECONDARY
                };

                interaction_frame.stroke(
                    &rect_path,
                    Stroke {
                        style: highlight_color.into(),
                        width: 1.5,
                        ..Default::default()
                    },
                );
            }
        }
        let interaction_geom = interaction_frame.into_geometry();

        vec![layout_geom, interaction_geom]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let cursor_position = cursor.position_in(bounds)?;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                // Hit-testing cards in reverse order (top of stack first)
                for card in self.cards.iter().rev() {
                    let card_rect = Rectangle {
                        x: card.x,
                        y: card.y,
                        width: card.width,
                        height: card.height,
                    };

                    if card_rect.contains(cursor_position) {
                        // Check if cursor is in the bottom-right resize zone (16x16 pixels)
                        let resize_zone = Rectangle {
                            x: card.x + card.width - 16.0,
                            y: card.y + card.height - 16.0,
                            width: 16.0,
                            height: 16.0,
                        };

                        let is_resize = resize_zone.contains(cursor_position);
                        let offset_x = cursor_position.x - card.x;
                        let offset_y = cursor_position.y - card.y;

                        return Some(canvas::Action::publish(Message::CardPressed {
                            id: card.id.clone(),
                            is_resize,
                            offset_x,
                            offset_y,
                        }));
                    }
                }
                None
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // Check if any card is currently dragging or resizing
                let is_interacting = self.cards.iter().any(|c| c.dragging || c.resizing);

                if is_interacting {
                    Some(canvas::Action::publish(Message::CardMoved {
                        x: cursor_position.x,
                        y: cursor_position.y,
                    }))
                } else {
                    // Update hover state if cursor is over a card
                    let mut hovered_id = None;
                    for card in self.cards.iter().rev() {
                        let card_rect = Rectangle {
                            x: card.x,
                            y: card.y,
                            width: card.width,
                            height: card.height,
                        };

                        if card_rect.contains(cursor_position) {
                            hovered_id = Some(card.id.clone());
                            break;
                        }
                    }

                    Some(canvas::Action::publish(Message::CardHovered(hovered_id)))
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let is_interacting = self.cards.iter().any(|c| c.dragging || c.resizing);
                if is_interacting {
                    Some(canvas::Action::publish(Message::CardReleased))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(cursor_position) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };

        if self.cards.iter().any(|c| c.dragging) {
            return mouse::Interaction::Grabbing;
        }

        if self.cards.iter().any(|c| c.resizing) {
            return mouse::Interaction::ResizingDiagonallyDown;
        }

        for card in self.cards.iter().rev() {
            let resize_zone = Rectangle {
                x: card.x + card.width - 16.0,
                y: card.y + card.height - 16.0,
                width: 16.0,
                height: 16.0,
            };

            if resize_zone.contains(cursor_position) {
                return mouse::Interaction::ResizingDiagonallyDown;
            }

            let card_rect = Rectangle {
                x: card.x,
                y: card.y,
                width: card.width,
                height: card.height,
            };

            if card_rect.contains(cursor_position) {
                return mouse::Interaction::Grab;
            }
        }

        mouse::Interaction::default()
    }
}
