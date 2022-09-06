use crate::*;

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{IntoStorageKey};

use super::raw_value::RawValue;


#[derive(BorshSerialize, BorshDeserialize)]
pub struct TreeMap {
    node_index: u32,
    tree: Vec<u8>,
    bit_width: u8,
    pub max_bits: u32,   // a fixed height tree
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Node{
    next: (Vec<u8>, Vec<u8>), //make vec<u8> as next node's key or final bits
    key: Vec<u8>
}


impl TryFrom<Vec<u8>> for Node {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        match value.try_into() {
            Ok(v) => Ok(v),
            Err(e) => Err(e)
        }
    }
}

impl Node {

    pub fn new(raw_key: &[u8], key_prefix: &[u8]) -> Self {
        let key = &[raw_key.clone(), key_prefix.clone()].concat();
        let storage: (Vec<u8>, Vec<u8>) = match env::storage_read(key) {
            Some(v) => BorshDeserialize::deserialize(&mut v.as_ref()).unwrap(),
            None => (Vec::new(), Vec::new())
        };
        Self {
            next: storage,
            key: key.to_vec(),
        }
    }


    pub fn get(&self, bit: &bool) -> Option<Vec<u8>> {
        match bit {
            true => {
                if self.next.1.is_empty() {
                    return None
                }
                Some(self.next.1.clone())
            },
            false => {
                if self.next.0.is_empty() {
                    return None
                }
                Some(self.next.0.clone())
            }
        }
    }
    
    pub fn set(&mut self, bit: &bool, node_index: u32) -> Vec<u8> {
        assert!((bit == &false && self.next.0.is_empty()) || (bit == &true && self.next.1.is_empty()), "set error");

        let mut bytes = Vec::new();
        let mut highest_index = 0;
        for i in (0..u32::BITS / u8::BITS).rev() {
            let mut bits = ((u8::MAX as u32) << (i * u8::BITS)) & node_index;
            if bits == 0 && i > highest_index { continue }
            highest_index = i;
            bits = bits >> (i * u8::BITS);
            bytes.push(bits as u8)
        }

        match bit {
            true => {
                self.next.1 = bytes.clone();
            },
            false => {
                self.next.0 = bytes.clone();
            }
        }
        env::storage_write(&self.key, &self.next.try_to_vec().unwrap());
        bytes
    }

    pub fn set_val(&mut self, bit: &bool, val: Vec<u8>) {
        match bit {
            true => self.next.1 = val,
            false => self.next.0 = val,
        }
        env::storage_write(&self.key, &self.next.try_to_vec().unwrap());
    }
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
    pub fn new(max_bits: u32, key_prefix: Vec<u8>, bit_width: u8) -> Self {
        Self {
            node_index: 0,
            tree: key_prefix,
            max_bits,
            bit_width
        }
    }

    

