use anyhow::Result;
use async_trait::async_trait;

use crate::types::channel::Channel;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex as SyncMutex;

#[cfg_attr(feature = "wasm", async_trait(?Send))]
#[cfg_attr(not(feature = "wasm"), async_trait)]
pub trait IceTransport<Ch: Channel> {
    type Connection;
    type Candidate;
    type Sdp;
    type Channel;
    type ConnectionState;
    type Msg;

    fn new(signaler: Arc<SyncMutex<Ch>>) -> Self;
    fn signaler(&self) -> Arc<SyncMutex<Ch>>;
    async fn start(&mut self, stun_server: String) -> Result<()>;
    async fn get_peer_connection(&self) -> Option<Arc<Self::Connection>>;
    async fn get_pending_candidates(&self) -> Vec<Self::Candidate>;
    async fn get_answer(&self) -> Result<Self::Sdp>;
    fn get_offer(&self) -> Result<Self::Sdp>;
    fn get_offer_str(&self) -> Result<String>;
    async fn get_data_channel(&self) -> Option<Arc<Self::Channel>>;

    async fn set_local_description<T>(&self, desc: T) -> Result<()>
    where
        T: Into<Self::Sdp> + Send;

    async fn set_remote_description<T>(&self, desc: T) -> Result<()>
    where
        T: Into<Self::Sdp> + Send;
    async fn on_ice_candidate(
        &self,
        f: Box<
            dyn FnMut(Option<Self::Candidate>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
                + Send
                + Sync,
        >,
    ) -> Result<()>;
    async fn on_peer_connection_state_change(
        &self,
        f: Box<
            dyn FnMut(Self::ConnectionState) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
                + Send
                + Sync,
        >,
    ) -> Result<()>;
    async fn on_data_channel(
        &self,
        f: Box<
            dyn FnMut(Arc<Self::Channel>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
                + Send
                + Sync,
        >,
    ) -> Result<()>;

    async fn on_message(
        &self,
        f: Box<
            dyn FnMut(Self::Msg) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
                + Send
                + Sync,
        >,
    ) -> Result<()>;
}

#[cfg_attr(feature = "wasm", async_trait(?Send))]
#[cfg_attr(not(feature = "wasm"), async_trait)]
pub trait IceTransportCallback<Ch: Channel>: IceTransport<Ch> {
    async fn setup_callback(&self) -> Result<()>;
    async fn on_ice_candidate_callback(
        &self,
    ) -> Box<
        dyn FnMut(Option<Self::Candidate>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync,
    >;
    async fn on_peer_connection_state_change_callback(
        &self,
    ) -> Box<
        dyn FnMut(Self::ConnectionState) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync,
    >;
    async fn on_data_channel_callback(
        &self,
    ) -> Box<
        dyn FnMut(Arc<Self::Channel>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync,
    >;
    async fn on_message_callback(
        &self,
    ) -> Box<dyn FnMut(Self::Msg) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>;
}
