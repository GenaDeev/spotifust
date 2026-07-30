use crate::app::{ContextMenuTarget, Message};
use crate::ui::icons::Icon;
use crate::ui::theme;
use iced::widget::{Button, Column, Container, Row, Scrollable, Text, TextInput};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

/// Renders a floating, high-performance Spotify-styled context menu.
#[allow(clippy::too_many_lines)]
pub fn view_context_menu(state: &crate::app::ContextMenuState) -> Element<'_, Message> {
    let mut menu_col = Column::new().spacing(4).padding(6);

    match &state.target {
        ContextMenuTarget::Track {
            track,
            from_playlist_id,
        } => {
            let track_uri = track.uri.clone();
            let track_title = track.title.clone();
            let album_name = track.album.clone();
            let artist_name = track.artist.clone();

            // Share link
            let share_url = format!("https://open.spotify.com/track/{track_uri}");
            menu_col = menu_col.push(menu_item_button(
                Icon::Share,
                "Compartir link",
                Message::CopyShareLink(track_title.clone(), share_url),
            ));

            // Add to playlist
            menu_col = menu_col.push(menu_item_button(
                Icon::Plus,
                "Agregar a playlist",
                Message::OpenAddToPlaylistModal(vec![track_uri.clone()]),
            ));

            // Remove from current playlist if inside a playlist
            if let Some(pl_id) = from_playlist_id {
                menu_col = menu_col.push(menu_item_button(
                    Icon::Trash,
                    "Eliminar de esta playlist",
                    Message::RemoveTrackFromCurrentPlaylist(pl_id.clone(), track_uri.clone()),
                ));
            }

            // Add to Queue
            menu_col = menu_col.push(menu_item_button(
                Icon::Queue,
                "Agregar a la fila",
                Message::AddToQueue(track.clone()),
            ));

            // Go to Queue tab
            menu_col = menu_col.push(menu_item_button(
                Icon::Queue,
                "Ir a la fila de reproducción",
                Message::OpenQueuePanel,
            ));

            // Go to Album
            menu_col = menu_col.push(menu_item_button(
                Icon::Album,
                format!("Ir al álbum ({album_name})"),
                Message::SelectAlbum(album_name),
            ));

            // Go to Artist
            menu_col = menu_col.push(menu_item_button(
                Icon::User,
                format!("Ir al artista ({artist_name})"),
                Message::SelectArtist(artist_name),
            ));
        }
        ContextMenuTarget::Album(album) => {
            let album_id = album.id.clone();
            let album_name = album.name.clone();
            let share_url = format!("https://open.spotify.com/album/{album_id}");

            menu_col = menu_col.push(menu_item_button(
                Icon::Share,
                "Compartir álbum",
                Message::CopyShareLink(album_name.clone(), share_url),
            ));

            menu_col = menu_col.push(menu_item_button(
                Icon::Heart,
                "Guardar en tu biblioteca",
                Message::SaveAlbumToggle(album_id.clone(), false),
            ));

            menu_col = menu_col.push(menu_item_button(
                Icon::Plus,
                "Agregar canciones del álbum a playlist",
                Message::OpenAddToPlaylistModal(vec![album_id]),
            ));
        }
        ContextMenuTarget::Playlist(playlist) => {
            let pl_id = playlist.id.clone();
            let pl_name = playlist.name.clone();
            let share_url = format!("https://open.spotify.com/playlist/{pl_id}");

            menu_col = menu_col.push(menu_item_button(
                Icon::Share,
                "Compartir playlist",
                Message::CopyShareLink(pl_name.clone(), share_url),
            ));

            menu_col = menu_col.push(menu_item_button(
                Icon::Edit,
                "Editar nombre y datos",
                Message::OpenEditPlaylistModal(pl_id.clone(), pl_name.clone(), String::new()),
            ));

            menu_col = menu_col.push(menu_item_button(
                Icon::Trash,
                "Eliminar playlist",
                Message::OpenConfirmDeletePlaylistModal(pl_id.clone(), pl_name.clone()),
            ));

            menu_col = menu_col.push(menu_item_button(
                Icon::Lock,
                "Hacer privada / pública",
                Message::TogglePlaylistPrivacy(pl_id.clone(), true),
            ));

            menu_col = menu_col.push(menu_item_button(
                Icon::Plus,
                "Copiar canciones a otra playlist",
                Message::OpenCopyPlaylistModal(pl_id, pl_name),
            ));
        }
        ContextMenuTarget::Artist {
            artist_id,
            artist_name,
        } => {
            let aid = artist_id.clone();
            let aname = artist_name.clone();

            menu_col = menu_col.push(menu_item_button(
                Icon::User,
                format!("Seguir a {aname}"),
                Message::FollowArtistToggle(aid.clone(), false),
            ));

            menu_col = menu_col.push(menu_item_button(
                Icon::Trash,
                format!("Dejar de seguir a {aname}"),
                Message::FollowArtistToggle(aid, true),
            ));
        }
    }

    let menu_card = Container::new(menu_col)
        .width(Length::Fixed(240.0))
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: theme::RADIUS_MD.into(),
                color: theme::BORDER_SUBTLE,
                width: 1.0,
            },
            ..Default::default()
        });

    let x_pos = state.position.x.clamp(10.0, 950.0);
    let y_pos = state.position.y.clamp(10.0, 600.0);

    let backdrop = Button::new(
        iced::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .style(|_theme, _status| iced::widget::button::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.25))),
        ..Default::default()
    })
    .on_press(Message::CloseContextMenu);

    let positioned_menu = Container::new(menu_card).padding(Padding {
        top: y_pos,
        right: 0.0,
        bottom: 0.0,
        left: x_pos,
    });

    iced::widget::Stack::new()
        .push(backdrop)
        .push(positioned_menu)
        .into()
}

