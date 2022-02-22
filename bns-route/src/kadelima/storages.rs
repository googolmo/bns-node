use crate::kadelima::candidates::Address;
use crate::kadelima::constants::KEY_EXPIRATION;
use anyhow::anyhow;
use anyhow::Result;
use dashmap::DashMap;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct SdpString(String, SystemTime);

#[derive(Default, Clone, Debug)]
pub struct Storage {
    items: DashMap<Address, SdpString>,
    collections: BTreeMap<SystemTime, HashSet<Address>>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            items: DashMap::new(),
            collections: BTreeMap::new(),
        }
    }

    fn remove_expired(&mut self) {
        let diff_t = SystemTime::now() - Duration::from_secs(KEY_EXPIRATION);
        let mut collections = self.collections.split_off(&diff_t);
        std::mem::swap(&mut self.collections, &mut collections);

        for address in collections
            .into_iter()
            .flat_map(|entry| entry.1.into_iter())
        {
            self.items.remove(&address);
        }
    }

    pub fn insert(&mut self, address: Address, value: String) {
        self.remove_expired();
        let curr_time = SystemTime::now();
        let sdp = SdpString(value, curr_time);
        if let Some(entry) = self.items.insert(address, sdp) {
            if let Some(addresses) = self.collections.get_mut(&entry.1) {
                addresses.remove(&address);
            }
        }

        self.collections
            .entry(curr_time)
            .or_insert_with(HashSet::new)
            .insert(address);
    }

    pub fn get(&mut self, address: &Address) -> Option<String> {
        self.remove_expired();
        self.items.get(address).map(|entry| entry.0.clone())
    }
}
