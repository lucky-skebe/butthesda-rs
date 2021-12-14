mod contracts;

use std::{sync::Arc, time::Duration};

pub use contracts::*;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info};

use crate::device::LogicMessage;
use crate::util::MaybeFrom;

struct RunningState {
    file: tokio::fs::File,
    loading: bool,
    starting: bool,
    index: usize,
    content: String,
}

enum LinkFileScannerState {
    Init(String),
    Running(RunningState),
}

pub struct LinkFileScanner(String);

impl LinkFileScanner {
    pub fn new(path: String) -> Self {
        Self(path)
    }
}

impl<H, I> iced_native::subscription::Recipe<H, I> for LinkFileScanner
where
    H: std::hash::Hasher,
{
    type Output = Event;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
        self.0.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: iced_futures::BoxStream<I>,
    ) -> iced_futures::BoxStream<Self::Output> {
        Box::pin(futures::stream::unfold(
            LinkFileScannerState::Init(self.0.clone()),
            |state| async move {
                match state {
                    LinkFileScannerState::Init(path) => {
                        let mut file = tokio::fs::File::open(&path).await.ok()?;
                        let mut content = String::new();

                        let mut starting = true;
                        let mut loading = false;

                        let mut last_index = 0;

                        file.read_to_string(&mut content).await.ok()?;

                        content = content
                            .replace("':TRUE", "':true")
                            .replace("':False", "':false")
                            .replace("'", "\"");

                        loop {
                            for (index, char) in content.chars().enumerate() {
                                if char == '\n' {
                                    let line = &content[last_index..index];
                                    last_index = index + 1;
                                    if line.starts_with("{") {
                                        if line != "{}" {
                                            let event = serde_json::from_str::<Event>(line);

                                            match event {
                                                Ok(Event::Game(GameEvent::LoadingSaveDone)) => {
                                                    loading = false;
                                                }
                                                Ok(ev @ Event::Game(GameEvent::LoadingSave(_))) => {
                                                    loading = true;
                                                    return Some((
                                                        ev,
                                                        LinkFileScannerState::Running(
                                                            RunningState {
                                                                content,
                                                                file,
                                                                index,
                                                                loading,
                                                                starting,
                                                            },
                                                        ),
                                                    ));
                                                }
                                                Ok(ev @ Event::Sla(_))
                                                | Ok(
                                                    ev @ Event::DD(DDEvent::EquipmentChanged(_)),
                                                ) => {
                                                    info!(?ev, "Handling Event");

                                                    return Some((
                                                        ev,
                                                        LinkFileScannerState::Running(
                                                            RunningState {
                                                                content,
                                                                file,
                                                                index,
                                                                loading,
                                                                starting,
                                                            },
                                                        ),
                                                    ));
                                                }
                                                Ok(ev) if loading | starting => {
                                                    debug!(?ev, "Skipping Event");
                                                }
                                                Ok(ev) => {
                                                    info!(?ev, "Handling Event");

                                                    return Some((
                                                        ev,
                                                        LinkFileScannerState::Running(
                                                            RunningState {
                                                                content,
                                                                file,
                                                                index,
                                                                loading,
                                                                starting,
                                                            },
                                                        ),
                                                    ));
                                                }
                                                Err(e) => {
                                                    error!(?e, ?line, "Could not Parse Event")
                                                }
                                            }
                                        }
                                    } else {
                                        info!("{}", line);
                                    }
                                }
                            }
                            starting = false;

                            tokio::time::sleep(Duration::from_millis(100)).await;

                            content.clear();
                            file.read_to_string(&mut content).await.ok()?;

                            content = content
                                .replace("':TRUE", "':true")
                                .replace("':False", "':false")
                                .replace("'", "\"");

                            last_index = 0;
                        }
                    }
                    LinkFileScannerState::Running(RunningState {
                        content,
                        starting,
                        loading,
                        index,
                        file,
                    }) => {
                        let mut last_index = index + 1;
                        let mut loading = loading;
                        let mut starting = starting;
                        let mut content = content;
                        let mut file = file;

                        loop {
                            for (index, char) in content.chars().enumerate().skip(last_index) {
                                if char == '\n' {
                                    let line = &content[last_index..index];
                                    last_index = index + 1;
                                    if line.starts_with("{") {
                                        if line != "{}" {
                                            let event = serde_json::from_str::<Event>(line);

                                            match event {
                                                Ok(Event::Game(GameEvent::LoadingSaveDone)) => {
                                                    loading = false;
                                                }
                                                Ok(ev @ Event::Game(GameEvent::LoadingSave(_))) => {
                                                    loading = true;
                                                    return Some((
                                                        ev,
                                                        LinkFileScannerState::Running(
                                                            RunningState {
                                                                content,
                                                                file,
                                                                index,
                                                                loading,
                                                                starting,
                                                            },
                                                        ),
                                                    ));
                                                }
                                                Ok(ev @ Event::Sla(_))
                                                | Ok(
                                                    ev @ Event::DD(DDEvent::EquipmentChanged(_)),
                                                ) => {
                                                    info!(?ev, "Handling Event");

                                                    return Some((
                                                        ev,
                                                        LinkFileScannerState::Running(
                                                            RunningState {
                                                                content,
                                                                file,
                                                                index,
                                                                loading,
                                                                starting,
                                                            },
                                                        ),
                                                    ));
                                                }
                                                Ok(ev) if loading | starting => {
                                                    debug!(?ev, "Skipping Event");
                                                }
                                                Ok(ev) => {
                                                    info!(?ev, "Handling Event");

                                                    return Some((
                                                        ev,
                                                        LinkFileScannerState::Running(
                                                            RunningState {
                                                                content,
                                                                file,
                                                                index,
                                                                loading,
                                                                starting,
                                                            },
                                                        ),
                                                    ));
                                                }
                                                Err(e) => {
                                                    error!(?e, ?line, "Could not Parse Event")
                                                }
                                            }
                                        }
                                    } else {
                                        info!("{}", line);
                                    }
                                }
                            }
                            starting = false;

                            tokio::time::sleep(Duration::from_millis(100)).await;

                            content.clear();

                            file.read_to_string(&mut content).await.ok()?;

                            content = content
                                .replace("':TRUE", "':true")
                                .replace("':False", "':false")
                                .replace("'", "\"");

                            last_index = 0;
                        }
                    }
                }
            },
        ))
    }
}