fn menu_item_button<'a>(
    icon: Icon,
    label: impl Into<String>,
    message: Message,
) -> Element<'a, Message> {
    let label_str = label.into();
    Button::new(
        Row::new()
            .spacing(10)
            .align_y(Alignment::Center)
            .push(icon.view_colored(16.0, theme::TEXT_PRIMARY))
            .push(
                Text::new(label_str)
                    .size(13)
                    .color(theme::TEXT_PRIMARY)
                    .width(Length::Fill),
            ),
    )
    .padding([8, 12])
    .width(Length::Fill)
    .on_press(message)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: theme::TEXT_PRIMARY,
            border: Border {
                radius: theme::RADIUS_SM.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
                iced::widget::button::Style {
                    background: Some(Background::Color(theme::SURFACE_HOVER)),
                    ..base
                }
            }
            _ => base,
        }
    })
    .into()
}

/// Renders Modal Dialog Overlays centered on top of the main window.
#[allow(clippy::too_many_lines)]
pub fn view_modal<'a>(
    modal: &'a crate::app::ActiveModal,
    user_playlists: &'a [crate::api::playlist::PlaylistSummary],
) -> Element<'a, Message> {
    let content: Element<'a, Message> = match modal {
        crate::app::ActiveModal::AddToPlaylist {
            track_uris,
            search_query,
        } => {
            let uris = track_uris.clone();
            let mut list_col = Column::new().spacing(6);

            let filtered_playlists: Vec<_> = user_playlists
                .iter()
                .filter(|p| {
                    search_query.is_empty()
                        || p.name.to_lowercase().contains(&search_query.to_lowercase())
                })
                .collect();

            if filtered_playlists.is_empty() {
                list_col = list_col.push(
                    Text::new("No se encontraron playlists")
                        .size(13)
                        .color(theme::TEXT_SECONDARY),
                );
            } else {
                for pl in filtered_playlists {
                    let pid = pl.id.clone();
                    let uris_clone = uris.clone();

                    let row_item = Button::new(
                        Row::new()
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .push(Icon::Plus.view_colored(16.0, theme::ACCENT))
                            .push(
                                Text::new(&pl.name)
                                    .size(13)
                                    .color(theme::TEXT_PRIMARY)
                                    .width(Length::Fill),
                            )
                            .push(
                                Text::new(format!("{} canciones", pl.total_tracks))
                                    .size(11)
                                    .color(theme::TEXT_SECONDARY),
                            ),
                    )
                    .padding([8, 12])
                    .width(Length::Fill)
                    .on_press(Message::AddTracksToPlaylistAction(pid, uris_clone))
                    .style(|_t, s| {
                        let base = iced::widget::button::Style {
                            background: Some(Background::Color(Color::TRANSPARENT)),
                            border: Border {
                                radius: theme::RADIUS_SM.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        };
                        if matches!(s, iced::widget::button::Status::Hovered) {
                            iced::widget::button::Style {
                                background: Some(Background::Color(theme::SURFACE_HOVER)),
                                ..base
                            }
                        } else {
                            base
                        }
                    });

                    list_col = list_col.push(row_item);
                }
            }

            Column::new()
                .spacing(16)
                .push(
                    Row::new()
                        .align_y(Alignment::Center)
                        .push(
                            Text::new("Agregar a playlist")
                                .size(20)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                                .color(theme::TEXT_PRIMARY),
                        )
                        .push(iced::widget::Space::new().width(Length::Fill))
                        .push(
                            Button::new(Icon::X.view_colored(16.0, theme::TEXT_SECONDARY))
                                .on_press(Message::CloseModal)
                                .style(|_t, _s| iced::widget::button::Style::default()),
                        ),
                )
                .push(
                    TextInput::new("Buscar playlist...", search_query)
                        .on_input(Message::ModalSearchInputChanged)
                        .padding(10)
                        .style(|_t, _s| iced::widget::text_input::Style {
                            background: Background::Color(theme::SURFACE_HOVER),
                            border: Border {
                                radius: theme::RADIUS_SM.into(),
                                color: theme::BORDER_SUBTLE,
                                width: 1.0,
                            },
                            value: theme::TEXT_PRIMARY,
                            placeholder: theme::TEXT_TERTIARY,
                            selection: theme::ACCENT,
                            icon: theme::TEXT_SECONDARY,
                        }),
                )
                .push(
                    Container::new(Scrollable::new(list_col))
                        .max_height(260.0)
                        .width(Length::Fill),
                )
                .into()
        }
        crate::app::ActiveModal::EditPlaylist {
            playlist_id,
            name_input,
            description_input,
        } => {
            let pid = playlist_id.clone();
            let name = name_input.clone();
            let desc = description_input.clone();

            Column::new()
                .spacing(16)
                .push(
                    Row::new()
                        .align_y(Alignment::Center)
                        .push(
                            Text::new("Editar Playlist")
                                .size(20)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                                .color(theme::TEXT_PRIMARY),
                        )
                        .push(iced::widget::Space::new().width(Length::Fill))
                        .push(
                            Button::new(Icon::X.view_colored(16.0, theme::TEXT_SECONDARY))
                                .on_press(Message::CloseModal)
                                .style(|_t, _s| iced::widget::button::Style::default()),
                        ),
                )
                .push(Text::new("Nombre").size(12).color(theme::TEXT_SECONDARY))
                .push(
                    TextInput::new("Nombre de la playlist", name_input)
                        .on_input(Message::ModalNameInputChanged)
                        .padding(10)
                        .style(|_t, _s| iced::widget::text_input::Style {
                            background: Background::Color(theme::SURFACE_HOVER),
                            border: Border {
                                radius: theme::RADIUS_SM.into(),
                                color: theme::BORDER_SUBTLE,
                                width: 1.0,
                            },
                            value: theme::TEXT_PRIMARY,
                            placeholder: theme::TEXT_TERTIARY,
                            selection: theme::ACCENT,
                            icon: theme::TEXT_SECONDARY,
                        }),
                )
                .push(
                    Text::new("Descripción")
                        .size(12)
                        .color(theme::TEXT_SECONDARY),
                )
                .push(
                    TextInput::new("Descripción opcional", description_input)
                        .on_input(Message::ModalDescInputChanged)
                        .padding(10)
                        .style(|_t, _s| iced::widget::text_input::Style {
                            background: Background::Color(theme::SURFACE_HOVER),
                            border: Border {
                                radius: theme::RADIUS_SM.into(),
                                color: theme::BORDER_SUBTLE,
                                width: 1.0,
                            },
                            value: theme::TEXT_PRIMARY,
                            placeholder: theme::TEXT_TERTIARY,
                            selection: theme::ACCENT,
                            icon: theme::TEXT_SECONDARY,
                        }),
                )
                .push(
                    Row::new()
                        .spacing(12)
                        .push(iced::widget::Space::new().width(Length::Fill))
                        .push(
                            Button::new(
                                Text::new("Cancelar").size(13).color(theme::TEXT_SECONDARY),
                            )
                            .on_press(Message::CloseModal)
                            .style(|_t, _s| iced::widget::button::Style::default()),
                        )
                        .push(
                            Button::new(Text::new("Guardar").size(13).color(theme::TEXT_PRIMARY))
                                .padding([8, 16])
                                .on_press(Message::SavePlaylistDetailsAction(pid, name, desc))
                                .style(|_t, _s| iced::widget::button::Style {
                                    background: Some(Background::Color(theme::ACCENT)),
                                    border: Border {
                                        radius: theme::RADIUS_PILL.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                        ),
                )
                .into()
        }
        crate::app::ActiveModal::ConfirmDeletePlaylist {
            playlist_id,
            playlist_name,
        } => {
            let pid = playlist_id.clone();
            Column::new()
                .spacing(16)
                .push(
                    Text::new("¿Eliminar de Tu biblioteca?")
                        .size(20)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(theme::TEXT_PRIMARY),
                )
                .push(
                    Text::new(format!(
                        "Esta acción eliminará '{playlist_name}' de tu cuenta de Spotify."
                    ))
                    .size(13)
                    .color(theme::TEXT_SECONDARY),
                )
                .push(
                    Row::new()
                        .spacing(12)
                        .push(iced::widget::Space::new().width(Length::Fill))
                        .push(
                            Button::new(
                                Text::new("Cancelar").size(13).color(theme::TEXT_SECONDARY),
                            )
                            .on_press(Message::CloseModal)
                            .style(|_t, _s| iced::widget::button::Style::default()),
                        )
                        .push(
                            Button::new(Text::new("Eliminar").size(13).color(theme::TEXT_PRIMARY))
                                .padding([8, 16])
                                .on_press(Message::DeletePlaylistConfirmed(pid))
                                .style(|_t, _s| iced::widget::button::Style {
                                    background: Some(Background::Color(Color::from_rgb(
                                        0.9, 0.2, 0.2,
                                    ))),
                                    border: Border {
                                        radius: theme::RADIUS_PILL.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                        ),
                )
                .into()
        }
        crate::app::ActiveModal::CopyPlaylistToAnother {
            source_playlist_id,
            source_playlist_name,
            search_query,
        } => {
            let src_id = source_playlist_id.clone();
            let mut list_col = Column::new().spacing(6);

            let filtered_playlists: Vec<_> = user_playlists
                .iter()
                .filter(|p| {
                    p.id != src_id
                        && (search_query.is_empty()
                            || p.name.to_lowercase().contains(&search_query.to_lowercase()))
                })
                .collect();

            if filtered_playlists.is_empty() {
                list_col = list_col.push(
                    Text::new("No hay otras playlists disponibles")
                        .size(13)
                        .color(theme::TEXT_SECONDARY),
                );
            } else {
                for pl in filtered_playlists {
                    let target_id = pl.id.clone();
                    let src_id_clone = src_id.clone();

                    let row_item = Button::new(
                        Row::new()
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .push(Icon::MusicNote.view_colored(16.0, theme::TEXT_SECONDARY))
                            .push(
                                Text::new(&pl.name)
                                    .size(13)
                                    .color(theme::TEXT_PRIMARY)
                                    .width(Length::Fill),
                            ),
                    )
                    .padding([8, 12])
                    .width(Length::Fill)
                    .on_press(Message::CopyPlaylistTracksAction(src_id_clone, target_id))
                    .style(|_t, s| {
                        let base = iced::widget::button::Style {
                            background: Some(Background::Color(Color::TRANSPARENT)),
                            border: Border {
                                radius: theme::RADIUS_SM.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        };
                        if matches!(s, iced::widget::button::Status::Hovered) {
                            iced::widget::button::Style {
                                background: Some(Background::Color(theme::SURFACE_HOVER)),
                                ..base
                            }
                        } else {
                            base
                        }
                    });

                    list_col = list_col.push(row_item);
                }
            }

            Column::new()
                .spacing(16)
                .push(
                    Row::new()
                        .align_y(Alignment::Center)
                        .push(
                            Text::new(format!("Copiar canciones de '{source_playlist_name}'"))
                                .size(20)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                                .color(theme::TEXT_PRIMARY),
                        )
                        .push(iced::widget::Space::new().width(Length::Fill))
                        .push(
                            Button::new(Icon::X.view_colored(16.0, theme::TEXT_SECONDARY))
                                .on_press(Message::CloseModal)
                                .style(|_t, _s| iced::widget::button::Style::default()),
                        ),
                )
                .push(
                    TextInput::new("Buscar playlist destino...", search_query)
                        .on_input(Message::ModalSearchInputChanged)
                        .padding(10)
                        .style(|_t, _s| iced::widget::text_input::Style {
                            background: Background::Color(theme::SURFACE_HOVER),
                            border: Border {
                                radius: theme::RADIUS_SM.into(),
                                color: theme::BORDER_SUBTLE,
                                width: 1.0,
                            },
                            value: theme::TEXT_PRIMARY,
                            placeholder: theme::TEXT_TERTIARY,
                            selection: theme::ACCENT,
                            icon: theme::TEXT_SECONDARY,
                        }),
                )
                .push(
                    Container::new(Scrollable::new(list_col))
                        .max_height(260.0)
                        .width(Length::Fill),
                )
                .into()
        }
    };

    let modal_card = Container::new(content)
        .padding(24)
        .width(Length::Fixed(440.0))
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: theme::RADIUS_LG.into(),
                color: theme::BORDER_SUBTLE,
                width: 1.0,
            },
            ..Default::default()
        });

    Container::new(modal_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.75,
            })),
            ..Default::default()
        })
        .into()
}

