// (C)opyleft 2013-2021 Frank Denis

//! Bloom filter for Rust
//!
//! This is a simple but fast Bloom filter implementation, that requires only
//! 2 hash functions, generated with SipHash-1-3 using randomized keys.
//!


mod bit_vec;

use std::collections::hash_map::DefaultHasher;
use std::f64;
use std::hash::{Hash, Hasher};

use crate::*;

use self::bit_vec::BitVec;

#[derive(Serialize, Deserialize, Hash)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub struct WrappedHash([u8;32]);


impl From<[u8;32]> for WrappedHash {
    fn from(hash: [u8;32]) -> Self {
        WrappedHash(hash)
    }
}

/// Bloom filter structure
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Bloom {
    bit_vec: BitVec,
    bitmap_bits: u32,
    k_num: u32,
}

impl Bloom {
    /// Create a new bloom filter structure.
    /// bitmap_size is the size in bytes (not bits) that will be allocated in memory
    /// items_count is an estimation of the maximum number of items to store.
    /// seed is a random value used to generate the hash functions.
    pub fn new(bitmap_size: usize, items_count: usize) -> Self {
        assert!(bitmap_size > 0 && items_count > 0);
        let bitmap_bits = (bitmap_size as u32) * 8u32;
        let k_num = Self::optimal_k_num(bitmap_bits as u64, items_count);
        let bitmap = BitVec::from_elem(bitmap_bits as u32);
        Self {
            bit_vec: bitmap,
            bitmap_bits,
            k_num,
        }
    }

    /// Create a new bloom filter structure.
    /// items_count is an estimation of the maximum number of items to store.
    /// fp_p is the wanted rate of false positives, in ]0.0, 1.0[
    pub fn new_for_fp_rate_with_seed(items_count: usize, fp_p: f64) -> Self {
        let bitmap_size = Self::compute_bitmap_size(items_count, fp_p);
        Bloom::new(bitmap_size, items_count)
    }

    /// Compute a recommended bitmap size for items_count items
    /// and a fp_p rate of false positives.
    /// fp_p obviously has to be within the ]0.0, 1.0[ range.
    pub fn compute_bitmap_size(items_count: usize, fp_p: f64) -> usize {
        assert!(items_count > 0);
        assert!(fp_p > 0.0 && fp_p < 1.0);
        let log2 = f64::consts::LN_2;
        let log2_2 = log2 * log2;
        ((items_count as f64) * f64::ln(fp_p) / (-8.0 * log2_2)).ceil() as usize
    }

    /// Record the presence of an item.
    pub fn set(&mut self, item: &WrappedHash)
    {
        let mut hashes = [0u64, 0u64];
        for k_i in 0..self.k_num {
            let bit_offset = (self.bloom_hash(&mut hashes, item, k_i) % self.bitmap_bits as u64) as u32;
            self.bit_vec.set(bit_offset, true);
        }
    }

    /// Check if an item is present in the set.
    /// There can be false positives, but no false negatives.
    pub fn check(&self, item: &WrappedHash) -> bool
    {
        let mut hashes = [0u64, 0u64];
        for k_i in 0..self.k_num {
            let bit_offset = (self.bloom_hash(&mut hashes, item, k_i) % self.bitmap_bits as u64) as u32;
            if self.bit_vec.get(bit_offset).unwrap() == false {
                return false;
            }
        }
        true
    }

    /// Record the presence of an item in the set,
    /// and return the previous state of this item.
    pub fn check_and_set(&mut self, item: &WrappedHash) -> bool
    {
        let mut hashes = [0u64, 0u64];
        let mut found = true;
        for k_i in 0..self.k_num {
            let bit_offset = (self.bloom_hash(&mut hashes, item, k_i) % self.bitmap_bits as u64);
            if self.bit_vec.get(bit_offset as u32).unwrap() == false {
                found = false;
                self.bit_vec.set(bit_offset as u32, true);
            }
        }
        found
    }

    /// Return the bitmap as a "BitVec" structure
    pub fn bit_vec(&self) -> &BitVec {
        &self.bit_vec
    }

    /// Return the number of bits in the filter
    pub fn number_of_bits(&self) -> u32 {
        self.bitmap_bits
    }

    /// Return the number of hash functions used for `check` and `set`
    pub fn number_of_hash_functions(&self) -> u32 {
        self.k_num
    }

    #[allow(dead_code)]
    fn optimal_k_num(bitmap_bits: u64, items_count: usize) -> u32 {
        let m = bitmap_bits as f64;
        let n = items_count as f64;
        let k_num = (m / n * f64::ln(2.0f64)).ceil() as u32;
        match k_num > 1 {
            true => k_num,
            false => 1,
        }
    }

    fn bloom_hash(&self, hashes: &mut [u64; 2], item: &WrappedHash, k_i: u32) -> u64
    {
        if k_i < 2 {
            let mut s = DefaultHasher::new();
            item.hash(&mut s);
            let hash = s.finish();
            hashes[k_i as usize] = hash;
            hash
        } else {
            (hashes[0] as u128).wrapping_add((k_i as u128).wrapping_mul(hashes[1] as u128)) as u64
                % 0xffffffffffffffc5
        }
    }
}

#[test]
#[cfg(feature = "random")]
fn bloom_test_set() {
    let mut bloom = Bloom::new(10, 80);
    let mut k = vec![0u8, 16];
    getrandom(&mut k).unwrap();
    assert!(bloom.check(&k) == false);
    bloom.set(&k);
    assert!(bloom.check(&k) == true);
}

#[test]
#[cfg(feature = "random")]
fn bloom_test_check_and_set() {
    let mut bloom = Bloom::new(10, 80);
    let mut k = vec![0u8, 16];
    getrandom(&mut k).unwrap();
    assert!(bloom.check_and_set(&k) == false);
    assert!(bloom.check_and_set(&k) == true);
}

#[test]
#[cfg(feature = "random")]
fn bloom_test_clear() {
    let mut bloom = Bloom::new(10, 80);
    let mut k = vec![0u8, 16];
    getrandom(&mut k).unwrap();
    bloom.set(&k);
    assert!(bloom.check(&k) == true);
    bloom.clear();
    assert!(bloom.check(&k) == false);
}

#[test]
#[cfg(feature = "random")]
fn bloom_test_load() {
    let mut original = Bloom::new(10, 80);
    let mut k = vec![0u8, 16];
    getrandom(&mut k).unwrap();
    original.set(&k);
    assert!(original.check(&k) == true);

    let cloned = Bloom::from_existing(
        &original.bitmap(),
        original.number_of_bits(),
        original.number_of_hash_functions(),
        original.sip_keys(),
    );
    assert!(cloned.check(&k) == true);
}