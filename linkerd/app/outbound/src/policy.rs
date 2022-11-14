use linkerd_app_core::{cache, transport::OrigDstAddr};
pub use linkerd_client_policy::*;
pub mod api;
pub mod store;

pub type Receiver = tokio::sync::watch::Receiver<ClientPolicy>;

#[derive(Clone, Debug)]
pub struct Policy {
    pub dst: OrigDstAddr,
    pub policy: cache::Cached<Receiver>,
}

pub trait GetPolicy {
    fn get_policy(&self, addr: OrigDstAddr) -> Policy;
}