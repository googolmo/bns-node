pub mod handler;

mod encoder;
pub use encoder::Encoded;

mod payload;
pub use payload::MessageRelay;
pub use payload::MessageRelayMethod;

use crate::dht::Did;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectNode {
    pub id: Did,
    pub handshake_info: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlreadyConnected;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectedNode {
    pub already_connected: bool,
    pub handshake_info: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FindSuccessor {
    id: Did,
    for_fix: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FoundSuccessor {
    successor: Did,
    for_fix: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotifyPredecessor {
    pub predecessor: Did,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotifiedPredecessor {
    pub predecessor: Did,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Message {
    None,
    ConnectNode(ConnectNode),
    AlreadyConnected(AlreadyConnected),
    ConnectedNode(ConnectedNode),
    FindSuccessor(FindSuccessor),
    FoundSuccessor(FoundSuccessor),
    NotifyPredecessor(NotifyPredecessor),
    NotifiedPredecessor(NotifiedPredecessor),
}
