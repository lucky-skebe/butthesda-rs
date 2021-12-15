use std::{fmt::Display, hash::Hash};

use buttplug::client::ButtplugClient;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all ="camelCase")]
pub enum DeviceInteraction {
    Vibrate,
    Rotate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeviceFeature {
    pub interaction: DeviceInteraction,
    pub index: u32,
}

impl Display for DeviceFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} - {}", self.interaction, self.index)
    }
}

#[derive(Debug, Clone)]
pub enum ButtplugConnection {
    InProcess,
}

#[derive(Debug, Clone)]
pub enum ButtplugOutMessage {
    ConnectTo(ButtplugConnection),
    StartScan,
    StopScan,
    Disconnect,
}

async fn handle_out_message(message: crate::Message, client: &ButtplugClient) {
    match message {
        crate::Message::ButtplugOut(message) => {
            if let Err(e) = match message {
                ButtplugOutMessage::ConnectTo(target) => match target {
                    ButtplugConnection::InProcess => client.connect_in_process(None).await,
                },
                ButtplugOutMessage::StartScan => client.start_scanning().await,
                ButtplugOutMessage::StopScan => client.stop_scanning().await,
                ButtplugOutMessage::Disconnect => client.disconnect().await,
            } {
                error!("{}", e);
                // maybe return some errors so we die instead of continue "silently"
            }
        }

        _ => {}
    }
}

pub async fn run(message_bus: crate::MessageBus) -> anyhow::Result<()> {
    info!("Buttplug integration starting.");
    
    let mut in_box = message_bus.subscribe();
    let client = buttplug::client::ButtplugClient::new("Butthesda-rs");

    let mut events = client.event_stream();

    info!("Buttplug integration started.");

    while tokio::select! {
        msg = in_box.recv() => { handle_out_message(msg?, &client).await; true },
        event = events.next() => {
                match event {
                    Some(event) => {
                        message_bus.send(event.into())?;
                        true
                    }
                    None => false
                }
            }
    } {}

    
    info!("Buttplug integration shutting down.");

    Ok(())
}
