use std::{fmt::Display, hash::Hash};

use buttplug::{
    client::ButtplugClient,
    connector::{ButtplugRemoteClientConnector, ButtplugWebsocketClientTransport},
    core::messages::serializer::ButtplugClientJSONSerializer,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    Websocket((url::Url, bool)),
}

#[derive(Debug, Clone)]
pub enum ButtplugOutMessage {
    ConnectTo(ButtplugConnection),
    StartScan,
    StopScan,
    Disconnect,
}

async fn handle_out_message(
    message: crate::Message,
    client: &ButtplugClient,
) -> Option<crate::Message> {
    match message {
        crate::Message::ButtplugOut(message) => {
            let (e, out_message) = match message {
                ButtplugOutMessage::ConnectTo(target) => match target {
                    ButtplugConnection::InProcess => (
                        client.connect_in_process(None).await,
                        Some(crate::Message::ButtplugIn(
                            ::buttplug::client::ButtplugClientEvent::ServerConnect,
                        )),
                    ),
                    ButtplugConnection::Websocket((url, bypas)) => {
                        let connector = ButtplugRemoteClientConnector::<
                            ButtplugWebsocketClientTransport,
                            ButtplugClientJSONSerializer,
                        >::new(if url.scheme() == "ws" {
                            ButtplugWebsocketClientTransport::new_insecure_connector(url.as_str())
                        } else {
                            ButtplugWebsocketClientTransport::new_secure_connector(
                                url.as_str(),
                                bypas,
                            )
                        });
                        (
                            client.connect(connector).await,
                            Some(crate::Message::ButtplugIn(
                                ::buttplug::client::ButtplugClientEvent::ServerConnect,
                            )),
                        )
                    }
                },
                ButtplugOutMessage::StartScan => (client.start_scanning().await, None),
                ButtplugOutMessage::StopScan => (client.stop_scanning().await, None),
                ButtplugOutMessage::Disconnect => (
                    client.disconnect().await,
                    Some(crate::Message::ButtplugIn(
                        ::buttplug::client::ButtplugClientEvent::ServerDisconnect,
                    )),
                ),
            };

            if let Err(e) = e {
                error!("{}", e);
                // maybe return some errors so we die instead of continue "silently"
                None
            } else {
                out_message
            }
        }

        _ => None,
    }
}

pub async fn run(message_bus: crate::MessageBus) -> anyhow::Result<()> {
    info!("Buttplug integration starting.");

    let mut in_box = message_bus.subscribe();
    let client = buttplug::client::ButtplugClient::new("Butthesda-rs");

    let mut events = client.event_stream();

    info!("Buttplug integration started.");

    while tokio::select! {
        msg = in_box.recv() => {
            if let Some(event) = handle_out_message(msg?, &client).await{
                dbg!(&event);
            message_bus.send(event.into())?;

        }
        true },
        event = events.next() => {
                match event {
                    Some(event) => {
                        dbg!(&event);
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
