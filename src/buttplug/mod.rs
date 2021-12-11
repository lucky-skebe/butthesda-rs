use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

use buttplug::client::{
    ButtplugClient, ButtplugClientDevice, ButtplugClientError, ButtplugClientEvent,
};
use iced::Subscription;

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

pub struct State {
    pub client: Option<Arc<ButtplugClient>>,
    devices: Vec<Device>,
    scan_btn: iced::button::State,
}

async fn connect(c: Arc<ButtplugClient>) -> Result<(), ButtplugClientError> {
    c.connect_in_process(Some(Default::default())).await
}

impl State {
    pub fn new() -> (Self, iced::Command<crate::Message>) {
        let client = Arc::new(ButtplugClient::new("Test"));

        let inner_client = client.clone();
        let f = connect(inner_client);

        (
            Self {
                client: Some(client.clone()),
                devices: Vec::new(),
                scan_btn: Default::default(),
            },
            {
                iced::Command::perform(f, |e| match e {
                    Ok(_) => crate::Message::Nothing,
                    Err(e_) => crate::Message::SomethingBroke("Buttplug".to_string()),
                })
            },
        )
    }

    pub fn update(&mut self, message: ButtplugClientEvent) -> iced::Command<crate::Message> {
        match message {
            ButtplugClientEvent::ScanningFinished => todo!(),
            ButtplugClientEvent::DeviceAdded(d) => {
                self.devices.push(Device {
                    inner: d,
                    mapping: HashMap::new(),
                });
            }
            ButtplugClientEvent::DeviceRemoved(buttplug_device) => {
                let indices: Vec<_> = {
                    let iter = self.devices.iter();
                    iter.enumerate()
                        .rev()
                        .filter(|(_, d)| d.inner.name == buttplug_device.name)
                        .map(|(i, _)| i)
                        .collect()
                };

                for index in indices {
                    self.devices.remove(index);
                }
            }
            ButtplugClientEvent::PingTimeout => todo!(),
            ButtplugClientEvent::ServerConnect => todo!(),
            ButtplugClientEvent::ServerDisconnect => todo!(),
            ButtplugClientEvent::Error(_) => todo!(),
        }
        iced::Command::none()
    }

    pub fn view(&mut self) -> iced::Element<'_, crate::Message> {
        let mut column = iced::Column::new().push(
            iced::button::Button::new(&mut self.scan_btn, iced::Text::new("Start Scanning"))
                .on_press(crate::Message::StartScan),
        );

        for d in self.devices.iter() {
            column = column.push(iced::Text::new(d.inner.name.clone()));
        }

        iced::Container::new(column).into()
    }

    pub fn subscription(&self) -> iced::Subscription<crate::Message> {
        match &self.client {
            Some(client) => Subscription::from_recipe(ButtplugSubscription::new(client.clone()))
                .map(|e| crate::Message::ButtplugEvent(e)),
            None => Subscription::none(),
        }
    }
}
