use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Display,
    hash::{Hash, Hasher},
    sync::Arc,
};

use buttplug::{
    client::{ButtplugClient, ButtplugClientDevice, ButtplugClientError, ButtplugClientEvent},
    core::messages::ButtplugCurrentSpecDeviceMessageType,
};
use iced::Subscription;
use serde::{Serialize, Deserialize};

use crate::{BodyPart, EventType};

struct ButtplugSubscription {
    client: Arc<ButtplugClient>,
}

impl ButtplugSubscription {
    pub fn new(client: Arc<ButtplugClient>) -> Self {
        Self { client }
    }
}

impl<H, I> iced_native::subscription::Recipe<H, I> for ButtplugSubscription
where
    H: Hasher,
{
    type Output = ButtplugClientEvent;

    fn hash(&self, state: &mut H) {
        std::any::TypeId::of::<Self>().hash(state);
        self.client.server_name().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: iced_futures::BoxStream<I>,
    ) -> iced_futures::BoxStream<Self::Output> {
        Box::pin(self.client.event_stream())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeviceInteraction {
    Vibrate,
    Rotate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeviceFeature {
    pub interaction: DeviceInteraction,
    pub index: u32,
}

impl Display for DeviceFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} - {}", self.interaction, self.index)
    }
}

pub struct Device {
    inner: Arc<ButtplugClientDevice>,
    mappings: BTreeMap<DeviceFeature, HashMap<crate::BodyPart, HashSet<crate::EventType>>>,
}

#[derive(Debug, Clone)]
pub enum ButtplugMessage {
    ClientEvent(ButtplugClientEvent),
    DeviceSelected(String),
    InteractionSelected(DeviceFeature),
    StopScan,
    StartScan,
    DeviceMappingChanged(BodyPart, EventType, bool),
}

pub struct State {
    pub client: Option<Arc<ButtplugClient>>,
    devices: BTreeMap<String, Device>,
    selected_device: Option<String>,
    selected_feature: Option<DeviceFeature>,

    scanning: bool,
    scan_btn: iced::button::State,
    device_list: iced::pick_list::State<String>,
    feature_list: iced::pick_list::State<DeviceFeature>,
}

async fn connect(c: Arc<ButtplugClient>) -> Result<(), ButtplugClientError> {
    c.connect_in_process(Some(Default::default())).await
}

async fn start_scan(client: Arc<ButtplugClient>) -> Result<(), ButtplugClientError> {
    client.start_scanning().await
}

async fn stop_scan(client: Arc<ButtplugClient>) -> Result<(), ButtplugClientError> {
    client.stop_scanning().await
}

impl State {
    pub fn new() -> (Self, iced::Command<crate::Message>) {
        let client = Arc::new(ButtplugClient::new("Test"));

        let inner_client = client.clone();
        let f = connect(inner_client);

        (
            Self {
                client: Some(client.clone()),
                devices: BTreeMap::new(),
                scanning: false,
                selected_device: None,
                selected_feature: None,
                scan_btn: Default::default(),
                device_list: Default::default(),
                feature_list: Default::default(),
            },
            {
                iced::Command::perform(f, |e| match e {
                    Ok(_) => crate::Message::Nothing,
                    Err(_e) => crate::Message::SomethingBroke("Buttplug".to_string()),
                })
            },
        )
    }

    pub fn update(&mut self, message: ButtplugMessage) -> iced::Command<crate::Message> {
        match message {
            ButtplugMessage::ClientEvent(ButtplugClientEvent::ScanningFinished) => {
                iced::Command::none()
            }
            ButtplugMessage::ClientEvent(ButtplugClientEvent::DeviceAdded(d)) => {
                let mut mappings = BTreeMap::new();

                if let Some(message) = d
                    .allowed_messages
                    .get(&ButtplugCurrentSpecDeviceMessageType::VibrateCmd)
                {
                    if let Some(feature_count) = message.feature_count {
                        for i in 1..=feature_count {
                            mappings.insert(
                                DeviceFeature {
                                    index: i,
                                    interaction: DeviceInteraction::Vibrate,
                                },
                                HashMap::new(),
                            );
                        }
                    }
                }

                if let Some(message) = d
                    .allowed_messages
                    .get(&ButtplugCurrentSpecDeviceMessageType::RotateCmd)
                {
                    if let Some(feature_count) = message.feature_count {
                        for i in 1..=feature_count {
                            mappings.insert(
                                DeviceFeature {
                                    index: i,
                                    interaction: DeviceInteraction::Rotate,
                                },
                                HashMap::new(),
                            );
                        }
                    }
                }

                self.devices
                    .insert(d.name.to_string(), Device { inner: d, mappings });

                iced::Command::none()
            }
            ButtplugMessage::ClientEvent(ButtplugClientEvent::DeviceRemoved(buttplug_device)) => {
                self.devices.remove(&buttplug_device.name);
                iced::Command::none()
            }
            ButtplugMessage::ClientEvent(ButtplugClientEvent::PingTimeout) => iced::Command::none(),
            ButtplugMessage::ClientEvent(ButtplugClientEvent::ServerConnect) => {
                iced::Command::none()
            }
            ButtplugMessage::ClientEvent(ButtplugClientEvent::ServerDisconnect) => {
                iced::Command::none()
            }
            ButtplugMessage::ClientEvent(ButtplugClientEvent::Error(_)) => iced::Command::none(),
            ButtplugMessage::DeviceSelected(d) => {
                self.selected_device = Some(d);
                self.selected_feature = None;
                iced::Command::none()
            }
            ButtplugMessage::InteractionSelected(i) => {
                self.selected_feature = Some(i);
                iced::Command::none()
            }
            ButtplugMessage::StopScan => {
                let f = {
                    let client = self.client.clone();
                    || async move {
                        match client {
                            Some(c) => stop_scan(c).await,
                            None => Ok(()),
                        }
                    }
                };

                self.scanning = false;

                iced::Command::perform(f(), |_| crate::Message::Nothing)
            }
            ButtplugMessage::StartScan => {
                let f = {
                    let client = self.client.clone();
                    || async move {
                        match client {
                            Some(c) => start_scan(c).await,
                            None => Ok(()),
                        }
                    }
                };

                self.scanning = true;

                iced::Command::perform(f(), |_| crate::Message::Nothing)
            }
            ButtplugMessage::DeviceMappingChanged(body_part, event_type, mapped) => {
                if let Some(selected_device) = &mut self.selected_device {
                    if let Some(selected_feature) = &mut self.selected_feature {
                        if let Some(device) = self.devices.get_mut(selected_device) {
                            if let Some(feature) = device.mappings.get_mut(selected_feature) {
                                if let Some(body_part_mapping) = feature.get_mut(&body_part) {
                                    if mapped {
                                        body_part_mapping.insert(event_type);
                                    } else {
                                        body_part_mapping.remove(&event_type);
                                    }
                                } else {
                                    let mut set = HashSet::new();
                                    if mapped {
                                        set.insert(event_type);
                                    }
                                    feature.insert(body_part, set);
                                }
                            }
                        }
                    }
                }
                iced::Command::none()
            }
        }
    }

