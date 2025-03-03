use std::cmp::Eq;
use std::cmp::PartialEq;
use std::ops::Add;
use std::ops::Deref;
use std::ops::Neg;
use std::ops::Sub;
use std::str::FromStr;

use num_bigint::BigUint;
use serde::Deserialize;
use serde::Serialize;
use web3::types::H160;

/// Did is a finate Ring R(P) where P = 2^160
use crate::ecc::HashStr;
use crate::err::Error;
use crate::err::Result;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize, Hash)]
pub struct Did(H160);

// Bias Did is a special Did which set origin Did's idendity to bias
#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize, Hash)]
pub struct BiasId {
    bias: Did,
    did: Did,
}

impl BiasId {
    pub fn new(bias: &Did, id: &Did) -> BiasId {
        BiasId {
            bias: *bias,
            did: *id - *bias,
        }
    }

    pub fn to_did(bid: &BiasId) -> Did {
        bid.did + bid.bias
    }

    pub fn pos(&self) -> Did {
        self.did
    }
}

impl PartialOrd for BiasId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if other.bias != self.bias {
            let did: Did = other.into();
            let bid = BiasId::new(&self.bias, &did);
            self.did.partial_cmp(&bid.did)
        } else {
            self.did.partial_cmp(&other.did)
        }
    }
}

impl PartialEq<Did> for BiasId {
    fn eq(&self, rhs: &Did) -> bool {
        let id: Did = self.into();
        id == *rhs
    }
}

impl Ord for BiasId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if other.bias != self.bias {
            let did: Did = other.into();
            let bid = BiasId::new(&self.bias, &did);
            self.did.cmp(&bid.did)
        } else {
            self.did.cmp(&other.did)
        }
    }
}

impl From<BiasId> for Did {
    fn from(id: BiasId) -> Did {
        BiasId::to_did(&id)
    }
}

impl From<&BiasId> for Did {
    fn from(id: &BiasId) -> Did {
        BiasId::to_did(id)
    }
}

impl TryFrom<HashStr> for Did {
    type Error = Error;
    fn try_from(s: HashStr) -> Result<Self> {
        Did::from_str(&s.inner())
    }
}

impl Did {
    // Test x <- (a, b)
    pub fn in_range(&self, id: &Self, a: &Self, b: &Self) -> bool {
        // Test x > a && b > x
        *self - *id > *a - *id && *b - *id > *self - *id
    }

    // Transform Did to BiasDid
    pub fn bias(&self, id: &Self) -> BiasId {
        BiasId::new(id, self)
    }
}

pub trait SortRing {
    fn sort(&mut self, id: Did);
}

impl SortRing for Vec<Did> {
    fn sort(&mut self, id: Did) {
        self.sort_by(|a, b| {
            let (da, db) = (*a - id, *b - id);
            (da).partial_cmp(&db).unwrap()
        });
    }
}

impl Deref for Did {
    type Target = H160;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Did> for H160 {
    fn from(a: Did) -> Self {
        a.0.to_owned()
    }
}

impl From<Did> for BigUint {
    fn from(did: Did) -> BigUint {
        BigUint::from_bytes_be(did.as_bytes())
    }
}

impl From<BigUint> for Did {
    fn from(a: BigUint) -> Self {
        let ff = a % (BigUint::from(2u16).pow(160));
        let mut va: Vec<u8> = ff.to_bytes_be();
        let mut res = vec![0u8; 20 - va.len()];
        res.append(&mut va);
        assert_eq!(res.len(), 20, "{:?}", res);
        Self(H160::from_slice(&res))
    }
}

impl From<H160> for Did {
    fn from(addr: H160) -> Self {
        Self(addr)
    }
}

impl FromStr for Did {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(Self(H160::from_str(s).map_err(|_| Error::BadCHexInCache)?))
    }
}

// impl Finate Ring For Did

impl Neg for Did {
    type Output = Self;
    fn neg(self) -> Self {
        let ret = BigUint::from(2u16).pow(160) - BigUint::from(self);
        ret.into()
    }
}

impl Add for Did {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        ((BigUint::from(self) + BigUint::from(rhs)) % (BigUint::from(2u16).pow(160))).into()
    }
}

impl Sub for Did {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_did() {
        let a = Did::from_str("0x11E807fcc88dD319270493fB2e822e388Fe36ab0").unwrap();
        let b = Did::from_str("0x999999cf1046e68e36E1aA2E0E07105eDDD1f08E").unwrap();
        let c = Did::from_str("0xc0ffee254729296a45a3885639AC7E10F9d54979").unwrap();
        assert!(c > b && b > a);
    }

    #[test]
    fn test_finate_ring_neg() {
        let zero = Did::from_str("0x0000000000000000000000000000000000000000").unwrap();
        let a = Did::from_str("0x11E807fcc88dD319270493fB2e822e388Fe36ab0").unwrap();
        assert_eq!(-a + a, zero);
        assert_eq!(-(-a), a);
    }

    #[test]
    fn test_sort() {
        let a = Did::from_str("0xaaE807fcc88dD319270493fB2e822e388Fe36ab0").unwrap();
        let b = Did::from_str("0xbb9999cf1046e68e36E1aA2E0E07105eDDD1f08E").unwrap();
        let c = Did::from_str("0xccffee254729296a45a3885639AC7E10F9d54979").unwrap();
        let d = Did::from_str("0xdddfee254729296a45a3885639AC7E10F9d54979").unwrap();
        let mut v = vec![c, b, a, d];
        v.sort(a);
        assert_eq!(v, vec![a, b, c, d]);
        v.sort(b);
        assert_eq!(v, vec![b, c, d, a]);
        v.sort(c);
        assert_eq!(v, vec![c, d, a, b]);
        v.sort(d);
        assert_eq!(v, vec![d, a, b, c]);
    }
}
