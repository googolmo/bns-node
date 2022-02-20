use crate::signing::SigMsg;
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::hash::Hash;
use web3::types::Address;

#[derive(Debug, Clone)]
struct BNSNode {
    key: Address,
    msg: (String, Vec<u8>),
}

impl BNSNode {
    fn new(key: Address, msg: (String, Vec<u8>)) -> BNSNode {
        BNSNode { key, msg }
    }
}

impl PartialEq for BNSNode {
    fn eq(&self, other: &BNSNode) -> bool {
        self.key == other.key
    }
}

impl Eq for BNSNode {}

impl PartialOrd for BNSNode {
    fn partial_cmp(&self, other: &BNSNode) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BNSNode {
    fn cmp(&self, other: &BNSNode) -> Ordering {
        self.key.cmp(&other.key)
    }
}

#[derive(Clone, Debug)]
pub struct HashRing {
    nodes: Vec<BNSNode>,
}

impl HashRing {
    pub fn new() -> Self {
        HashRing { nodes: vec![] }
    }
}

impl HashRing {
    pub fn insert(&mut self, key: Address, msg: (String, Vec<u8>)) {
        self.nodes.push(BNSNode::new(key, msg));
        self.nodes.sort();
    }
}
