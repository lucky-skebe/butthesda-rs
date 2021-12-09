use memory::Process;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod memory;

fn main() {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let mut process = Process::open(memory::SKYRIM_SE).unwrap().unwrap();

    dbg!(process.inject());

    println!("Pid: {}", process.pid);
}
