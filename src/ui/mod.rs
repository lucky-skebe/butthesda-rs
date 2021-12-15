use futures::StreamExt;
use iced::{Application, Length};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tokio::io::AsyncReadExt;
use tracing::error;

use crate::{
    buttplug::{ButtplugConnection, ButtplugOutMessage},
    util::{MaybeFrom, StreamSubscription},
    LazyStaticTokioExecutor, Message,
};

mod devices;
mod game_select;
mod start;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Page {
    GameSelect,
    Devices,
    Start,
}

impl Page {
    fn prev(&self) -> Option<Self> {
        match self {
            Page::GameSelect => None,
            Page::Devices => Some(Self::GameSelect),
            Page::Start => Some(Self::Devices),
        }
    }

    fn next(&self) -> Option<Self> {
        match self {
            Page::GameSelect => Some(Self::Devices),
            Page::Devices => Some(Self::Start),
            Page::Start => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ButtplugInMessage {
    DeviceConnected(String, (u32, u32, u32)),
    DeviceDisconnected(String),
    StartScanning,
    StopScanning,
}

#[derive(Debug, Clone)]
pub enum InMessage {
    Buttplug(ButtplugInMessage),
    Device(crate::device::ConfigMessage),
}

impl MaybeFrom<crate::Message> for UIMessage {
    fn maybe_from(from: crate::Message) -> Option<Self> {
        match from {
            Message::ButtplugIn(::buttplug::client::ButtplugClientEvent::DeviceAdded(device)) => {
                let vibrators = device
                    .allowed_messages
                    .get(
                        &buttplug::core::messages::ButtplugCurrentSpecDeviceMessageType::VibrateCmd,
                    )
                    .map(|attributes| attributes.feature_count)
                    .flatten()
                    .unwrap_or_default();

                let rotators = device
                    .allowed_messages
                    .get(&buttplug::core::messages::ButtplugCurrentSpecDeviceMessageType::RotateCmd)
                    .map(|attributes| attributes.feature_count)
                    .flatten()
                    .unwrap_or_default();

                let actuators = device
                    .allowed_messages
                    .get(&buttplug::core::messages::ButtplugCurrentSpecDeviceMessageType::LinearCmd)
                    .map(|attributes| attributes.feature_count)
                    .flatten()
                    .unwrap_or_default();

                Some(UIMessage::InMessage(InMessage::Buttplug(
                    ButtplugInMessage::DeviceConnected(
                        device.name.clone(),
                        (vibrators, rotators, actuators),
                    ),
                )))
            }
            Message::ButtplugIn(::buttplug::client::ButtplugClientEvent::DeviceRemoved(device)) => {
                Some(UIMessage::InMessage(InMessage::Buttplug(
                    ButtplugInMessage::DeviceDisconnected(device.name.clone()),
                )))
            }
            Message::ButtplugIn(_) => None,
            Message::ButtplugOut(ButtplugOutMessage::StartScan) => Some(UIMessage::InMessage(
                InMessage::Buttplug(ButtplugInMessage::StartScanning),
            )),
            Message::ButtplugOut(ButtplugOutMessage::StopScan) => Some(UIMessage::InMessage(
                InMessage::Buttplug(ButtplugInMessage::StopScanning),
            )),
            Message::ButtplugOut(ButtplugOutMessage::ConnectTo(_)) => None,
            Message::ButtplugOut(ButtplugOutMessage::Disconnect) => None,

            Message::DeviceConfiguration(msg) => Some(UIMessage::InMessage(InMessage::Device(msg))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    game_select: game_select::Config,
    devices: devices::Config,
}

impl Config {
    async fn load(path: PathBuf) -> anyhow::Result<Self> {
        let mut file = tokio::fs::File::open(path).await?;
        let mut content = String::new();
        file.read_to_string(&mut content).await?;
        let config = serde_json::from_str::<Config>(&content)?;
        Ok(config)
    }
}

// MessageBus,
// PlayerState(debug::Message),
// Error(String),
// FileEvent(link_file::Event),
// FunscriptsLoaded(funscript::Funscripts),
// ButtplugMessage(buttplug::ButtplugMessage)
#[derive(Debug, Clone)]
pub enum UIMessage {
    InMessage(InMessage),
    OutMessage(Message),
    GameSelect(game_select::Message),
    Devices(devices::Message),
    SelectPage(Page),
    Error(String, bool),
    Save,
    Close,
    Load(PathBuf),
    Loaded(Config),
}

impl From<game_select::Message> for UIMessage {
    fn from(message: game_select::Message) -> Self {
        Self::GameSelect(message)
    }
}

impl From<devices::Message> for UIMessage {
    fn from(message: devices::Message) -> Self {
        Self::Devices(message)
    }
}

pub struct Options {
    pub message_bus: Arc<tokio::sync::broadcast::Sender<crate::Message>>,
}

pub struct UI {
    page: Page,
    message_bus: Arc<tokio::sync::broadcast::Sender<Message>>,
    game_select: game_select::State,
    devices: devices::State,
    start: start::State,
    btn_prev: iced::button::State,
    btn_game_select: iced::button::State,
    btn_devices: iced::button::State,
    btn_start: iced::button::State,
    btn_next: iced::button::State,
    btn_load: iced::button::State,
    btn_save: iced::button::State,
    close: bool,
}

impl UI {
    fn is_page_ok(&mut self) -> bool {
        match self.page {
            Page::GameSelect => self.game_select.is_ok(),
            Page::Devices => self.devices.is_ok(),
            Page::Start => self.start.is_ok(),
        }
    }

    fn load(&mut self, config: &Config) {
        self.game_select.load(&config.game_select);
    }

    fn save(&self) -> Option<Config> {
        Some(Config {
            devices: self.devices.save(),
            game_select: self.game_select.save()?,
        })
    }
}

impl Application for UI {
    type Executor = LazyStaticTokioExecutor;

    type Message = UIMessage;

    type Flags = Options;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                page: Page::GameSelect,
                message_bus: flags.message_bus.clone(),
                game_select: game_select::State::new(),
                devices: devices::State::new(),
                start: start::State::new(),
                btn_prev: iced::button::State::new(),
                btn_game_select: iced::button::State::new(),
                btn_devices: iced::button::State::new(),
                btn_start: iced::button::State::new(),
                btn_next: iced::button::State::new(),
                btn_load: iced::button::State::new(),
                btn_save: iced::button::State::new(),
                close: false,
            },
            iced::Command::perform(async {}, |_| {
                UIMessage::OutMessage(Message::ButtplugOut(ButtplugOutMessage::ConnectTo(
                    ButtplugConnection::InProcess,
                )))
            }),
        )
    }

    fn should_exit(&self) -> bool {
        self.close
    }

    fn title(&self) -> String {
        "Butthesda".to_string()
    }

    fn update(
        &mut self,
        message: Self::Message,
        _clipboard: &mut iced::Clipboard,
    ) -> iced::Command<Self::Message> {
        let load = |path| Config::load(path);

        match message {
            UIMessage::GameSelect(message) => self.game_select.update(message).map(Into::into),
            UIMessage::SelectPage(p) => {
                self.page = p;
                iced::Command::none()
            }
            UIMessage::InMessage(InMessage::Buttplug(ButtplugInMessage::DeviceConnected(
                name,
                (vibrators, rotators, actuators),
            ))) => {
                self.devices
                    .add_device(name, vibrators, rotators, actuators);
                iced::Command::none()
            }
            UIMessage::InMessage(InMessage::Buttplug(ButtplugInMessage::DeviceDisconnected(
                name,
            ))) => {
                self.devices.remove_device(name);
                iced::Command::none()
            }
            UIMessage::InMessage(InMessage::Buttplug(ButtplugInMessage::StartScanning)) => {
                self.devices.scanning = true;
                iced::Command::none()
            }
            UIMessage::InMessage(InMessage::Buttplug(ButtplugInMessage::StopScanning)) => {
                self.devices.scanning = false;
                iced::Command::none()
            }
            UIMessage::InMessage(InMessage::Device(crate::device::ConfigMessage::Change(c))) => {
                self.devices.device_config.set_should_handle(
                    c.device,
                    c.feature,
                    c.body_part,
                    c.event_type,
                    c.should_handle,
                );
                iced::Command::none()
            }
            UIMessage::InMessage(InMessage::Device(crate::device::ConfigMessage::Complete(
                config,
            ))) => {
                self.devices.device_config = config;
                iced::Command::none()
            }
            UIMessage::OutMessage(message) => {
                if let Err(e) = self.message_bus.send(message) {
                    error!("{}", e);
                    self.close = true;
                }
                iced::Command::none()
            }
            UIMessage::Devices(message) => self.devices.update(message).map(Into::into),
            UIMessage::Error(e, close) => {
                error!("{}", e);
                self.close = close;
                iced::Command::none()
            }
            UIMessage::Save => {
                let config = self.save();
                dbg!(config);
                
                iced::Command::none()
            }
            UIMessage::Close => {
                self.close = true;
                iced::Command::none()
            }
            UIMessage::Load(path) => iced::Command::perform(load(path), |m| match m{
                Ok(c) => UIMessage::Loaded(c),
                Err(e) => UIMessage::Error(format!("{}", e), false),
            }),
            UIMessage::Loaded(config) => {
                self.load(&config);

                iced::Command::perform(async { UIMessage::OutMessage(crate::Message::DeviceConfiguration(crate::device::ConfigMessage::Complete(config.devices))) }, |m| m)
            }
            // Message::PlayerState(message) => self.player_state.update(message),
            // Message::Error(s) => {
            //     error!("{}", s);
            //     iced::Command::none()
            // }
            // Message::FileEvent(ev) => self.player_state.handle(ev),
            // Message::FunscriptsLoaded(f) => {
            //     self.funscripts = Some(f);
            //     iced::Command::none()
            // }
            // Message::ButtplugMessage(ev) => self.buttplug.update(ev),
            // Message::Nothing => iced::Command::none(),
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        // let subscriptions = [
        //     self.player_state.subscription(),
        //     self.buttplug.subscription(),
        //     iced::Subscription::from_recipe(StreamSubscription::new(
        //         tokio_stream::wrappers::BroadcastStream::new(self.link_file_sender.subscribe()),
        //     ))
        //     .map(|result| match result {
        //         Ok(event) => Message::FileEvent(event),
        //         Err(_) => Message::Error("File Scanner".to_string()),
        //     }),
        // ];

        // iced::Subscription::batch(subscriptions)

        iced::Subscription::batch([
            iced_native::subscription::events_with(|e, _| match e {
                iced_native::Event::Window(iced_native::window::Event::FileDropped(path)) => {
                    Some(UIMessage::Load(path))
                }
                _ => None,
            }),
            iced::Subscription::from_recipe(StreamSubscription::new(
                tokio_stream::wrappers::BroadcastStream::new(self.message_bus.subscribe())
                    .filter_map(|r| async {
                        match r {
                            Ok(m) => UIMessage::maybe_from(m),
                            Err(e) => Some(UIMessage::Error(format!("{}", e), true)),
                        }
                    }),
            )),
        ])
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        let page_ok = self.is_page_ok();

        let mut btn_prev = iced::Button::new(&mut self.btn_prev, iced::Text::new("<"));
        let mut btn_game_select =
            iced::Button::new(&mut self.btn_game_select, iced::Text::new("Select Game"));
        let mut btn_devices =
            iced::Button::new(&mut self.btn_devices, iced::Text::new("Map Devices"));
        let mut btn_start = iced::Button::new(&mut self.btn_start, iced::Text::new("Start"));
        let mut btn_next = iced::Button::new(&mut self.btn_next, iced::Text::new(">"));

        if let Some(prev) = self.page.prev() {
            btn_prev = btn_prev.on_press(UIMessage::SelectPage(prev));
        }

        if self.page >= Page::GameSelect {
            btn_game_select = btn_game_select.on_press(UIMessage::SelectPage(Page::GameSelect));
        }

        if self.page >= Page::Devices || self.game_select.is_ok() {
            btn_devices = btn_devices.on_press(UIMessage::SelectPage(Page::Devices));
        }

        if self.page >= Page::Start || self.devices.is_ok() {
            btn_start = btn_start.on_press(UIMessage::SelectPage(Page::Start));
        }

        if let Some(next) = self.page.next() {
            if page_ok {
                btn_next = btn_next.on_press(UIMessage::SelectPage(next));
            }
        }

        let stepper = iced::Row::new()
            .width(Length::Fill)
            .push(btn_prev)
            .push(iced::Space::with_width(Length::Fill))
            .push(btn_game_select)
            .push(btn_devices)
            .push(btn_start)
            .push(iced::Space::with_width(Length::Fill))
            .push(btn_next);

        let header = iced::Row::new()
            .push(match self.page {
                Page::GameSelect => iced::Text::new("Select the game you are playing:"),
                Page::Devices => iced::Text::new("Map your Devices to Events:"),
                Page::Start => iced::Text::new("Play the Game:"),
            })
            .push(iced::Space::with_width(Length::Fill))
            .push(iced::Button::new(
                &mut self.btn_load,
                iced::Text::new("Load"),
            ))
            .push(
                iced::Button::new(&mut self.btn_save, iced::Text::new("Save"))
            );

        let column = iced::Column::new()
            .push(header)
            .push(match self.page {
                Page::GameSelect => iced::Row::new().push(self.game_select.view()),
                Page::Devices => iced::Row::new().push(self.devices.view()),
                Page::Start => iced::Row::new().push(self.start.view()),
            })
            .push(iced::Space::with_height(Length::Fill))
            .push(stepper);

        iced::Container::new(column)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}
