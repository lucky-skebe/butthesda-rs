use super::UIMessage;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Game {
    // Skyrim,
    SkyrimSE,
    SkyrimVR,
    Fallout4,
}

#[derive(Debug, Clone)]
pub enum Message {
    GameSelected(Game),
    ModPathChanged(String),
}

pub struct State {
    game: Option<Game>,
    mod_path: String,
    mod_path_state: iced::text_input::State,
    // pick_mod_path_state: iced::button::State,
}

impl State {
    pub fn new() -> Self {
        Self {
            game: None,
            mod_path: String::new(),
            mod_path_state: Default::default(),
            // pick_mod_path_state: Default::default(),
        }
    }

    pub fn is_ok(&self) -> bool {
        self.game.is_some() && !self.mod_path.is_empty()
    }

    pub(crate) fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::GameSelected(game) => {
                self.game = Some(game);
                iced::Command::none()
            }
            Message::ModPathChanged(p) => {
                self.mod_path = p;
                iced::Command::none()
            }
        }
    }

    pub fn view(&mut self) -> iced::Element<'_, UIMessage> {
        let column = iced::Column::new()
            //todo: test skyrim
            // .push(iced::Radio::new(
            //     Game::Skyrim,
            //     "Skyrim",
            //     self.game,
            //     |game| Message::GameSelected(game).into(),
            // ))
            .push(iced::Radio::new(
                Game::SkyrimSE,
                "Skyrim SE",
                self.game,
                |game| Message::GameSelected(game).into(),
            ))
            .push(iced::Radio::new(
                Game::SkyrimVR,
                "Skyrim VR",
                self.game,
                |game| Message::GameSelected(game).into(),
            ))
            .push(iced::Radio::new(
                Game::Fallout4,
                "Fallout 4",
                self.game,
                |game| Message::GameSelected(game).into(),
            ))
            .push(
                iced::Row::new().push(iced::TextInput::new(
                    &mut self.mod_path_state,
                    "",
                    &self.mod_path,
                    |s| Message::ModPathChanged(s).into(),
                )), //todo: Add a File Picker
                    // .push(iced::Button::new(
                    //     &mut self.pick_mod_path_state,
                    //     ,
                    //     &self.mod_path,
                    //     |s| Message::ModPathChanged(s).into(),
                    // ))
            );
        iced::Container::new(column).into()
    }
}
