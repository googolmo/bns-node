use crate::kadelima::candidates::{Address, Candidate};
use crate::kadelima::constants::{BUCKET_REFRESH_INTERVAL, REPLICATION_PARAM, ROUTING_TABLE_SIZE};

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::{cmp, mem};

#[derive(Clone, Debug)]
struct KBucket {
    candidates: Vec<Candidate>,
    updated_at: SystemTime,
}

impl KBucket {
    fn new() -> Self {
        KBucket {
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

    fn split(&mut self, address: &Address, index: usize) -> KBucket {
        let (bucket0, bucket1) = self
            .candidates
            .drain(..)
            .partition(|c| c.id.xor(address).leading_zeros() == index);
        std::mem::replace(&mut self.candidates, bucket0);
        KBucket {
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
pub struct KTable {
    buckets: Vec<KBucket>,
    candidate: Arc<Candidate>,
}

impl KTable {
    pub fn new(candidate: Arc<Candidate>) -> Self {
        let mut buckets = Vec::new();
        buckets.push(KBucket::new());
        KTable { buckets, candidate }
    }

    pub fn update_candidate(&mut self, candidate: Candidate) -> bool {
        let distance = self.candidate.id.xor(&candidate.id).leading_zeros();
        let mut target = cmp::min(distance, self.buckets.len() - 1);

        if self.buckets[target].contains(&candidate) {
            self.buckets[target].update_candidate(candidate);
            return true;
        }

        loop {
            // bucket is not full
            if self.buckets[target].candidates.len() < REPLICATION_PARAM {
                self.buckets[target].update_candidate(candidate);
                return true;
            }

            let is_last = target == self.buckets.len() - 1;
            let is_full = self.buckets.len() == ROUTING_TABLE_SIZE;

            // bucket cannot be split
            if !is_last || is_full {
                return false;
            }

            // split bucket
            let bucket = self.buckets[target].split(&self.candidate.id, target);
            self.buckets.push(bucket);

            target = cmp::min(distance, self.buckets.len() - 1);
        }
    }

    /// Returns the closest `count` candidates to `address`.
    pub fn get_closest_candidate(&self, address: &Address, count: usize) -> Vec<Candidate> {
        let index = cmp::min(
            self.candidate.id.xor(address).leading_zeros(),
            self.buckets.len() - 1,
        );
        let mut ret = Vec::new();
        ret.extend_from_slice(self.buckets[index].get_candidates());
        if ret.len() < count {
            for i in (index + 1)..self.buckets.len() {
                ret.extend_from_slice(self.buckets[i].get_candidates());
            }
        }
        if ret.len() < count {
            for i in (0..index).rev() {
                ret.extend_from_slice(self.buckets[i].get_candidates());
                if ret.len() >= count {
                    break;
                }
            }
        }

        ret.sort_by_key(|c| c.id.xor(address));
        ret.truncate(count);
        ret
    }

    pub fn remove_lrs(&mut self, address: &Address) -> Option<Candidate> {
        let index = cmp::min(
            self.candidate.id.xor(address).leading_zeros(),
            self.buckets.len() - 1,
        );
        self.buckets[index].remove_lrs()
    }

    pub fn remove_candidate(&mut self, candidate: &Candidate) {
        let index = cmp::min(
            self.candidate.id.xor(&candidate.id).leading_zeros(),
            self.buckets.len() - 1,
        );
        self.buckets[index].remove_candidate(candidate);
    }

    pub fn get_stale_indexes(&self) -> Vec<usize> {
        let mut ret = Vec::new();
        for (i, bucket) in self.buckets.iter().enumerate() {
            if bucket.is_stale() {
                ret.push(i);
            }
        }
        ret
    }
}
