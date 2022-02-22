use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Result};

// web3::types::Address;
// Address = H160(pub [u8; 20])
//
#[derive(
    Debug, Ord, PartialOrd, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Default, Copy,
)]
pub struct Address(pub [u8; 20]);

impl Address {
    pub fn new(data: [u8; 20]) -> Self {
        Address(data)
    }

    pub(super) fn xor(&self, address: &Address) -> Address {
        let mut ret = [0; 20];
        for (i, byte) in ret.iter_mut().enumerate() {
            *byte = self.0[i] ^ address.0[i];
        }
        Address(ret)
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

#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Candidate {
    /// The sessionDescription of candidate
    pub sdp: String,
    /// The public address of the candidiate.
    pub id: Address,
}

impl Debug for Candidate {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{} - {:?}", self.sdp, self.id)
    }
}

/// A struct that contains a `Candidate` and a distance.
#[derive(Eq, Clone, Debug)]
pub struct CandidateDistance(pub Candidate, pub Address);

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
