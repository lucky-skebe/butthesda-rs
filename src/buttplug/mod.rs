use std::{
    collections::{BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

use buttplug::client::{
    ButtplugClient, ButtplugClientDevice, ButtplugClientError, ButtplugClientEvent,
};
use iced::Subscription;

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

pub struct Device {
    inner: Arc<ButtplugClientDevice>,
    mapping: HashMap<crate::BodyPart, HashSet<crate::EventType>>,
}

#[derive(Debug, Clone)]
pub enum ButtplugMessage {
    ClientEvent(ButtplugClientEvent),
    DeviceSelected(String),
    StopScan,
    StartScan,
    DeviceMappingChanged(BodyPart, EventType, bool),
}

pub struct State {
    pub client: Option<Arc<ButtplugClient>>,
    devices: BTreeMap<String, Device>,
    selected_device: Option<String>,

    scanning: bool,
    scan_btn: iced::button::State,
    device_list: iced::pick_list::State<String>,
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
                scan_btn: Default::default(),
                device_list: Default::default(),
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
                self.devices.insert(
                    d.name.to_string(),
                    Device {
                        inner: d,
                        mapping: HashMap::new(),
                    },
                );

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
            ButtplugMessage::DeviceSelected(s) => {
                self.selected_device = Some(s);
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
                    if let Some(device) = self.devices.get_mut(selected_device) {
                        if let Some(body_part_mapping) = device.mapping.get_mut(&body_part) {
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
                            device.mapping.insert(body_part, set);
                        }
                    }
                }
                iced::Command::none()
            }
        }
    }

    pub fn view(&mut self) -> iced::Element<'_, crate::Message> {
        let keys: Vec<String> = self.devices.keys().cloned().collect();

        let picklist = iced::pick_list::PickList::new(
            &mut self.device_list,
            keys,
            self.selected_device.clone(),
            |s| crate::Message::ButtplugMessage(ButtplugMessage::DeviceSelected(s)),
        );

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

        column = column.push(picklist);

        if let Some(selected_device) = &self.selected_device {
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
                        body_part: &BodyPart,
                        event_type: &EventType,
                    ) -> Option<()> {
                        if devices
                            .get(device)?
                            .mapping
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
                        is_device_mapped(&self.devices, &selected_device, &body_part, &event_type)
                            .is_some(),
                        "",
                        move |checked| {
                            crate::Message::ButtplugMessage(ButtplugMessage::DeviceMappingChanged(
                                body_part, event_type, checked,
                            ))
                        },
                    ));
                }

                row = row.push(column);
            }

            column = column.push(row);
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
