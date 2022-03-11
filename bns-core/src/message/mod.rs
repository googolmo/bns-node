pub mod handler;

mod encoder;
pub use encoder::Encoded;

mod payload;
pub use payload::MessagePayload;

mod msrp;
pub use msrp::{MessageRelay, RelayProtocol};

use crate::routing::Did;
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FoundSuccessor {
    id: Did,
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
pub struct FindSuccessorForFix {
    pub successor: Did,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FoundSuccessorForFix {
    pub successor: Did,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Message {
    ConnectNode(MessageRelay, ConnectNode),
    AlreadyConnected(MessageRelay, AlreadyConnected),
    ConnectedNode(MessageRelay, ConnectedNode),
    FindSuccessor(MessageRelay, FindSuccessor),
    FoundSuccessor(MessageRelay, FoundSuccessor),
    FindSuccessorForFix(MessageRelay, FindSuccessorForFix),
    FoundSuccessorForFix(MessageRelay, FoundSuccessorForFix),
    NotifyPredecessor(MessageRelay, NotifyPredecessor),
    NotifiedPredecessor(MessageRelay, NotifiedPredecessor),
}
