use crate::*;

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{IntoStorageKey};

use super::raw_value::RawValue;


#[derive(BorshSerialize, BorshDeserialize)]
pub struct TreeMap {
    node_index: u32,
    tree: Vec<u8>,
    pub max_bits: u32,   // a fixed height tree
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Node {
    next: LookupMap<bool,Vec<u8>>   //make vec<u8> as next node's key or final bits
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
    for i in (0..u32::BITS / u8::BITS).rev() {
        let bits = ((u8::MAX as u32) << (i * u8::BITS)) & node_index;
        if bits == 0 { continue }
        bytes.push(bits as u8)
    }
    bytes
}

const MAX_VAL_BYTE: u32 = 1;   //the last 8 bits to represent 256 bit fields

impl TreeMap
{
    pub fn new(max_bits: u32, key_prefix: String) -> Self {
        Self {
            node_index: 0,
            tree: key_prefix.as_bytes().to_vec(),
            max_bits,
        }
    }

    

    pub fn set(&mut self, key: &[u8], val: u8) {
        let root_key = self.tree.clone();
        let mut node = Node {
            next: LookupMap::new(root_key.into_storage_key())
        };
        //find the bits node
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit == 1 {true} else {false};
    
            let raw_key = get_raw_key(&bit, &node);
            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap::new(raw_key.into_storage_key())
                };
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

        let mut bit_vec = RawValue::try_from(node.next.get(&true).unwrap_or(Vec::new())).unwrap();
        let bit_index: u8 = key[self.max_bits as usize + MAX_VAL_BYTE as usize]; 
        bit_vec.set_val(bit_index, val);
        node.next.insert(&true, &(bit_vec.try_into().unwrap()));

    }

    pub fn del(&mut self, key: &[u8]) {
        let root_key = self.tree.clone();
        let mut node = Node {
            next: LookupMap::new(root_key.into_storage_key())
        };
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit == 1 {true} else {false};
    
            let raw_key = get_raw_key(&bit, &node);

            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap::new(raw_key.into_storage_key())
                }
            } else {
                return
            }
        }

        let mut bit_vec = RawValue::try_from(
            match node.next.get(&true) {
                Some(v) => v,
                None => {
                    return
                }
            }
        ).unwrap();
        let bit_index: u8 = key[self.max_bits as usize + MAX_VAL_BYTE as usize]; 
        bit_vec.del_val(bit_index);
        node.next.insert(&true, &(bit_vec.try_into().unwrap()));
    }

    pub fn get(&self, key: &[u8]) -> Option<u8> {
        let root_key = self.tree.clone();
        let mut node = Node {
            next: LookupMap::new(root_key.into_storage_key())
        };
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit == 1 {true} else {false};

            let raw_key = get_raw_key(&bit, &node);

            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap::new(raw_key.into_storage_key())
                }
            } else {
                return None
            }
        }
        let bit_vec = RawValue::try_from(
            match node.next.get(&true) {
                Some(v) => v,
                None => {
                    return None
                }
            }
        ).unwrap();

        let bit_index: u8 = key[self.max_bits as usize + MAX_VAL_BYTE as usize]; 
        bit_vec.get_val(bit_index)
    }

    pub fn check(&self, key: &[u8]) -> bool {
        let root_key = self.tree.clone();
        let mut node = Node {
            next: LookupMap::new(root_key.into_storage_key())
        };
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit == 1 {true} else {false};

            let raw_key = get_raw_key(&bit, &node);
            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap::new(raw_key.into_storage_key())
                }
            } else {
                return false
            }
        }
        node.next.get(&true).is_some()
    }

    pub fn get_all_nodes(&self) -> Vec<HashMap<(u32, bool), Option<u32>>> {
        let mut arr = Vec::new();
        for i in 0..self.node_index {
            let mut map = HashMap::new();
            let raw_key = make_raw_key(i);
            let node = Node {
                next: LookupMap::new(raw_key)
            };
            match node.next.get(&true) {
                Some(v) => {
                    map.insert((i, true), Some(u32::try_from_slice(&v[0..4]).unwrap()))
                },
                None => map.insert((i, true), None)
            };
            match node.next.get(&true) {
                Some(v) => map.insert((i, false), Some(u32::try_from_slice(&v[0..4]).unwrap())),
                None => map.insert((i, false), None)
            };
            arr.push(map);
        }
        arr
    }
}
