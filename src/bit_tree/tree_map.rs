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
        Some(v) => Some(v),
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

fn get_u8(bit_index: u32, key: &[u8]) -> u8 {
    let bytes = bit_index / u8::BITS;
    let bits = bit_index % u8::BITS;
    let mut block = key[bytes as usize];
    if bits == 0 {
        block
    } else {
        let mut val:u8 = 0;
        for i in bits..bits + u8::BITS {
            let mut flag = 0;
            let mut offset = i;
            if offset >= u8::BITS {
                block = key[bytes as usize + 1];
                offset = offset - u8::BITS;
            } 
            flag = 1 << offset;
            
            if (block & flag) >> offset == 1 {
                val |= flag;
            }
        }
        val
    }
}

const MAX_VAL_BYTE: u32 = 1;   //the last 8 bits to represent 256 bit fields

impl TreeMap
{
    pub fn new(max_bits: u32, key_prefix: Vec<u8>) -> Self {
        Self {
            node_index: 0,
            tree: key_prefix,
            max_bits,
        }
    }

    

    pub fn set(&mut self, key: &[u8], val: u8) {
        let root_key = self.tree.clone();
        let mut node = Node {
            next: LookupMap::new(root_key)
        };
        //find the bits node
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit > 0 {true} else {false};

            let raw_key = get_raw_key(&bit, &node);
            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap::new([raw_key.clone(), self.tree.clone()].concat())
                };
            } else {
                self.node_index += 1;
                let new_key = make_raw_key(self.node_index); 
                let new_node = Node {
                    next: LookupMap::new([new_key.clone(), self.tree.clone()].concat())
                };
                node.next.insert(&bit, &new_key);
                node = new_node;
            }
        }
        let mut bit_vec = RawValue::try_from(node.next.get(&true).unwrap_or(Vec::new())).unwrap();
        let bit_index: u8 = get_u8(self.max_bits, key);
        bit_vec.set_val(bit_index, val);
        node.next.insert(&true, &(bit_vec.try_into().unwrap()));

    }

    pub fn del(&mut self, key: &[u8]) {
        let root_key = self.tree.clone();
        let mut node = Node {
            next: LookupMap::new(root_key)
        };
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit > 0 {true} else {false};
    
            let raw_key = get_raw_key(&bit, &node);

            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap::new([raw_key.clone(), self.tree.clone()].concat())
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
        let bit_index: u8 = get_u8(self.max_bits, key);
        bit_vec.del_val(bit_index);
        node.next.insert(&true, &(bit_vec.try_into().unwrap()));
    }

    pub fn get(&self, key: &[u8]) -> Option<u8> {
        let root_key = self.tree.clone();
        let mut node = Node {
            next: LookupMap::new(root_key)
        };
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit > 0 {true} else {false};
            let raw_key = get_raw_key(&bit, &node);

            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap::new([raw_key.clone(), self.tree.clone()].concat())
                };
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

        let bit_index: u8 = get_u8(self.max_bits, key);
        bit_vec.get_val(bit_index)
    }

    pub fn check(&self, key: &[u8]) -> bool {
        let root_key = self.tree.clone();
        let mut node = Node {
            next: LookupMap::new(root_key)
        };
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit > 0 {true} else {false};

            let raw_key = get_raw_key(&bit, &node);
            if let Some(raw_key) = raw_key {
                node = Node {
                    next: LookupMap::new([raw_key.clone(), self.tree.clone()].concat())
                }
            } else {
                return false
            }
        }
        let bit_vec = RawValue::try_from(
            match node.next.get(&true) {
                Some(v) => v,
                None => {
                    return false
                }
            }
        ).unwrap();

        let bit_index: u8 = get_u8(self.max_bits, key);
        bit_vec.get_val(bit_index).is_some()
    }
}



#[cfg(test)]
mod tests {
    use near_sdk::env;

    use crate::bit_tree::{tree_map::get_u8, BitTree};


    #[test]
    pub fn test() {

    }

    #[test]
    pub fn test_set() {
        let arr: Vec<u8> = vec![44, 236, 49, 109, 179, 223, 84, 234, 247, 12, 229, 59, 27, 84, 177, 70, 75, 115, 100, 209, 117, 121, 112, 241, 92, 182, 155, 50, 187, 142, 233, 57];
        let mut bit_tree = BitTree::new(28, vec![1]);
        bit_tree.set(&arr, 2);
        let res = bit_tree.get(&arr);
        println!("{:?}", res);
    }
}