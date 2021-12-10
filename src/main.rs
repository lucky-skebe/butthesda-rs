use std::{io::Read, time::Duration};

use link_file::{DDEvent, Event, GameEvent};
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod link_file;
mod memory;

fn main() {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // let mut process = Process::open(memory::SKYRIM_SE).unwrap().unwrap();

    // dbg!(process.inject());

    // println!("Pid: {}", process.pid);

    let mut file =
        std::fs::File::open("E:\\ModOrganizer2\\SSE\\mods\\Butthesda\\FunScripts\\link.txt")
            .unwrap();
    let mut content = String::new();

    let (mut tx, rx) = std::sync::mpsc::channel();

    let mut loading = false;
    let mut old_events = true;

    loop {
        let bytes = file.read_to_string(&mut content).unwrap();
        if bytes != 0 {
            let content = content
                .replace("':TRUE", "':true")
                .replace("':False", "':false")
                .replace("'", "\"");

            for line in content.lines() {
                if line.starts_with("{") {
                    if line != "{}" {
                        let event = serde_json::from_str::<link_file::Event>(line);

                        match event {
                            Ok(Event::Game(GameEvent::LoadingSaveDone)) => {
                                loading = false;
                            }
                            Ok(ev @ Event::Game(GameEvent::LoadingSave(_))) => {
                                loading = true;
                                tx.send(ev).unwrap();
                            }
                            Ok(ev @ Event::Sla(_))
                            | Ok(ev @ Event::DD(DDEvent::EquipmentChanged(_))) => {
                                info!(?ev, "Handling Event");

                                tx.send(ev).unwrap();
                            }
                            Ok(ev) if loading | old_events => {
                                debug!(?ev, "Skipping Event");
                            }
                            Ok(ev) => {
                                info!(?ev, "Handling Event");

                                tx.send(ev).unwrap();
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

        std::thread::sleep(Duration::from_millis(100));
    }
}
