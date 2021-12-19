use crate::{link_file::EquipmentState, GameState};

#[derive(Debug)]
pub struct State {
    pub arousal: u8,
    pub equipment_state: EquipmentState,
    pub detected_mods: Vec<String>,
    pub game_state: GameState,
    pub funscript_count: usize,
    btn_refresh: iced::button::State,
}

impl State {
    pub fn new() -> Self {
        Self {
            arousal: Default::default(),
            equipment_state: Default::default(),
            detected_mods: Default::default(),
            game_state: Default::default(),
            btn_refresh: Default::default(),
            funscript_count: 0,
        }
    }

    pub fn view(&mut self) -> iced::Element<'_, super::UIMessage> {
        let mut column = iced::Column::new()
            .spacing(2)
            .push(iced::Text::new(format!("Status:")).size(30))
            .push(iced::Text::new(format!("Funscripts loaded: {}", self.funscript_count)).size(25))
            .push(iced::Text::new(format!("Arousal: {}", self.arousal)).size(25))
            .push(iced::Text::new(format!("Devious Devices:")).size(25))
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
            .push(iced::Text::new(format!("Mods Detected:")).size(25));

        for detected_mod in self.detected_mods.iter() {
            column = column.push(iced::Text::new(detected_mod.clone()));
        }

        column = column
            .push(iced::Text::new(format!("Game State: {:?}", self.game_state)).size(25))
            .push(
                iced::Button::new(&mut self.btn_refresh, iced::Text::new("Refresh"))
                    .padding(10)
                    .on_press(super::UIMessage::RefreshState),
            );

        iced::Container::new(column).into()
    }
}
