use std::{str::FromStr, sync::Arc};

use ::buttplug::{
    client::{ButtplugClient, ButtplugClientError, ButtplugClientEvent},
    server::comm_managers::DeviceCommunicationManager,
};
use iced::Application;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod buttplug;
mod funscript;
mod link_file;
mod memory;
mod player_state;

fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_env_filter(EnvFilter::from_str("debug,wgpu_core=warn")?)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    Ok(UI::run(iced::Settings::with_flags(Options {
        file_path: "E:\\ModOrganizer2\\SSE\\mods\\Butthesda\\FunScripts\\link.txt".to_string(),
    }))?)

    // let mut process = Process::open(memory::SKYRIM_SE).unwrap().unwrap();

    // dbg!(process.inject());

    // println!("Pid: {}", process.pid);

    // let (sender, receiver) = tokio::sync::mpsc::channel(100);

    // let res = tokio::try_join! {
    //     {
    //         let sender = sender.clone();
    //         link_file::run(
    //         "E:\\ModOrganizer2\\SSE\\mods\\Butthesda\\FunScripts\\link.txt",
    //         sender,
    //     )}
    // };

    // if let Err(e) = res {
    //     Err(e)
    // } else {
    //     Ok(())
    // }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
}

#[derive(Debug, Clone)]
pub enum Message {
    PlayerState(player_state::Message),
    SomethingBroke(String),
    FileEvent(link_file::Event),
    FunscriptsLoaded(funscript::Funscripts),
    ButtplugEvent(ButtplugClientEvent),
    Nothing,
    StartScan,
}

pub struct Options {
    file_path: String,
}

pub struct UI {
    player_state: player_state::State,
    pub buttplug: buttplug::State,
    funscripts: Option<funscript::Funscripts>,
}

async fn start_scan(client: Arc<ButtplugClient>) -> Result<(), ButtplugClientError> {
    client.start_scanning().await
}

impl Application for UI {
    type Executor = iced_futures::executor::Tokio;

    type Message = Message;

    type Flags = Options;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let (buttplug, buttplug_command) = buttplug::State::new();
        (
            Self {
                player_state: player_state::State::new(flags.file_path),
                funscripts: None,
                buttplug,
            },
            iced::Command::batch([
                buttplug_command,
                iced::Command::perform(
                    funscript::Funscripts::load(
                        "E:\\ModOrganizer2\\SSE\\mods\\Butthesda\\FunScripts",
                    ),
                    |f| Message::FunscriptsLoaded(f.unwrap()),
                ),
            ]),
        )
    }

    fn title(&self) -> String {
        "Butthesda".to_string()
    }

    fn update(
        &mut self,
        message: Self::Message,
        _clipboard: &mut iced::Clipboard,
    ) -> iced::Command<Self::Message> {

        let f = {
            let client = self.buttplug.client.clone();
            || async move {
                match client {
                    Some(c) => start_scan(c).await,
                    None => Ok(()),
                }
            }
        };

        match message {
            Message::PlayerState(message) => self.player_state.update(message),
            Message::SomethingBroke(_s) => iced::Command::none(),
            Message::FileEvent(ev) => self.player_state.handle(ev),
            Message::FunscriptsLoaded(f) => {
                self.funscripts = Some(f);
                iced::Command::none()
            }
            Message::ButtplugEvent(ev) => self.buttplug.update(ev),
            Message::Nothing => iced::Command::none(),
            Message::StartScan => iced::Command::perform(
                f(),
                |_| Message::Nothing,
            ),
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let subscriptions = [
            self.player_state.subscription(),
            self.buttplug.subscription(),
        ];

        iced::Subscription::batch(subscriptions)
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        let row = iced::Row::new()
            .push(self.player_state.view())
            .push(self.buttplug.view());

        iced::Container::new(row).into()
    }
}
