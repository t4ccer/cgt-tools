//! Fixed-size bit vec

use super::BitVecRef;
use std::ops::{Deref, DerefMut};

/// Fixed-size bit vec
///
/// Note that the size is in *bytes* rather than bits since that would require `generic_const_exprs`
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedBitVec<const BYTE_LEN: usize> {
    data: [u8; BYTE_LEN],
}

impl<const BYTE_LEN: usize> std::fmt::Debug for FixedBitVec<BYTE_LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        BitVecRef::from_inner(&self.data).fmt(f)
    }
}

impl<const BYTE_LEN: usize> FixedBitVec<BYTE_LEN> {
    /// Construct new empty (all false) bitvec
    #[inline]
    pub const fn new() -> FixedBitVec<BYTE_LEN> {
        FixedBitVec {
            data: [0; BYTE_LEN],
        }
    }

    /// Construct new filled (all true) bitvec
    #[inline]
    pub const fn filled() -> FixedBitVec<BYTE_LEN> {
        FixedBitVec {
            data: [u8::MAX; BYTE_LEN],
        }
    }
}

impl<const BYTE_LEN: usize> Deref for FixedBitVec<BYTE_LEN> {
    type Target = BitVecRef;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        BitVecRef::from_inner(&self.data)
    }
}

impl<const BYTE_LEN: usize> DerefMut for FixedBitVec<BYTE_LEN> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        BitVecRef::from_inner_mut(&mut self.data)
    }
}

#[test]
fn get_set() {
    let mut bit_vec = FixedBitVec::<{ 32 / 8 }>::new();
    bit_vec.set(10, true);
    bit_vec.set(16, true);
    assert!(!bit_vec.get(4));
    assert!(bit_vec.get(10));
    assert!(!bit_vec.get(14));
    assert!(bit_vec.get(16));
    assert!(!bit_vec.get(30));
}
