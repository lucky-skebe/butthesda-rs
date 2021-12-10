use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod link_file;
mod memory;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let (sender, receiver) = tokio::sync::mpsc::channel(100);

    let res = tokio::try_join! {
        {
            let sender = sender.clone();
            link_file::run(
            "E:\\ModOrganizer2\\SSE\\mods\\Butthesda\\FunScripts\\link.txt",
            sender,
        )}
    };

    if let Err(e) = res {
        Err(e)
    } else {
        Ok(())
    }
}