    pub fn view(&mut self) -> iced::Element<'_, crate::Message> {
        let mut column = iced::Column::new();

        if self.scanning {
            column = column.push(
                iced::button::Button::new(&mut self.scan_btn, iced::Text::new("Stop Scanning"))
                    .on_press(crate::Message::ButtplugMessage(ButtplugMessage::StopScan)),
            );
        } else {
            column = column.push(
                iced::button::Button::new(&mut self.scan_btn, iced::Text::new("Start Scanning"))
                    .on_press(crate::Message::ButtplugMessage(ButtplugMessage::StartScan)),
            );
        }

        let devices: Vec<String> = self.devices.keys().cloned().collect();
        let device_picklist = iced::pick_list::PickList::new(
            &mut self.device_list,
            devices,
            self.selected_device.clone(),
            |s| crate::Message::ButtplugMessage(ButtplugMessage::DeviceSelected(s)),
        );

        column = column.push(device_picklist);

        let feature_picklist = {
            let mut features = Vec::new();

            if let Some(selected_device) = &self.selected_device {
                if let Some(selected_device) = self.devices.get(selected_device) {
                    features = selected_device.mappings.keys().cloned().collect();
                }
            }

            iced::pick_list::PickList::new(
                &mut self.feature_list,
                features,
                self.selected_feature.clone(),
                |s| crate::Message::ButtplugMessage(ButtplugMessage::InteractionSelected(s)),
            )
        };

        column = column.push(feature_picklist);

        if let Some(selected_device) = &self.selected_device {
            if let Some(selected_feature) = &self.selected_feature {
                let mut row = iced::Row::new();

                {
                    let mut column = iced::Column::new().push(iced::Text::new(" "));
                    for body_part in BodyPart::variants() {
                        column = column.push(iced::Text::new(format!("{:?}", body_part)))
                    }
                    row = row.push(column);
                }

                for event_type in EventType::variants() {
                    let mut column =
                        iced::Column::new().push(iced::Text::new(format!("{:?}", event_type)));

                    for body_part in BodyPart::variants() {
                        fn is_device_mapped(
                            devices: &BTreeMap<String, Device>,
                            device: &String,
                            feature: &DeviceFeature,
                            body_part: &BodyPart,
                            event_type: &EventType,
                        ) -> Option<()> {
                            if devices
                                .get(device)?
                                .mappings
                                .get(feature)?
                                .get(body_part)?
                                .get(event_type)
                                .is_some()
                            {
                                Some(())
                            } else {
                                None
                            }
                        }

                        column = column.push(iced::checkbox::Checkbox::new(
                            is_device_mapped(
                                &self.devices,
                                &selected_device,
                                &selected_feature,
                                &body_part,
                                &event_type,
                            )
                            .is_some(),
                            "",
                            move |checked| {
                                crate::Message::ButtplugMessage(
                                    ButtplugMessage::DeviceMappingChanged(
                                        body_part, event_type, checked,
                                    ),
                                )
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

    pub fn subscription(&self) -> iced::Subscription<crate::Message> {
        match &self.client {
            Some(client) => Subscription::from_recipe(ButtplugSubscription::new(client.clone()))
                .map(|e| crate::Message::ButtplugMessage(ButtplugMessage::ClientEvent(e))),
            None => Subscription::none(),
        }
    }
}