pub async fn run(
    link_file_path: String,
    sender: Arc<tokio::sync::broadcast::Sender<Event>>,
    sender2: Arc<tokio::sync::mpsc::Sender<crate::device::LogicMessage>>,
) -> anyhow::Result<()> {
    let mut file = tokio::fs::File::open(link_file_path).await?;

    let mut content = String::new();

    let mut loading = false;
    let mut old_events = true;

    loop {
        let bytes = file.read_to_string(&mut content).await?;
        if bytes != 0 {
            let content = content
                .replace("':TRUE", "':true")
                .replace("':False", "':false")
                .replace("'", "\"");

            for line in content.lines() {
                if line.starts_with("{") {
                    if line != "{}" {
                        let event = serde_json::from_str::<Event>(line);

                        match event {
                            Ok(Event::Game(GameEvent::LoadingSaveDone)) => {
                                loading = false;
                            }
                            Ok(ev @ Event::Game(GameEvent::LoadingSave(_))) => {
                                loading = true;
                                if let Some(message) = LogicMessage::maybe_from(ev.clone()) {
                                    sender2.send(message).await?;
                                }
                                sender.send(ev)?;
                            }
                            Ok(ev @ Event::Sla(_))
                            | Ok(ev @ Event::DD(DDEvent::EquipmentChanged(_))) => {
                                info!(?ev, "Handling Event");

                                if let Some(message) = LogicMessage::maybe_from(ev.clone()) {
                                    sender2.send(message).await?;
                                }
                                sender.send(ev)?;
                            }
                            Ok(ev) if loading | old_events => {
                                debug!(?ev, "Skipping Event");
                            }
                            Ok(ev) => {
                                info!(?ev, "Handling Event");

                                if let Some(message) = LogicMessage::maybe_from(ev.clone()) {
                                    sender2.send(message).await?;
                                }
                                sender.send(ev)?;
                            }
                            Err(e) => error!(?e, ?line, "Could not Parse Event"),
                        }
                    }
                } else {
                    info!("{}", line);
                }
            }
        }

        old_events = false;

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
