use iced::{Element, Task};
use crate::error::AppError;

pub struct App {
    // Basic model state placeholder
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Message {
    ErrorEncountered(AppError),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {},
            Task::none(),
        )
    }

    #[allow(clippy::unused_self)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ErrorEncountered(e) => {
                eprintln!("Error encountered: {e:?}");
                Task::none()
            }
        }
    }

    #[allow(clippy::unused_self)]
    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::container(iced::widget::text("Spotifust - MVU Loop Initialized"))
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill)
            .into()
    }
}
