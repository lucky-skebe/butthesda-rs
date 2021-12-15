use std::{hash::Hash, str::FromStr, sync::Arc};

use iced::Application;
use iced_native::Widget;
use image::ImageDecoder;
use serde::{Deserialize, Serialize};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

pub mod buttplug;
pub mod device;
// pub mod funscript;
// pub mod link_file;
// pub mod process;
mod ui;
pub mod util;

pub type MessageBus = tokio::sync::broadcast::Sender<Message>;

const ICON: &[u8] = include_bytes!("../app.ico");

#[derive(Debug, Clone)]
pub enum Message {
    ButtplugOut(buttplug::ButtplugOutMessage),
    ButtplugIn(::buttplug::client::ButtplugClientEvent),
    DeviceConfiguration(device::ConfigMessage),
}

impl From<::buttplug::client::ButtplugClientEvent> for Message {
    fn from(from: ::buttplug::client::ButtplugClientEvent) -> Self {
        Self::ButtplugIn(from)
    }
}

lazy_static::lazy_static! {
    static ref RUNTIME: tokio::runtime::Runtime = {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    };
}

pub struct LazyStaticTokioExecutor;

impl iced_futures::Executor for LazyStaticTokioExecutor {
    fn new() -> Result<Self, futures::io::Error>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn spawn(&self, future: impl futures::Future<Output = ()> + Send + 'static) {
        let _ = RUNTIME.spawn(future);
    }

    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = RUNTIME.enter();
        f()
    }
}

fn main() -> anyhow::Result<()> {
    RUNTIME.block_on(async {
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(EnvFilter::from_str("error,butthesda_rs=debug")?)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        let (message_bus, message_bus_handle) = tokio::sync::broadcast::channel::<Message>(100);

        let _buttplug_handle = tokio::spawn(buttplug::run(message_bus.clone()));

        // let _handle = tokio::spawn(device::run(message_bus_handle));

        let icon_reader =
            image::io::Reader::with_format(std::io::Cursor::new(ICON), image::ImageFormat::Ico);

        let icon = icon_reader.decode().unwrap();
        let icon = icon.into_rgba8();

        let icon_height = icon.height();
        let icon_width = icon.width();
        let icon_data = icon.into_raw();

        let mut settings = iced::Settings::with_flags(ui::Options {
            message_bus: Arc::new(message_bus),
        });

        settings.window = iced::window::Settings {
            icon: Some(iced::window::Icon::from_rgba(icon_data, icon_width, icon_height).unwrap()),
            ..Default::default()
        };
        Ok(ui::UI::run(settings)?)
    })
}

#[derive(Debug, PartialEq, Eq)]
enum GameState {
    Stopped,
    Running,
    Paused,
}

impl Default for GameState {
    fn default() -> Self {
        Self::Stopped
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all ="camelCase")]
pub enum BodyPart {
    Head,
    Body,
    Breast,
    Belly,
    Feet,
    Mouth,
    Vaginal,
    Clit,
    Anal,
}

impl BodyPart {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "head" => Some(Self::Head),
            "body" => Some(Self::Body),
            "breast" => Some(Self::Breast),
            "belly" => Some(Self::Belly),
            "feet" => Some(Self::Feet),
            "mouth" => Some(Self::Mouth),
            "vaginal" => Some(Self::Vaginal),
            "clit" => Some(Self::Clit),
            "anal" => Some(Self::Anal),
            _ => None,
        }
    }

    fn variants() -> [Self; 9] {
        [
            Self::Head,
            Self::Body,
            Self::Breast,
            Self::Belly,
            Self::Feet,
            Self::Mouth,
            Self::Vaginal,
            Self::Clit,
            Self::Anal,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all ="camelCase")]
pub enum EventType {
    Shock,
    Damage,
    Penetrate,
    Vibrate,
    Equip,
}

impl EventType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "shock" => Some(Self::Shock),
            "damage" => Some(Self::Damage),
            "penetrate" => Some(Self::Penetrate),
            "vibrate" => Some(Self::Vibrate),
            "equip" => Some(Self::Equip),
            _ => None,
        }
    }

    fn variants() -> [Self; 5] {
        [
            Self::Shock,
            Self::Damage,
            Self::Penetrate,
            Self::Vibrate,
            Self::Equip,
        ]
    }
}
