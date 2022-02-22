use anyhow::{anyhow, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use web3::types::Address;

const KEY_EXPIRATION: u64 = 3600;
const ROUTING_TABLE_SIZE: usize = 20;
const REPLICATION_PARAM: usize = 20;
const BUCKET_REFRESH_INTERVAL: u64 = 3600;

// web3::types::Address;
// Address = H160(pub [u8; 20])
//
#[derive(
    Debug, Ord, PartialOrd, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Default, Copy,
)]
pub struct Key(pub [u8; 20]);

impl From<Address> for Key {
    fn from(addr: Address) -> Self {
        Key(addr.0)
    }
}

impl Key {
    pub fn new(data: [u8; 20]) -> Self {
        Key(data)
    }

    pub(super) fn xor(&self, addr: &Key) -> Key {
        let mut ret = [0; 20];
        for (i, byte) in ret.iter_mut().enumerate() {
            *byte = self.0[i] ^ addr.0[i];
        }
        Key(ret)
    }

    pub(super) fn leading_zeros(&self) -> usize {
        let mut ret = 0;
        for i in 0..20 {
            if self.0[i] == 0 {
                ret += 8
            } else {
                return ret + self.0[i].leading_zeros() as usize;
            }
        }
        ret
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Debug)]
pub struct Candidate {
    /// The sessionDescription of candidate
    pub sdp: String,
    /// The public address of the candidiate.
    pub key: Key,
}

/// A struct that contains a `Candidate` and a distance.
#[derive(Eq, Clone, Debug)]
pub struct CandidateDistance(pub Candidate, pub Key);

impl PartialEq for CandidateDistance {
    fn eq(&self, other: &CandidateDistance) -> bool {
        self.0.eq(&other.0)
    }
}

impl PartialOrd for CandidateDistance {
    fn partial_cmp(&self, other: &CandidateDistance) -> Option<Ordering> {
        Some(other.1.cmp(&self.1))
    }
}

impl Ord for CandidateDistance {
    fn cmp(&self, other: &CandidateDistance) -> Ordering {
        other.1.cmp(&self.1)
    }
}

#[derive(Clone, Debug)]
struct RouteBucket {
    candidates: Vec<Candidate>,
    updated_at: SystemTime,
}

impl RouteBucket {
    fn new() -> Self {
        RouteBucket {
            candidates: Vec::new(),
            updated_at: SystemTime::now(),
        }
    }

    fn update_candidate(&mut self, candidate: Candidate) {
        self.updated_at = SystemTime::now();
        if let Some(index) = self.candidates.iter().position(|c| *c == candidate) {
            self.candidates.remove(index);
        }
        self.candidates.push(candidate);
        if self.candidates.len() > REPLICATION_PARAM {
            self.candidates.remove(0);
        }
    }

    fn contains(&self, candidate: &Candidate) -> bool {
        self.candidates.iter().any(|c| c == candidate)
    }

    fn split(&mut self, key: &Key, index: usize) -> RouteBucket {
        let (bucket0, bucket1) = self
            .candidates
            .drain(..)
            .partition(|c| c.key.xor(key).leading_zeros() == index);
        std::mem::replace(&mut self.candidates, bucket0);
        RouteBucket {
            candidates: bucket1,
            updated_at: self.updated_at,
        }
    }

    fn get_candidates(&self) -> &[Candidate] {
        self.candidates.as_slice()
    }

    fn remove_lrs(&mut self) -> Option<Candidate> {
        if self.candidates.len() == 0 {
            None
        } else {
            Some(self.candidates.remove(0))
        }
    }

    /// Removes `candidate` from the bucket.
    pub fn remove_candidate(&mut self, candidate: &Candidate) -> Option<Candidate> {
        if let Some(index) = self.candidates.iter().position(|c| c == candidate) {
            Some(self.candidates.remove(index))
        } else {
            None
        }
    }

    pub fn is_stale(&self) -> bool {
        let diff_t = SystemTime::now()
            .duration_since(self.updated_at)
            .expect("Clock may have gone backwards");
        diff_t > Duration::from_secs(BUCKET_REFRESH_INTERVAL)
    }
}

#[derive(Clone, Debug)]
pub struct RouteTable {
    buckets: Arc<Mutex<Vec<RouteBucket>>>,
    candidate: Arc<Mutex<Option<Arc<Candidate>>>>,
}

impl RouteTable {
    pub fn new() -> Self {
        let mut buckets = Vec::new();
        buckets.push(RouteBucket::new());
        RouteTable {
            buckets: Arc::new(Mutex::new(buckets)),
            candidate: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn update_candidate(&mut self, other: Candidate) -> bool {
        match self.candidate.lock().await.clone() {
            Some(candidate) => {
                let distance = candidate.key.xor(&other.key).leading_zeros();
                let mut target = std::cmp::min(distance, self.buckets.lock().await.len() - 1);
                if self.buckets.lock().await[target].contains(&other) {
                    let buckets = Arc::clone(&self.buckets);
                    buckets.lock().await[target].update_candidate(other);
                    return true;
                }
                loop {
                    if self.buckets.lock().await[target].candidates.len() < REPLICATION_PARAM {
                        self.buckets.lock().await[target].update_candidate(other);
                        return true;
                    }
                    let bucket_length = self.buckets.lock().await.len();
                    if !target == bucket_length - 1 || bucket_length == ROUTING_TABLE_SIZE {
                        return false;
                    }
                    let bucket = self.buckets.lock().await[target].split(&other.key, target);
                    self.buckets.lock().await.push(bucket);
                    target = std::cmp::min(distance, self.buckets.lock().await.len() - 1);
                }
            }
            None => false,
        }
    }

    /// Returns the closest `count` candidates to `address`.
    pub async fn get_closest_candidate(&self, key: &Key, count: usize) -> Vec<Candidate> {
        match self.candidate.lock().await.clone() {
            Some(candidate) => {
                let index = std::cmp::min(
                    candidate.key.xor(key).leading_zeros(),
                    self.buckets.lock().await.len() - 1,
                );
                let mut ret = vec![];
                ret.extend_from_slice(self.buckets.lock().await[index].get_candidates());
                if ret.len() < count {
                    for i in (0..index).rev() {
                        ret.extend_from_slice(self.buckets.lock().await[i].get_candidates());
                        if ret.len() >= count {
                            break;
                        }
                    }
                }
                ret.sort_by_key(|c| c.key.xor(key));
                ret.truncate(count);
                ret
            }
            None => vec![],
        }
    }

    pub async fn remove_lrs(&mut self, key: &Key) -> Option<Candidate> {
        match self.candidate.lock().await.clone() {
            Some(candidate) => {
                let index = std::cmp::min(
                    candidate.key.xor(key).leading_zeros(),
                    self.buckets.lock().await.len() - 1,
                );
                self.buckets.lock().await[index].remove_lrs()
            }
            None => None,
        }
    }

    pub async fn remove_candidate(&mut self, other: &Candidate) -> Result<()> {
        match self.candidate.lock().await.clone() {
            Some(candidate) => {
                let index = std::cmp::min(
                    candidate.key.xor(&other.key).leading_zeros(),
                    self.buckets.lock().await.len() - 1,
                );
                self.buckets.lock().await[index].remove_candidate(other);
                Ok(())
            }
            _ => Err(anyhow!("None Candidate")),
        }
    }

    pub async fn get_stale_indexes(&self) -> Vec<usize> {
        let mut ret = vec![];
        for (i, bucket) in self.buckets.lock().await.iter().enumerate() {
            if bucket.is_stale() {
                ret.push(i);
            }
        }
        ret
    }
}