/// Renders Toast Notifications overlay in the bottom right corner.
pub fn view_toasts(toast: Option<&String>) -> Element<'_, Message> {
    if let Some(msg) = toast {
        let toast_card = Container::new(
            Row::new()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(Icon::MusicNote.view_colored(16.0, theme::ACCENT))
                .push(
                    Text::new(msg)
                        .size(13)
                        .color(theme::TEXT_PRIMARY)
                        .width(Length::Shrink),
                )
                .push(
                    Button::new(Icon::X.view_colored(14.0, theme::TEXT_SECONDARY))
                        .padding([2, 6])
                        .on_press(Message::DismissToast)
                        .style(|_t, _s| iced::widget::button::Style {
                            background: Some(Background::Color(Color::TRANSPARENT)),
                            ..Default::default()
                        }),
                ),
        )
        .padding([10, 16])
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::SURFACE_CARD)),
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                color: theme::ACCENT,
                width: 1.0,
            },
            shadow: iced::Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        });

        Container::new(
            Column::new()
                .push(iced::widget::Space::new().height(Length::Fill))
                .push(
                    Row::new()
                        .push(iced::widget::Space::new().width(Length::Fill))
                        .push(toast_card)
                        .padding(24),
                ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        Container::new(iced::widget::Space::new()).into()
    }
}
