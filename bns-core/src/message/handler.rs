use crate::message::{
    AlreadyConnected, ConnectNode, ConnectedNode, FindSuccessor, FoundSuccessor, Message,
    MessagePayload, MessageRelay, NotifiedPredecessor, NotifyPredecessor, RelayProtocol,
};
use crate::routing::{Chord, ChordAction, RemoteAction};
use crate::swarm::Swarm;
use anyhow::anyhow;
use anyhow::Result;
use futures::lock::Mutex;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use std::collections::VecDeque;
use std::sync::Arc;
use web3::types::Address;

pub struct MessageHandler {
    routing: Arc<Mutex<Chord>>,
    swarm: Arc<Mutex<Swarm>>,
}

impl MessageHandler {
    pub fn new(routing: Arc<Mutex<Chord>>, swarm: Arc<Mutex<Swarm>>) -> Self {
        Self { routing, swarm }
    }

    pub async fn send_message(&self, address: &Address, message: Message) -> Result<()> {
        // TODO: diff ttl for each message?
        let swarm = self.swarm.lock().await;
        let payload = MessagePayload::new(message, &swarm.key, None)?;
        swarm.send_message(address, payload).await
    }

    pub async fn handle_message(&self, message: &Message) -> Result<()> {
        match message {
            Message::FindSuccessor(relay, msg) => self.handle_find_successor(relay, msg).await,
            Message::FoundSuccessor(relay, msg) => self.handle_found_successor(relay, msg).await,
            Message::NotifyPredecessor(relay, msg) => {
                self.handle_notify_predecessor(relay, msg).await
            }
            Message::NotifiedPredecessor(relay, msg) => {
                self.handle_notified_predecessor(relay, msg).await
            }
            _ => Err(anyhow!("Unsupported message type")),
        }
    }

    async fn handle_connect_node(&self, relay: &MessageRelay, msg: &ConnectNode) -> Result<()> {
        // TODO: How to verify necessity based on Chord to decrease connections but make sure availablitity.
        Ok(())
    }

    async fn handle_already_connected(
        &self,
        relay: &MessageRelay,
        msg: &AlreadyConnected,
    ) -> Result<()> {
        Ok(())
    }

    async fn handle_connected_node(&self, relay: &MessageRelay, msg: &ConnectedNode) -> Result<()> {
        Ok(())
    }

    async fn handle_notify_predecessor(
        &self,
        relay: &MessageRelay,
        msg: &NotifyPredecessor,
    ) -> Result<()> {
        let mut chord = self.routing.lock().await;
        chord.notify(msg.predecessor);
        let prev = relay.to_path[relay.to_path.len() - 1];
        let report = MessageRelay::new(
            relay.tx_id.clone(),
            relay.message_id.clone(),
            relay.to_path.clone(),
            relay.from_path.clone(),
            RelayProtocol::REPORT,
        );
        let message = Message::NotifiedPredecessor(
            report,
            NotifiedPredecessor {
                predecessor: chord.predecessor.clone().unwrap(),
            },
        );
        self.send_message(&(prev.into()), message).await
    }

    async fn handle_notified_predecessor(
        &self,
        relay: &MessageRelay,
        msg: &NotifiedPredecessor,
    ) -> Result<()> {
        let mut chord = self.routing.lock().await;
        assert_eq!(relay.protocol, RelayProtocol::REPORT);
        assert_eq!(relay.to_path[relay.to_path.len() - 1], chord.id);
        // if successor: predecessor is between (id, successor]
        // then update local successor
        chord.successor = msg.predecessor;
        Ok(())
    }

