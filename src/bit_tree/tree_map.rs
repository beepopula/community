use crate::*;

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{IntoStorageKey};
use std::{marker::PhantomData};


#[derive(BorshSerialize, BorshDeserialize)]
pub struct TreeMap {
    node_index: u32,
    tree: Node,
    pub max_bits: u32,   // a fixed height tree
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Node {
    next: LookupMap<bool,Vec<u8>>   //make vec<u8> as next node's key or final bits
}

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Value(Vec<(u8, u16)>);

impl Value {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseVecError {}

impl TryFrom<Vec<u8>> for Value {
    type Error = ();

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value).unwrap()
    }
}

impl From<Value> for Vec<u8> {
    fn from(value: Value) -> Self {
        json!(value).to_string().into_bytes()
    }
}

fn get_raw_key(bit: &bool, node: &Node) -> Option<Vec<u8>> {
    match node.next.get(bit) {
        Some(v) => {
            if v.len() == 0 {
                None
            } else {
                Some(v)
            }
        },
        None => None
    }
}

fn make_raw_key(node_index: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in (0..u32::BITS / 8).rev() {
        let bits = ((u8::BITS as u32) << i) & node_index;
        if bits == 0 { continue }
        bytes.push(bits as u8)
    }
    bytes
}

const MAX_VAL_BYTE: u32 = 1;   //the last 8 bits to represent 256 bit fields
const VAL_BITS: u32 = 2;      //use 2 bits to store value

impl TreeMap
{
    pub fn new(max_bits: u32, key_prefix: String) -> Self {
        Self {
            node_index: 0,
            tree: Node {
                next: LookupMap::new(key_prefix.as_bytes())
            },
            max_bits,
        }
    }

    

    pub fn set(&mut self, key: &[u8], val: u8) {
        let mut node = self.tree;

        //find the bits node
        for i in 0..(self.max_bits as usize + 1) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit == 1 {true} else {false};
    
            let raw_key = get_raw_key(&bit, &node);
            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap {
                        key_prefix: raw_key.into_storage_key(), 
                        el: PhantomData
                    }
                }
            } else {
                self.node_index += 1;
                let new_key = make_raw_key(self.node_index); 
                let new_node = Node {
                    next: LookupMap::new(new_key.clone())
                };
                node.next.insert(&bit, &new_key);
                node = new_node;
            }
        }

        let bit_vec = node.next.get(&true).unwrap_or(Vec::new());
        let mut bit_vec = Value::try_from(bit_vec).unwrap();

        let bit_index: u8 = key[self.max_bits as usize + MAX_VAL_BYTE as usize]; 
        let w = bit_index / u8::BITS as u8;
        let b = bit_index % u8::BITS as u8;
        let flag = (1 as u8) << b;

        for i in 0..w {
            if bit_vec.0.get(i as usize).is_none() {
                bit_vec.0.push((0 as u8, 0 as u16))
            }
        }

        let mut bit = bit_vec.0.get_mut(w as usize).unwrap();
        bit.0 |= flag;
        let flag = (val as u16) << (b * 2);
        bit.1 |= flag;
        node.next.insert(&true, &(Vec::from(bit_vec) as Vec<u8>));

    }

    pub fn del(&mut self, key: &[u8]) {
        let mut node = self.tree;
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit == 1 {true} else {false};
    
            let raw_key = get_raw_key(&bit, &node);

            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap {
                        key_prefix: raw_key.into_storage_key(), 
                        el: PhantomData
                    }
                }
            } else {
                return
            }
        }

        let bit_vec = match node.next.get(&true) {
            Some(v) => v,
            None => {
                return
            }
        };
        let mut bit_vec = Value::try_from(bit_vec).unwrap();

        let bit_index: u8 = key[self.max_bits as usize + MAX_VAL_BYTE as usize]; 
        let w = bit_index / u8::BITS as u8;
        let b = bit_index % u8::BITS as u8;
        let flag = (1 as u8) << b;

        for i in 0..w {
            if bit_vec.0.get(i as usize).is_none() {
                return
            }
        }

        let mut bit = bit_vec.0.get_mut(w as usize).unwrap();
        bit.0 &= !flag;
        let flag = (3 as u16) << (b * 2);
        bit.1 &= !flag;
        node.next.insert(&true, &(Vec::from(bit_vec) as Vec<u8>));
    }

    pub fn get(&self, key: &[u8]) -> Option<u8> {
        let mut node = self.tree;
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit == 1 {true} else {false};

            let raw_key = get_raw_key(&bit, &node);

            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap {
                        key_prefix: raw_key.into_storage_key(), 
                        el: PhantomData
                    }
                }
            } else {
                return None
            }
        }
        let bit_vec = match node.next.get(&true) {
            Some(v) => v,
            None => {
                return None
            }
        };
        let bit_vec = Value::try_from(bit_vec).unwrap();

        let bit_index: u8 = key[self.max_bits as usize + MAX_VAL_BYTE as usize]; 
        let w = bit_index / u8::BITS as u8;
        let b = bit_index % u8::BITS as u8;
        let flag = (1 as u8) << b;

        for i in 0..w {
            if bit_vec.0.get(i as usize).is_none() {
                return None
            }
        }

        let bit = bit_vec.0.get(w as usize).unwrap();
        if (bit.0 | flag) >> b == 0 {
            return None
        }
        let flag = (3 as u16) << (b * VAL_BITS as u8);
        Some(((bit.1 | flag) >> (b * VAL_BITS as u8)) as u8)
    }

    pub fn check(&self, key: &[u8]) -> bool {
        let mut node = self.tree;
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit == 1 {true} else {false};

            let raw_key = get_raw_key(&bit, &node);

            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap {
                        key_prefix: raw_key.into_storage_key(), 
                        el: PhantomData
                    }
                }
            } else {
                return false
            }
        }
        node.next.get(&true).is_some()
    }
}
