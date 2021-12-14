use crate::GameState;

#[derive(Debug)]
pub struct State {
    arousal: u8,
    // equipment_state: EquipmentState,
    detected_mods: Vec<String>,
    game_status: GameState,
}

impl State {
    pub fn new() -> Self {
        Self {
            arousal: Default::default(),
            // equipment_state: Default::default(),
            detected_mods: Default::default(),
            game_status: Default::default(),
        }
    }

    pub fn is_ok(&self) -> bool {
        false
    }

    pub fn view(&mut self) -> iced::Element<'_, super::UIMessage> {
        let mut column = iced::Column::new()
            .push(iced::Text::new(format!("Arousal: {}", self.arousal)))
            .push(iced::Text::new(format!("Devious Devices:")))
            // .push(iced::Text::new(format!(
            //     "Vaginal Plug: {:?}",
            //     self.equipment_state.vaginal
            // )))
            // .push(iced::Text::new(format!(
            //     "Anal Plug: {:?}",
            //     self.equipment_state.anal
            // )))
            // .push(iced::Text::new(format!(
            //     "Vaginal Piercing: {:?}",
            //     self.equipment_state.vaginal_piercing
            // )))
            // .push(iced::Text::new(format!(
            //     "Nipple Piercing: {:?}",
            //     self.equipment_state.nipple_piercing
            // )))
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
}
