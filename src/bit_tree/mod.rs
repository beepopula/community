use crate::*;
use tree_map::TreeMap;

mod tree_map;
mod raw_value;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct BitTree {
    bit_tree: TreeMap,
}

impl BitTree {
    pub fn new(max_bits: u32, key_prefix: Vec<u8>) -> Self {
        Self {
            bit_tree: TreeMap::new(max_bits, key_prefix),
        }
        //total 36 bits, 28 bits instead
    }

    pub fn get(&self, key: &[u8]) -> Option<u8> {
        self.bit_tree.get(key)
    }

    pub fn set(&mut self, key: &[u8], val: u8) {
        self.bit_tree.set(key, val)
    }

    pub fn del(&mut self, key: &[u8]) {
        self.bit_tree.del(key)
    }

    pub fn check(&self ,key: &[u8]) -> bool {
        self.bit_tree.check(key)
    }

    pub fn get_and_set(&mut self, key: &[u8], val: u8 ) -> Option<u8> {
        let bits = self.get(key);
        self.set(key, val);
        bits
    }

    pub fn check_and_set(&mut self, key: &[u8], val: u8 ) -> bool {
        let bits = self.check(key);
        self.set(key, val);
        bits
    }

    pub fn get_all_nodes(&self) -> Vec<HashMap<(u32, bool), Option<u32>>> {
        self.bit_tree.get_all_nodes()
    }

    pub fn test_get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.bit_tree.test_get(key)
    }

    pub fn test_set(&mut self, key: &[u8], val: u8) {
        self.bit_tree.test_set(key, val)
    }
}