// Copyright 2012-2020 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// FIXME(Gankro): BitVec and BitSet are very tightly coupled. Ideally (for
// maintenance), they should be in separate files/modules, with BitSet only
// using BitVec's public API. This will be hard for performance though, because
// `BitVec` will not want to leak its internal representation while its internal
// representation as `u32`s must be assumed for best performance.

// (1) Be careful, most things can overflow here because the amount of bits in
//     memory can overflow `usize`.
// (2) Make sure that the underlying vector has no excess length:
//     E. g. `nbits == 16`, `storage.len() == 2` would be excess length,
//     because the last word isn't used at all. This is important because some
//     methods rely on it (for *CORRECTNESS*).
// (3) Make sure that the unused bits in the last word are zeroed out, again
//     other methods rely on it for *CORRECTNESS*.
// (4) `BitSet` is tightly coupled with `BitVec`, so any changes you make in
// `BitVec` will need to be reflected in `BitSet`.

use crate::*;


#[derive(BorshDeserialize, BorshSerialize)]
pub struct BitVec 
{
    /// Internal representation of the bit vector
    storage: UnorderedMap<u32, u32>,
    /// The number of valid bits in the internal representation
    nbits: u32
}

/// Computes how many blocks are needed to store that many bits
fn blocks_for_bits(bits: u32) -> u32 {
    // If we want 17 bits, dividing by 32 will produce 0. So we add 1 to make sure we
    // reserve enough. But if we want exactly a multiple of 32, this will actually allocate
    // one too many. So we need to check if that's the case. We can do that by computing if
    // bitwise AND by `32 - 1` is 0. But LLVM should be able to optimize the semantically
    // superior modulo operator on a power of two to this.
    //
    // Note that we can technically avoid this branch with the expression
    // `(nbits + U32_BITS - 1) / 32::BITS`, but if nbits is almost usize::MAX this will overflow.
    if bits % u32::BITS == 0 {
        bits / u32::BITS
    } else {
        bits / u32::BITS + 1
    }
}
impl BitVec {

    pub fn from_elem(nbits: u32) -> Self {
        let nblocks = blocks_for_bits(nbits);
        let mut bit_vec = BitVec {
            storage: UnorderedMap::new(b'v'),
            nbits,
        };
        bit_vec
    }
}

impl BitVec {


    /// Retrieves the value at index `i`, or `None` if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use bit_vec::BitVec;
    ///
    /// let bv = BitVec::from_bytes(&[0b01100000]);
    /// assert_eq!(bv.get(0), Some(false));
    /// assert_eq!(bv.get(1), Some(true));
    /// assert_eq!(bv.get(100), None);
    ///
    /// // Can also use array indexing
    /// assert_eq!(bv[1], true);
    /// ```
    #[inline]
    pub fn get(&self, i: u32) -> Option<bool> {
        if i >= self.nbits {
            return None;
        }
        let w = i / u32::BITS;
        let b = i % u32::BITS;
        self.storage.get(&w).map(|block|
            (block & ((1 as u32) << b)) != 0
        )
    }

    /// Sets the value of a bit at an index `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use bit_vec::BitVec;
    ///
    /// let mut bv = BitVec::from_elem(5, false);
    /// bv.set(3, true);
    /// assert_eq!(bv[3], true);
    /// ```
    #[inline]
    pub fn set(&mut self, i: u32, x: bool) {
        assert!(i < self.nbits, "index out of bounds: {:?} >= {:?}", i, self.nbits);
        let w = i / u32::BITS;
        let b = i % u32::BITS;
        let flag = (1 as u32) << b;
        let val = if x { self.storage.get(&w).unwrap_or(0) | flag }
                  else { self.storage.get(&w).unwrap_or(0) & !flag };
        self.storage.insert(&w, &val);
    }
}

// impl<u32> Clone for BitVec<u32> {
//     #[inline]
//     fn clone(&self) -> Self {
//         self.ensure_invariant();
//         BitVec { storage: self.storage.clone(), nbits: self.nbits }
//     }

//     #[inline]
//     fn clone_from(&mut self, source: &Self) {
//         debug_assert!(source.is_last_block_fixed());
//         self.nbits = source.nbits;
//         self.storage.clone_from(&source.storage);
//     }
// }

