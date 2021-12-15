use std::{collections::BTreeMap, sync::Arc};

pub use crate::device::Config;
use crate::{buttplug::DeviceFeature, BodyPart, EventType};

#[derive(Debug, Clone)]
pub enum Message {
    DeviceSelected(String),
    FeatureSelected(DeviceFeature),
}

pub struct State {
    pub(crate) device_config: Config,
    devices: BTreeMap<String, (u32, u32, u32)>,
    selected_device: Option<String>,
    selected_feature: Option<DeviceFeature>,
    pub(crate) scanning: bool,
    scan_btn: iced::button::State,
    device_list: iced::pick_list::State<String>,
    feature_list: iced::pick_list::State<DeviceFeature>,
}

impl State {
    pub fn new() -> Self {
        Self {
            device_config: Default::default(),
            devices: BTreeMap::new(),
            scanning: false,
            selected_device: None,
            selected_feature: None,
            scan_btn: Default::default(),
            device_list: Default::default(),
            feature_list: Default::default(),
        }
    }

    pub fn is_ok(&self) -> bool {
        !self.devices.is_empty()
    }

    pub fn save(&self) -> Config {
        self.device_config.clone()
    }

    pub(crate) fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::DeviceSelected(device) => {
                self.selected_device = Some(device);
                iced::Command::none()
            }
            Message::FeatureSelected(feature) => {
                self.selected_feature = Some(feature);
                iced::Command::none()
            }
        }
    }

    pub fn view(&mut self) -> iced::Element<'_, super::UIMessage> {
        let mut column = iced::Column::new();

        if self.scanning {
            column = column.push(
                iced::button::Button::new(&mut self.scan_btn, iced::Text::new("Stop Scanning"))
                    .on_press(super::UIMessage::OutMessage(crate::Message::ButtplugOut(
                        crate::buttplug::ButtplugOutMessage::StopScan,
                    ))),
            );
        } else {
            column = column.push(
                iced::button::Button::new(&mut self.scan_btn, iced::Text::new("Start Scanning"))
                    .on_press(super::UIMessage::OutMessage(crate::Message::ButtplugOut(
                        crate::buttplug::ButtplugOutMessage::StartScan,
                    ))),
            );
        }

        let devices: Vec<_> = self.devices.keys().cloned().collect();
        let device_picklist = iced::pick_list::PickList::new(
            &mut self.device_list,
            devices,
            self.selected_device.clone(),
            |s| Message::DeviceSelected(s).into(),
        );

        column = column.push(device_picklist);

        let feature_picklist = {
            let mut features = Vec::new();

            if let Some(selected_device) = &self.selected_device {
                if let Some(selected_device) = self.devices.get(selected_device) {
                    for index in 0..selected_device.0 {
                        features.push(DeviceFeature {
                            index,
                            interaction: crate::buttplug::DeviceInteraction::Vibrate,
                        });
                    }

                    for index in 0..selected_device.1 {
                        features.push(DeviceFeature {
                            index,
                            interaction: crate::buttplug::DeviceInteraction::Rotate,
                        });
                    }

                    // for index in 0..selected_device.2 {} //todo: linear devices
                }
            }

            iced::pick_list::PickList::new(
                &mut self.feature_list,
                features,
                self.selected_feature.clone(),
                |s| Message::FeatureSelected(s).into(),
            )
        };

        column = column.push(feature_picklist);

        if let Some(selected_device) = &self.selected_device {
            if let Some(selected_feature) = &self.selected_feature {
                let mut row = iced::Row::new();

                {
                    let mut column = iced::Column::new()
                        .push(iced::Text::new(" "))
                        .spacing(2)
                        .width(iced::Length::Fill);
                    for body_part in BodyPart::variants() {
                        column = column.push(iced::Text::new(format!("{:?}", body_part)))
                    }

                    row = row.push(column);
                }

                for event_type in EventType::variants() {
                    let mut column = iced::Column::new()
                        .push(iced::Text::new(format!("{:?}", event_type)))
                        .spacing(2)
                        .width(iced::Length::Fill)
                        .align_items(iced::Align::Center);

                    for body_part in BodyPart::variants() {
                        let device = Arc::new(selected_device.clone());
                        let feature = Arc::new(selected_feature.clone());

                        column = column.push(iced::checkbox::Checkbox::new(
                            self.device_config.should_handle(
                                &selected_device,
                                &selected_feature,
                                &body_part,
                                &event_type,
                            ),
                            "",
                            move |should_handle| {
                                super::UIMessage::OutMessage(crate::Message::DeviceConfiguration(
                                    crate::device::ConfigMessage::Change(
                                        crate::device::ConfigChange {
                                            device: (*device).clone(),
                                            feature: (*feature).clone(),
                                            body_part,
                                            event_type,
                                            should_handle,
                                        },
                                    ),
                                ))
                            },
                        ));
                    }

                    row = row.push(column);
                }

                column = column.push(row);
            }
        }

        iced::Container::new(column).into()
    }

    pub(crate) fn add_device(
        &mut self,
        name: String,
        vibrators: u32,
        rotators: u32,
        actuators: u32,
    ) {
        self.devices.insert(name, (vibrators, rotators, actuators));
    }

    pub(crate) fn remove_device(&mut self, name: String) {
        self.devices.remove(&name);
    }
}