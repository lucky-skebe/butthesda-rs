mod contracts;

use std::path::PathBuf;
use std::time::Duration;

pub use contracts::*;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub enum InMessage {
    FileEvent(Event),
}

#[derive(Debug, Clone)]
pub enum OutMessage {
    StartScan(PathBuf),
}

pub async fn run(
    message_bus: tokio::sync::broadcast::Sender<crate::Message>,
) -> anyhow::Result<()> {
    let mut in_box = message_bus.subscribe();

    let mut try_path: Option<PathBuf> = None;
    loop {
        loop {
            let result = tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(100)) => None,
                r = in_box.recv() => Some(r)
            };

            match result {
                Some(Ok(crate::Message::LinkFileOut(OutMessage::StartScan(new_path)))) => {
                    try_path = Some(new_path);
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => return Err(err.into()),
                None => break,
            }
        }

        if let Some(path) = &try_path {
            let mut p = path.clone();
            p.push("Funscripts/link.txt");
            let file = tokio::fs::File::open(&p).await;
            try_path = None;
            let mut file = match file {
                Ok(o) => o,
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
            };

            let mut loading = false;
            let mut old_events = true;

            loop {
                let mut content = String::new();
                let bytes = match file.read_to_string(&mut content).await {
                    Ok(o) => o,
                    Err(e) => {
                        error!("{}", e);
                        break;
                    }
                };
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
                                        message_bus.send(InMessage::FileEvent(ev).into())?;
                                    }
                                    Ok(ev @ Event::Sla(_))
                                    | Ok(ev @ Event::DD(DDEvent::EquipmentChanged(_))) => {
                                        info!(?ev, "Handling Event");

                                        message_bus.send(InMessage::FileEvent(ev).into())?;
                                    }
                                    Ok(_ev) if loading | old_events => {
                                        // debug!(?ev, "Skipping Event");
                                    }
                                    Ok(ev) => {
                                        info!(?ev, "Handling Event");

                                        message_bus.send(InMessage::FileEvent(ev).into())?;
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

                loop {
                    let result = tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(100)) => None,
                        r = in_box.recv() => Some(r)
                    };

                    match result {
                        Some(Ok(crate::Message::LinkFileOut(OutMessage::StartScan(new_path)))) => {
                            try_path = Some(new_path);
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => return Err(err.into()),
                        None => break,
                    }
                }
            }
        }
    }
}