    pub fn set(&mut self, key: &[u8], val: u8) {
        let root_key = self.tree.clone();
        let mut node = Node::new(&vec![], &root_key);
        //find the bits node
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit > 0 {true} else {false};

            let raw_key = node.get(&bit);
            if let Some(raw_key) = raw_key {
                node = Node::new(&raw_key, &root_key);
            } else {
                self.node_index += 1;
                let new_key = node.set(&bit, self.node_index);
                let new_node = Node::new(&new_key, &root_key);
                node = new_node;
            }
        }
        let mut bit_vec = RawValue::new(node.get(&true).unwrap_or(Vec::new()), self.bit_width);
        let bit_index: u8 = get_u8(self.max_bits, key);
        bit_vec.set_val(bit_index, val);
        node.set_val(&true, bit_vec.try_into().unwrap());

    }

    pub fn del(&mut self, key: &[u8]) {
        let root_key = self.tree.clone();
        let mut node = Node::new(&vec![], &root_key);
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit > 0 {true} else {false};
    
            let raw_key = node.get(&bit);
            if let Some(raw_key) = raw_key {
                node = Node::new(&raw_key, &root_key);
            } else {
                return
            }
        }

        let mut bit_vec = RawValue::new(
            match node.get(&true) {
                Some(v) => v,
                None => {
                    return
                }
            }
        , self.bit_width);
        let bit_index: u8 = get_u8(self.max_bits, key);
        bit_vec.del_val(bit_index);
        node.set_val(&true, bit_vec.try_into().unwrap());
    }

    pub fn get(&self, key: &[u8]) -> Option<u8> {
        let root_key = self.tree.clone();
        let mut node = Node::new(&vec![], &root_key);
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit > 0 {true} else {false};
            let raw_key = node.get(&bit);

            if let Some(raw_key) = raw_key {
                node = Node::new(&raw_key, &root_key);
            } else {
                return None
            }
        }
        let bit_vec = RawValue::new(
            match node.get(&true) {
                Some(v) => v,
                None => {
                    return None
                }
            }
        , self.bit_width);

        let bit_index: u8 = get_u8(self.max_bits, key);
        bit_vec.get_val(bit_index)
    }

    pub fn check(&self, key: &[u8]) -> bool {
        let root_key = self.tree.clone();
        let mut node = Node::new(&vec![], &root_key);
        for i in 0..(self.max_bits as usize) {
            let bytes = i / u8::BITS as usize;
            let bits = i % u8::BITS as usize;
            let block = key[bytes];

            let bit = block & ((1 as u8) << bits);
            let bit = if bit > 0 {true} else {false};

            let raw_key = node.get(&bit);
            if let Some(raw_key) = raw_key {
                node = Node::new(&raw_key, &root_key);
            } else {
                return false
            }
        }
        let bit_vec = RawValue::new(
            match node.get(&true) {
                Some(v) => v,
                None => {
                    return false
                }
            }
        , self.bit_width);

        let bit_index: u8 = get_u8(self.max_bits, key);
        bit_vec.get_val(bit_index).is_some()
    }
}



#[cfg(test)]
mod tests {
    use near_sdk::env;

    use crate::bit_tree::{tree_map::{get_u8}, BitTree};


    #[test]
    pub fn test() {
        let arr: Vec<u8> = vec![44, 236, 49, 109, 179, 223, 84, 234, 247, 12, 229, 59, 27, 84, 177, 70, 75, 115, 100, 209, 117, 121, 112, 241, 92, 182, 155, 50, 187, 142, 233, 57];
        get_u8(54, &arr);
    }

    #[test]
    pub fn test_set() {
        let arr_1: Vec<u8> = vec![30, 251, 48, 71, 8, 210, 194, 46, 32, 19, 145, 193, 172, 138, 250, 149, 194, 75, 213, 124, 130, 229, 51, 16, 153, 211, 74, 53, 150, 219, 63, 1];
        let arr_2: Vec<u8> = vec![191, 157, 146, 44, 88, 172, 153, 245, 116, 125, 38, 50, 216, 166, 80, 124, 177, 238, 154, 37, 88, 193, 67, 164, 59, 215, 150, 219, 60, 249, 68, 3];
        let mut bit_tree = BitTree::new(28, vec![1], 0);
        bit_tree.set(&arr_1, 0);
        bit_tree.set(&arr_2, 0);
        let res = bit_tree.get(&arr_1);
        println!("{:?}", res);
    }

    #[test]
    pub fn test_node_index() {
        let arr: Vec<u8> = vec![30, 251, 48, 71, 8, 210, 194, 46, 32, 19, 145, 193, 172, 138, 250, 149, 194, 75, 213, 124, 130, 229, 51, 16, 153, 211, 74, 53, 150, 219, 63, 1];
        let mut bit_tree = BitTree::new(28, vec![1], u16::BITS as u8);
        bit_tree.set(&arr, 2);
        let res = env::storage_read(&vec![1,28,1]).unwrap();
        println!("res: {:?}", res);
    }
}