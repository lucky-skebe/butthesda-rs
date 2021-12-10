mod contracts;

use std::time::Duration;

pub use contracts::*;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info};

pub async fn run(
    link_file_path: &str,
    sender: tokio::sync::mpsc::Sender<Event>,
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
                                sender.send(ev).await?;
                            }
                            Ok(ev @ Event::Sla(_))
                            | Ok(ev @ Event::DD(DDEvent::EquipmentChanged(_))) => {
                                info!(?ev, "Handling Event");

                                sender.send(ev).await?;
                            }
                            Ok(ev) if loading | old_events => {
                                debug!(?ev, "Skipping Event");
                            }
                            Ok(ev) => {
                                info!(?ev, "Handling Event");

                                sender.send(ev).await?;
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