    async fn handle_find_successor(&self, relay: &MessageRelay, msg: &FindSuccessor) -> Result<()> {
        let chord = self.routing.lock().await;
        match chord.find_successor(msg.id) {
            Ok(action) => match action {
                ChordAction::Some(id) => {
                    let prev = relay.find_prev();
                    let report = MessageRelay::get_report_relay(
                        relay,
                        relay.tx_id.clone(),
                        relay.message_id.clone(),
                    );
                    let message = Message::FoundSuccessor(
                        report,
                        FoundSuccessor {
                            successor: id,
                            for_fix: msg.for_fix,
                        },
                    );
                    self.send_message(&prev.unwrap().into(), message).await
                }
                ChordAction::RemoteAction((next, RemoteAction::FindSuccessor(id))) => {
                    let send = MessageRelay::get_send_relay(
                        relay,
                        relay.tx_id.clone(),
                        relay.message_id.clone(),
                        chord.id,
                        next,
                    );
                    let message = Message::FindSuccessor(
                        send,
                        FindSuccessor {
                            id,
                            for_fix: msg.for_fix,
                        },
                    );
                    self.send_message(&next.into(), message).await
                }
                _ => panic!(""),
            },
            Err(e) => panic!("{:?}", e),
        }
    }

    async fn handle_found_successor(
        &self,
        relay: &MessageRelay,
        msg: &FoundSuccessor,
    ) -> Result<()> {
        let mut chord = self.routing.lock().await;
        let current = chord.id;
        assert_eq!(relay.protocol, RelayProtocol::REPORT);
        let mut relay = relay.clone();
        assert_eq!(Some(current), relay.remove_to_path());
        relay.remove_from_path();
        if relay.to_path.len() > 0 {
            let prev = relay.find_prev();
            let report = MessageRelay::get_report_relay(
                &relay,
                relay.tx_id.clone(),
                relay.message_id.clone(),
            );
            let message = Message::FoundSuccessor(report, msg.clone());
            self.send_message(&prev.unwrap().into(), message).await
        } else {
            if msg.for_fix == true {
                let fix_finger_index = chord.fix_finger_index;
                chord.finger[fix_finger_index as usize] = Some(msg.successor);
            } else {
                chord.successor = msg.successor;
            }
            Ok(())
        }
    }

    pub async fn listen(&self) {
        let swarm = self.swarm.lock().await;
        let payloads = swarm.iter_messages();

        pin_mut!(payloads);

        while let Some(payload) = payloads.next().await {
            if payload.is_expired() || !payload.verify() {
                log::error!("Cannot verify msg or it's expired: {:?}", payload);
                continue;
            }

            if let Err(e) = self.handle_message(&payload.data).await {
                log::error!("Error in handle_message: {}", e);
                continue;
            }
        }
    }

    pub async fn stabilize(&self) {
        loop {
            let mut chord = self.routing.lock().await;
            let (current, successor) = (chord.id, chord.successor);
            let tx_id = String::from("");
            let message_id = String::from("");
            let mut to_path = VecDeque::new();
            let mut from_path = VecDeque::new();
            to_path.push_back(successor);
            from_path.push_back(current);
            let relay =
                MessageRelay::new(tx_id, message_id, to_path, from_path, RelayProtocol::SEND);
            let message = Message::NotifyPredecessor(
                relay,
                NotifyPredecessor {
                    predecessor: current,
                },
            );
            self.send_message(&current.into(), message).await.unwrap();

            // fix fingers
            match chord.fix_fingers() {
                Ok(action) => match action {
                    ChordAction::None => {
                        log::info!("")
                    }
                    ChordAction::RemoteAction((
                        next,
                        RemoteAction::FindSuccessorForFix(current),
                    )) => {
                        let tx_id = String::from("");
                        let message_id = String::from("");
                        let relay = MessageRelay::new2(
                            tx_id,
                            message_id,
                            next,
                            current,
                            RelayProtocol::SEND,
                        );
                        let message = Message::FindSuccessor(
                            relay,
                            FindSuccessor {
                                id: current,
                                for_fix: true,
                            },
                        );
                        self.send_message(&next.into(), message).await.unwrap();
                    }
                    _ => {
                        log::error!("Invalid Chord Action");
                    }
                },
                Err(e) => log::error!("{:?}", e),
            }
        }
    }
}
