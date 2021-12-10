use std::{
    io::{Read},
    time::Duration,
};

use tracing::{error, info, Level};
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

    // file.seek(SeekFrom::End(0)).unwrap();

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
                            Ok(ev) => info!(?ev, ?line, "Parsed Event"),
                            Err(e) => error!(?e, ?line, "Could not Parse Event"),
                        }
                    }
                } else {
                    info!("{}", line);
                }
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
