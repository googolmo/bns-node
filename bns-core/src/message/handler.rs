use crate::message::*;
use crate::routing::Chord;
use crate::swarm::Swarm;
use anyhow::anyhow;
use anyhow::Result;
use futures::lock::Mutex;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use std::sync::Arc;
use web3::types::Address;

pub struct MessageHandler {
    routing: Arc<Mutex<Chord>>,
    swarm: Arc<Mutex<Swarm>>,
}

impl MessageHandler {
    pub fn new(routing: Arc<Mutex<Chord>>, swarm: Arc<Swarm>) -> Self {
        Self { routing, swarm }
    }

    pub async fn send_message(&self, address: &Address, message: Message) -> Result<()> {
        // TODO: diff ttl for each message?
        let payload = MessagePayload::new(message, &self.swarm.key, None)?;
        self.swarm.send_message(address, payload).await
    }

    pub async fn handle_message(&self, message: &Message, prev: &Did) -> Result<()> {
        let current = &self.routing.lock().await.id;

        match message {
            Message::ConnectNode(msrp, msg) => {
                self.handle_connect_node(msrp.record(prev), msg).await
            }
            Message::AlreadyConnected(msrp, msg) => {
                self.handle_already_connected(msrp.record(prev, current)?, msg)
                    .await
            }
            Message::ConnectNodeResponse(msrp, msg) => {
                self.handle_connect_node_response(msrp.record(prev, current)?, msg)
                    .await
            }
            Message::NotifyPredecessor(msrp, msg) => {
                self.handle_notify_predecessor(msrp.record(prev), msg).await
            }
            Message::NotifyPredecessorResponse(msrp, msg) => {
                self.handle_notify_predecessor_response(msrp.record(prev, current)?, msg)
                    .await
            }
            Message::FindSuccessor(msrp, msg) => {
                self.handle_find_successor(msrp.record(prev), msg).await
            }
            Message::FindSuccessorResponse(msrp, msg) => {
                self.handle_find_successor_response(msrp.record(prev, current)?, msg)
                    .await
            }
            Message::FindSuccessorAndAddToFinger(msrp, msg) => {
                self.handle_find_successor_add_finger(msrp.record(prev), msg)
                    .await
            }
            Message::FindSuccessorAndAddToFingerResponse(msrp, msg) => {
                self.handle_find_success_add_finger_response(msrp.record(prev, current)?, msg)
                    .await
            }
            _ => Err(anyhow!("Unsupported message type")),
        }
    }

    async fn handle_connect_node(&self, _msrp: MsrpSend, _msg: &ConnectNode) -> Result<()> {
        // TODO: How to verify necessity based on Chord to decrease connections but make sure availablitity.
        Ok(())
    }

    async fn handle_already_connected(
        &self,
        _msrp: MsrpReport,
        _msg: &AlreadyConnected,
    ) -> Result<()> {
        Ok(())
    }

    async fn handle_connect_node_response(
        &self,
        _msrp: MsrpReport,
        _msg: &ConnectNodeResponse,
    ) -> Result<()> {
        Ok(())
    }

    async fn handle_notify_predecessor(
        &self,
        msrp: MsrpSend,
        msg: &NotifyPredecessor,
    ) -> Result<()> {
        let mut chord = self.routing.lock().await;
        chord.notify(msg.predecessor);
        let mut report: MsrpReport = msrp.into();
        let prev = report.to_path.pop();
        let response = NotifyPredecessorResponse {
            predecessor: msg.predecessor.clone(),
        };
        let message = Message::NotifyPredecessorResponse(report, response);
        self.send_message(&(prev.unwrap().into()), message);
        Ok(())
    }

    async fn handle_notify_predecessor_response(
        &self,
        msrp: MsrpReport,
        msg: &NotifyPredecessorResponse,
    ) -> Result<()> {
        let remote = msrp.from_path[msrp.from_path.len() - 1];
        log::info!("Remote {:?} find predecessor and update", remote);
        let mut chord = self.routing.lock().await;
        // if successor: predecessor is between (id, successor]
        // then update local successor
        chord.successor = msg.predecessor;
        Ok(())
    }

    async fn handle_find_successor(&self, msrp: MsrpSend, msg: &FindSuccessor) -> Result<()> {
        let mut chord = self.routing.lock().await;
        match chord.find_successor(msg.id) {
            Ok()
        }
        Ok(())
    }

    async fn handle_find_successor_response(
        &self,
        msrp: MsrpReport,
        msg: &FindSuccessorResponse,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn listen(&self) {
        let payloads = self.swarm.clone().iter_messages();

        pin_mut!(payloads);

        while let Some(payload) = payloads.next().await {
            if payload.is_expired() || !payload.verify() {
                log::error!("Cannot verify msg or it's expired: {:?}", payload);
                continue;
            }

            if let Err(e) = self
                .handle_message(&payload.data, &payload.addr.into())
                .await
            {
                log::error!("Error in handle_message: {}", e);
                continue;
            }
        }
    }
}
