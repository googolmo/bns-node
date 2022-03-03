/// implementation of CHORD DHT
/// ref: https://pdos.csail.mit.edu/papers/ton:chord/paper-ton.pdf
/// With high probability, the number of nodes that must be contacted to find a successor in an N-node network is O(log N).
use crate::did::Did;
use anyhow::anyhow;
use anyhow::Result;
use num_bigint::BigUint;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RemoteAction {
    // Ask did__a to find did_b
    FindSuccessor((Did, Did)),
    // ask Did_a to notify(did_b)
    Notify((Did, Did)),
    FindSuccessorAndAddToFinger((u8, Did, Did)),
    CheckPredecessor(Did),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChordAction {
    None,
    Some(Did),
    RemoteAction(RemoteAction),
}

#[derive(Clone, Debug)]
pub struct Chord {
    // first node on circle that succeeds (n + 2 ^(k-1) ) mod 2^m , 1 <= k<= m
    // for index start with 0, it should be (n+2^k) mod 2^m
    pub finger: Vec<Option<Did>>,
    // The next node on the identifier circle; finger[1].node
    pub successor: Did,
    // The previous node on the identifier circle
    pub predecessor: Option<Did>,
    pub id: Did,
    pub fix_finger_index: u8,
}

impl Chord {
    // create a new Chord ring.
    pub fn new(id: Did) -> Self {
        Self {
            successor: id,
            predecessor: None,
            // for Eth address, it's 160
            finger: vec![None; 160],
            id,
            fix_finger_index: 0,
        }
    }

    // join a Chord ring containing node id .
    pub fn join(&mut self, id: Did) -> ChordAction {
        if id == self.id {
            return ChordAction::None;
        }
        for k in 0u32..160u32 {
            // (n + 2^k) % 2^m >= n
            // pos >= id
            // from n to n + 2^160
            let pos = self.id + Did::from(BigUint::from(2u16).pow(k));
            // pos less than id or id is on another side of ring
            if pos <= id || pos >= -id {
                println!("{:?}", pos);
                match self.finger[k as usize] {
                    Some(v) => {
                        // for a existed value v
                        // if id < v, then it's more close to this range
                        if id < v || id > -v {
                            self.finger[k as usize] = Some(id);
                            // if id is more close to successor
                        }
                    }
                    None => {
                        self.finger[k as usize] = Some(id);
                    }
                }
            }
        }
        if (id - self.id) < (id - self.successor) || self.id == self.successor {
            // 1) id should follows self.id
            // 2) #fff should follow #001 because id space is a Finate Ring
            // 3) #001 - #fff = #001 + -(#fff) = #001
            self.successor = id;
        }
        ChordAction::RemoteAction(RemoteAction::FindSuccessor((self.successor, self.id)))
    }

    // called periodically. verifies n’s immediate
    // successor, and tells the successor about n.
    pub fn stablilize(&mut self) -> ChordAction {
        // x = successor:predecessor;
        // if (x in (n, successor)) { successor = x; successor:notify(n); }
        if let Some(x) = self.predecessor {
            if x > self.id && x < self.successor {
                self.successor = x;
                return ChordAction::RemoteAction(RemoteAction::Notify((x, self.id)));
                // successor.notify(n)
            }
        }
        ChordAction::None
    }

    // n' thinks it might be our predecessor.
    pub fn notify(&mut self, id: Did) {
        // if (predecessor is nil or n' /in (predecessor; n)); predecessor = n';
        match self.predecessor {
            Some(pre) => {
                if id > pre && id < self.id {
                    self.predecessor = Some(id)
                }
            }
            None => self.predecessor = Some(id),
        }
    }

    // called periodically. refreshes finger table entries.
    // next stores the index of the next finger to fix.
    pub fn fix_fingers(&mut self) -> Result<ChordAction> {
        // next = next + 1;
        //if (next > m) next = 1;
        // finger[next] = find_successor(n + 2^(next-1) );
        // for index start with 0
        // finger[next] = find_successor(n + 2^(next) );
        self.fix_finger_index += 1;
        if self.fix_finger_index >= 160 {
            self.fix_finger_index = 0;
        }
        let did: BigUint = (BigUint::from(self.id)
            + BigUint::from(2u16).pow(self.fix_finger_index.into()))
            % BigUint::from(2u16).pow(160);
        match self.find_successor(Did::try_from(did)?) {
            Ok(res) => match res {
                ChordAction::Some(v) => {
                    self.finger[self.fix_finger_index as usize] = Some(v);
                    Ok(ChordAction::None)
                }
                ChordAction::RemoteAction(RemoteAction::FindSuccessor((a, b))) => {
                    Ok(ChordAction::RemoteAction(
                        RemoteAction::FindSuccessorAndAddToFinger((self.fix_finger_index, a, b)),
                    ))
                }
                _ => {
                    log::error!("Invalid Chord Action");
                    Err(anyhow!("Invalid Chord Action"))
                }
            },
            Err(e) => Err(anyhow!(e)),
        }
    }

    // called periodically. checks whether predecessor has failed.
    pub fn check_predecessor(&self) -> ChordAction {
        match self.predecessor {
            Some(p) => ChordAction::RemoteAction(RemoteAction::CheckPredecessor(p)),
            None => ChordAction::None,
        }
    }

    pub fn closest_preceding_node(&self, id: Did) -> Result<Did> {
        for i in (0..159).rev() {
            if let Some(v) = self.finger[i] {
                if v > self.id && v < id {
                    // check a recorded did x in (self.id, target_id)
                    return Ok(v);
                }
            }
        }
        Err(anyhow!("cannot find cloest preceding node"))
    }

    // Fig.5 n.find_successor(id)
    pub fn find_successor(&self, id: Did) -> Result<ChordAction> {
        // if (id \in (n; successor]); return successor
        if self.id < id && id <= self.successor {
            Ok(ChordAction::Some(id))
        } else {
            // n = closest preceding node(id);
            // return n.find_successor(id);
            match self.closest_preceding_node(id) {
                Ok(n) => Ok(ChordAction::RemoteAction(RemoteAction::FindSuccessor((
                    n, id,
                )))),
                Err(e) => Err(anyhow!(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_chord_finger() {
        let a = Did::from_str("0x00E807fcc88dD319270493fB2e822e388Fe36ab0").unwrap();
        let b = Did::from_str("0x119999cf1046e68e36E1aA2E0E07105eDDD1f08E").unwrap();
        let c = Did::from_str("0xccffee254729296a45a3885639AC7E10F9d54979").unwrap();
        let d = Did::from_str("0xffffee254729296a45a3885639AC7E10F9d54979").unwrap();

        assert!(a < b && b < c);
        // distence between (a, d) is less than (b, d)
        assert!((a - d) < (b - d));

        let mut node_a = Chord::new(a);
        assert_eq!(node_a.successor, a);

        // for increase seq join
        node_a.join(a);
        // Node A wont add self to finder
        assert_eq!(node_a.finger, [None; 160]);
        node_a.join(b);
        // b is very far away from a
        // a.finger should store did as range
        // [(a, a+2), (a+2, a+4), (a+4, a+8), ..., (a+2^159, a + 2^160)]
        // b is in range(a+2^156, a+2^157)
        assert!(BigUint::from(b) > BigUint::from(2u16).pow(156));
        assert!(BigUint::from(b) < BigUint::from(2u16).pow(157));
        // Node A's finter should be [None, .., B]
        assert!(node_a.finger.contains(&Some(b)));
        assert!(node_a.finger.contains(&None));

        // Node A starts to query node b for it's successor
        assert_eq!(
            node_a.join(b),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((b, a)))
        );
        assert_eq!(node_a.successor, b);
        // Node A keep querying node b for it's successor
        assert_eq!(
            node_a.join(c),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((b, a)))
        );
        // Node A's finter should be [None, ..B, C]
        assert!(node_a.finger.contains(&Some(c)), "{:?}", node_a.finger);
        // c is in range(a+2^159, a+2^160)
        assert!(BigUint::from(c) > BigUint::from(2u16).pow(159));
        assert!(BigUint::from(c) < BigUint::from(2u16).pow(160));

        assert_eq!(node_a.finger[159], Some(c));
        assert_eq!(node_a.finger[158], Some(c));
        assert_eq!(node_a.finger[155], Some(b));
        assert_eq!(node_a.finger[156], Some(b));

        assert_eq!(node_a.successor, b);
        // Node A will query c to find d
        assert_eq!(
            node_a.find_successor(d).unwrap(),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((c, d)))
        );
        assert_eq!(
            node_a.find_successor(c).unwrap(),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((b, c)))
        );

        // for decrease seq join
        let mut node_d = Chord::new(d);
        assert_eq!(
            node_d.join(c),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((c, d)))
        );
        assert_eq!(
            node_d.join(b),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((b, d)))
        );
        assert_eq!(
            node_d.join(a),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((a, d)))
        );

        // for over half ring join
        let mut node_d = Chord::new(d);
        assert_eq!(node_d.successor, d);
        assert_eq!(
            node_d.join(a),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((a, d)))
        );
        // for a ring a, a is over 2^152 far away from d
        assert!(d + Did::from(BigUint::from(2u16).pow(152)) > a);
        assert!(d + Did::from(BigUint::from(2u16).pow(151)) < a);
        assert!(node_d.finger.contains(&Some(a)));
        assert_eq!(node_d.finger[151], Some(a));
        assert_eq!(node_d.finger[152], None);
        assert_eq!(node_d.finger[0], Some(a));
        // when b insearted a is still more close to d
        assert_eq!(
            node_d.join(b),
            ChordAction::RemoteAction(RemoteAction::FindSuccessor((a, d)))
        );
        assert!(d + Did::from(BigUint::from(2u16).pow(159)) > b);
        assert_eq!(node_d.successor, a);
    }
}
