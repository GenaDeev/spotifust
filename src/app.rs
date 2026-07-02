use iced::{
    widget::canvas::Cache,
    Element, Task, Point, Rectangle, Size,
};
use crate::error::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CardState {
    pub bounds: Rectangle,
    pub is_hovered: bool,
    pub is_dragging: bool,
    pub title: String,
}

pub struct App {
    pub cards: Vec<CardState>,
    pub canvas_cache: Cache,
    pub dragging_card_idx: Option<usize>,
    pub drag_offset: Point,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Message {
    ErrorEncountered(AppError),
    CursorMoved(Point),
    CursorPressed(Point),
    CursorReleased,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                cards: vec![
                    CardState {
                        bounds: Rectangle::new(Point::new(50.0, 50.0), Size::new(200.0, 150.0)),
                        is_hovered: false,
                        is_dragging: false,
                        title: "Playlist 1".to_string(),
                    },
                    CardState {
                        bounds: Rectangle::new(Point::new(300.0, 50.0), Size::new(200.0, 150.0)),
                        is_hovered: false,
                        is_dragging: false,
                        title: "Playlist 2".to_string(),
                    },
                ],
                canvas_cache: Cache::default(),
                dragging_card_idx: None,
                drag_offset: Point::ORIGIN,
            },
            Task::none(),
        )
    }

    #[allow(clippy::unused_self)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ErrorEncountered(e) => {
                eprintln!("Error encountered: {e:?}");
            }
            Message::CursorMoved(point) => {
                if let Some(idx) = self.dragging_card_idx {
                    self.cards[idx].bounds.x = point.x - self.drag_offset.x;
                    self.cards[idx].bounds.y = point.y - self.drag_offset.y;
                    self.canvas_cache.clear(); // Geometry changed
                } else {
                    for card in &mut self.cards {
                        let contains = card.bounds.contains(point);
                        if card.is_hovered != contains {
                            card.is_hovered = contains;
                            // Hover change doesn't alter layout, cache not cleared.
                        }
                    }
                }
            }
            Message::CursorPressed(point) => {
                for (idx, card) in self.cards.iter_mut().enumerate().rev() {
                    if card.bounds.contains(point) {
                        self.dragging_card_idx = Some(idx);
                        card.is_dragging = true;
                        self.drag_offset = Point::new(
                            point.x - card.bounds.x,
                            point.y - card.bounds.y,
                        );
                        break;
                    }
                }
            }
            Message::CursorReleased => {
                if let Some(idx) = self.dragging_card_idx.take() {
                    self.cards[idx].is_dragging = false;
                }
            }
        }
        Task::none()
    }

    #[allow(clippy::unused_self)]
    pub fn view(&self) -> Element<'_, Message> {
        let canvas = iced::widget::canvas(crate::ui::canvas_view::CanvasView {
            cards: &self.cards,
            cache: &self.canvas_cache,
        })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill);

        iced::widget::container(canvas)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }
}
