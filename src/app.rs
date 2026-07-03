use crate::audio::engine::{AudioCommand, AudioEngine};
use crate::error::AppError;
use iced::{Element, Point, Rectangle, Size, Task, widget::canvas::Cache};
use tokio::sync::mpsc as tokio_mpsc;
use crate::ui::login;

#[derive(Debug, Clone)]
pub struct CardState {
    pub bounds: Rectangle,
    pub is_hovered: bool,
    pub is_dragging: bool,
    pub title: String,
}

pub enum AppState {
    Login {
        is_loading: bool,
        error: Option<String>,
    },
    Main {
        cards: Vec<CardState>,
        canvas_cache: Cache,
        dragging_card_idx: Option<usize>,
        drag_offset: Point,
    }
}

pub struct App {
    pub state: AppState,
    #[allow(dead_code)]
    pub audio_tx: tokio_mpsc::Sender<AudioCommand>,
}

#[derive(Debug, Clone)]
pub enum Message {
    #[allow(dead_code)]
    ErrorEncountered(AppError),
    // Login Messages
    LoginRequested,
    CheckLogin,
    CheckLoginFailed,
    LoginSuccess, // Will be fired from async task
    LoginFailed(String),
    // Main UI Messages
    CursorMoved(Point),
    CursorPressed(Point),
    CursorReleased,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let audio_tx = AudioEngine::spawn();
        let _ = audio_tx.try_send(AudioCommand::Play); // Auto-play the test sine wave

        (
            Self {
                state: AppState::Login {
                    is_loading: true, // Initial check
                    error: None,
                },
                audio_tx,
            },
            Task::perform(
                async { crate::api::auth::check_existing_login().await },
                |res| match res {
                    Ok(_spotify) => Message::LoginSuccess,
                    Err(_) => Message::LoginFailed("No token".to_string()),
                }
            )
        )
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        match &self.state {
            AppState::Login { is_loading: true, .. } => {
                iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::CheckLogin)
            }
            _ => iced::Subscription::none(),
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ErrorEncountered(e) => {
                eprintln!("Error encountered: {e:?}");
                Task::none()
            }
            Message::LoginRequested => {
                if let AppState::Login { is_loading, .. } = &mut self.state {
                    *is_loading = true;
                    
                    return Task::perform(
                        async { crate::api::auth::do_login_flow().await },
                        |res| match res {
                            Ok(()) => Message::LoginSuccess,
                            Err(e) => Message::LoginFailed(e.to_string()),
                        }
                    );
                }
                Task::none()
            }
            Message::CheckLogin => {
                Task::perform(
                    async { crate::api::auth::check_existing_login().await },
                    |res| match res {
                        Ok(_) => Message::LoginSuccess,
                        Err(_) => Message::CheckLoginFailed,
                    }
                )
            }
            Message::CheckLoginFailed => {
                // Do nothing, keep polling
                Task::none()
            }
            Message::LoginSuccess => {
                self.state = AppState::Main {
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
                };
                Task::none()
            }
            Message::LoginFailed(err) => {
                if let AppState::Login { is_loading, error, .. } = &mut self.state {
                    *is_loading = false;
                    if err != "No token" {
                        *error = Some(err);
                    }
                }
                Task::none()
            }
            Message::CursorMoved(point) => {
                if let AppState::Main { cards, canvas_cache, dragging_card_idx, drag_offset } = &mut self.state {
                    if let Some(idx) = *dragging_card_idx {
                        cards[idx].bounds.x = point.x - drag_offset.x;
                        cards[idx].bounds.y = point.y - drag_offset.y;
                        canvas_cache.clear(); // Geometry changed
                    } else {
                        for card in cards {
                            let contains = card.bounds.contains(point);
                            if card.is_hovered != contains {
                                card.is_hovered = contains;
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::CursorPressed(point) => {
                if let AppState::Main { cards, dragging_card_idx, drag_offset, .. } = &mut self.state {
                    for (idx, card) in cards.iter_mut().enumerate().rev() {
                        if card.bounds.contains(point) {
                            *dragging_card_idx = Some(idx);
                            card.is_dragging = true;
                            *drag_offset =
                                Point::new(point.x - card.bounds.x, point.y - card.bounds.y);
                            break;
                        }
                    }
                }
                Task::none()
            }
            Message::CursorReleased => {
                if let AppState::Main { cards, dragging_card_idx, .. } = &mut self.state {
                    if let Some(idx) = dragging_card_idx.take() {
                        cards[idx].is_dragging = false;
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            AppState::Login { is_loading, error } => {
                login::view("", "", *is_loading, error.as_deref())
            }
            AppState::Main { cards, canvas_cache, .. } => {
                crate::ui::main_layout::view(cards, canvas_cache)
            }
        }
    }
}
