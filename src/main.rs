use std::str::FromStr;

use iced::Application;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

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

#[derive(Debug)]
pub enum Message {
    PlayerState(player_state::Message),
    SomethingBroke(String),
    FileEvent(link_file::Event),
    FunscriptsLoaded(funscript::Funscripts),
}

pub struct Options {
    file_path: String,
}

pub struct UI {
    player_state: player_state::State,
    funscripts: Option<funscript::Funscripts>,
}

impl Application for UI {
    type Executor = iced_futures::executor::Tokio;

    type Message = Message;

    type Flags = Options;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                player_state: player_state::State::new(flags.file_path),
                funscripts: None,
            },
            iced::Command::perform(
                funscript::Funscripts::load("E:\\ModOrganizer2\\SSE\\mods\\Butthesda\\FunScripts"),
                |f| Message::FunscriptsLoaded(f.unwrap()),
            ),
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
        match message {
            Message::PlayerState(message) => self.player_state.update(message),
            Message::SomethingBroke(_s) => iced::Command::none(),
            Message::FileEvent(ev) => self.player_state.handle(ev),
            Message::FunscriptsLoaded(f) => {
                self.funscripts = Some(f);
                iced::Command::none()
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let subscriptions = [self.player_state.subscription()];

        iced::Subscription::batch(subscriptions)
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        self.player_state.view()
    }
}
