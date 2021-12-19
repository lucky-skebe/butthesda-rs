use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Game;

use super::UIMessage;

#[derive(Debug, Clone)]
pub enum Message {
    GameSelected(Game),
    PickModPath,
    ModPathInput(String),
    ModPathPicked(PathBuf),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    game: Option<Game>,
    mod_path: PathBuf,
}

pub struct State {
    game: Option<Game>,
    pub mod_path: PathBuf,
    mod_path_state: iced::text_input::State,
    pick_mod_path_state: iced::button::State,
}

impl State {
    pub fn new() -> Self {
        Self {
            game: None,
            mod_path: PathBuf::new(),
            mod_path_state: Default::default(),
            pick_mod_path_state: Default::default(),
        }
    }

    pub fn save(&self) -> Config {
        Config {
            game: self.game.clone(),
            mod_path: self.mod_path.clone(),
        }
    }

    pub(crate) fn load(&mut self, config: &Config) {
        self.mod_path = config.mod_path.clone();
        self.game = config.game.clone();
    }

    pub(crate) fn update(&mut self, message: Message) -> iced::Command<UIMessage> {
        match message {
            Message::GameSelected(game) => {
                self.game = Some(game);
                iced::Command::perform(
                    async move { UIMessage::OutMessage(crate::Message::ConnectToProcess(game)) },
                    |m| m,
                )
            }
            Message::ModPathPicked(p) => {
                self.mod_path = p;
                let base_path = self.mod_path.clone();
                iced::Command::batch([
                    iced::Command::perform(async { UIMessage::LoadFunscripts }, |m| m),
                    iced::Command::perform(
                        async {
                            UIMessage::OutMessage(crate::Message::LinkFileOut(
                                crate::link_file::OutMessage::StartScan(base_path),
                            ))
                        },
                        |m| m,
                    ),
                ])
            }
            Message::PickModPath => {
                iced::Command::perform(rfd::AsyncFileDialog::new().pick_folder(), |p| match p {
                    Some(path) => Message::ModPathPicked(path.path().to_path_buf()).into(),
                    None => UIMessage::Noop,
                })
            }
            Message::ModPathInput(s) => {
                self.mod_path = std::path::Path::new(&s).to_path_buf();

                iced::Command::none()
            }
        }
    }

    pub fn view(&mut self) -> iced::Element<'_, UIMessage> {
        let column = iced::Column::new()
            .spacing(2)
            //todo: test skyrim
            // .push(iced::Radio::new(
            //     Game::Skyrim,
            //     "Skyrim",
            //     self.game,
            //     |game| Message::GameSelected(game).into(),
            // ))
            .push(iced::Text::new(format!("Select Game:")).size(30))
            .push(iced::Radio::new(
                Game::SkyrimSE,
                "Skyrim SE",
                self.game,
                |game| Message::GameSelected(game).into(),
            ))
            .push(iced::Radio::new(
                Game::SkyrimVR,
                "Skyrim VR (Experimental)",
                self.game,
                |game| Message::GameSelected(game).into(),
            ))
            // .push(iced::Radio::new(
            //     Game::Fallout4,
            //     "Fallout 4",
            //     self.game,
            //     |game| Message::GameSelected(game).into(),
            // ))
            .push(iced::Text::new(format!("Mod Directory:")))
            .push(
                iced::Row::new()
                    .push(
                        iced::TextInput::new(
                            &mut self.mod_path_state,
                            "",
                            self.mod_path.to_str().unwrap_or_default(),
                            |s| Message::ModPathInput(s).into(),
                        )
                        .padding(10),
                    )
                    .push(
                        iced::Button::new(&mut self.pick_mod_path_state, iced::Text::new("Pick"))
                            .on_press(Message::PickModPath.into())
                            .padding(10),
                    ),
            );
        iced::Container::new(column).into()
    }
}
