use iced::Subscription;

use crate::link_file::{EquipmentChanged as EqipmentState, Event, LinkFileScanner};

#[derive(Debug, Clone)]
enum GameState {
    Running,
    Paused,
    NotRunning,
}

impl Default for GameState {
    fn default() -> Self {
        Self::NotRunning
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Arousal(u8),
}

#[derive(Debug)]
pub struct State {
    arousal: u8,
    equipment_state: EqipmentState,
    detected_mods: Vec<String>,
    game_status: GameState,
    file_path: String,
}

impl State {
    pub fn new(file_path: String) -> Self {
        Self {
            arousal: Default::default(),
            equipment_state: Default::default(),
            detected_mods: Default::default(),
            game_status: Default::default(),
            file_path,
        }
    }

    pub fn update(&mut self, _message: Message) -> iced::Command<crate::Message> {
        iced::Command::none()
    }

    pub fn handle(&mut self, ev: crate::link_file::Event) -> iced::Command<crate::Message> {
        match ev {
            Event::DD(crate::link_file::DDEvent::EquipmentChanged(state)) => {
                self.equipment_state = state;
            }
            Event::DD(_) => {}
            Event::Game(crate::link_file::GameEvent::MenuClosed) => {
                self.game_status = GameState::Running
            }
            Event::Game(crate::link_file::GameEvent::MenuOpened) => {
                self.game_status = GameState::Paused
            }
            Event::Game(crate::link_file::GameEvent::LoadingSave(l)) => {
                let mut mods = Vec::new();

                if l.bf_running {
                    mods.push("Being Female".to_string())
                }

                if l.dd_running {
                    mods.push("Devious Devices".to_string())
                }

                if l.mme_running {
                    mods.push("Milk Mod Economy".to_string())
                }

                if l.sgo_running {
                    mods.push("Soulgem Oven".to_string())
                }

                if l.sla_running {
                    mods.push("SexLab Aroused".to_string())
                }

                self.detected_mods = mods;
            }
            Event::Game(_) => {}
            Event::Sla(crate::link_file::SLAEvent { arousal }) => {
                self.arousal = arousal;
            }
            Event::Sexlab(_) => todo!(),
            Event::MilkMod(_) => todo!(),
        }
        iced::Command::none()
    }

    pub fn view(&mut self) -> iced::Element<'_, crate::Message> {
        let mut column = iced::Column::new()
            .push(iced::Text::new(format!("Arousal: {}", self.arousal)))
            .push(iced::Text::new(format!("Devious Devices:")))
            .push(iced::Text::new(format!(
                "Vaginal Plug: {:?}",
                self.equipment_state.vaginal
            )))
            .push(iced::Text::new(format!(
                "Anal Plug: {:?}",
                self.equipment_state.anal
            )))
            .push(iced::Text::new(format!(
                "Vaginal Piercing: {:?}",
                self.equipment_state.vaginal_piercing
            )))
            .push(iced::Text::new(format!(
                "Nipple Piercing: {:?}",
                self.equipment_state.nipple_piercing
            )))
            .push(iced::Text::new(format!("Mods Detected:")));

        for detected_mod in self.detected_mods.iter() {
            column = column.push(iced::Text::new(detected_mod.clone()));
        }

        column = column.push(iced::Text::new(format!(
            "Game State: {:?}",
            self.game_status
        )));

        iced::Container::new(column).into()
    }

    pub fn subscription(&self) -> iced::Subscription<crate::Message> {
        Subscription::from_recipe(LinkFileScanner::new(self.file_path.clone()))
            .map(|e| crate::Message::FileEvent(e))
    }
}
