use iced::{
    mouse,
    widget::canvas::{self, Action, Cache, Geometry, Program},
    Rectangle, Theme,
};
use crate::app::Message;

#[allow(dead_code)]
pub struct CardLayout {
    pub bounds: Rectangle,
    pub is_hovered: bool,
    pub is_dragging: bool,
    pub title: String,
}

#[allow(dead_code)]
pub struct CanvasView<'a> {
    pub cards: &'a [CardLayout],
    pub cache: &'a Cache,
}

impl Program<Message> for CanvasView<'_> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |_frame| {
            // Draw cards
        });
        
        vec![geometry]
    }
}
