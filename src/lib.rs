#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::alloc as __alloc;
#[cfg(not(feature = "std"))]
use core::convert::TryInto;
#[cfg(not(feature = "std"))]
use core::mem;
#[cfg(not(feature = "std"))]
use core::ops;

#[cfg(feature = "std")]
use std::alloc::alloc as __alloc;
#[cfg(feature = "std")]
use std::convert::TryInto;
#[cfg(feature = "std")]
use std::ops;

/// Allocate in blocks of type `B`.
type B = u64;

#[derive(Debug, PartialEq)]
pub struct BitVec {
    /// Byte sequence used to store bits
    store: *mut B,
    /// Number of byte stores of size B
    num_stores: usize,
    /// Length of current sequence, index = len - 1
    len: usize,
}

#[derive(Debug)]
pub enum Error {
    OutOfBounds,
}

impl BitVec {
    pub fn new() -> BitVec {
        let layout = __alloc::Layout::new::<B>();
        let ptr = unsafe { __alloc::alloc_zeroed(layout) };

        if ptr.is_null() {
            panic!("unable to initialize (allocate) bitvec");
        }

        #[allow(clippy::cast_ptr_alignment)]
        BitVec {
            store: ptr as *mut _,
            num_stores: 2,
            len: 0,
        }
    }

    #[inline]
    fn store_size() -> usize {
        mem::size_of::<B>() * 8
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.num_stores * Self::store_size()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn lookup_store(&self, index: usize) -> *const B {
        let store_index = index / Self::store_size();
        unsafe { self.store.add(store_index) }
    }

    #[inline]
    fn lookup_store_mut(&self, index: usize) -> *mut B {
        let store_index = index / Self::store_size();
        unsafe { self.store.add(store_index) }
    }

    #[inline]
    fn lookup_mask(&self, index: usize) -> B {
        let bit_index = index % Self::store_size();
        1 << bit_index
    }

    /// Grow or shrink number of stores by a relative change.
    fn resize(&mut self, change: isize) {
        self.num_stores = (self.num_stores as isize + change)
            .try_into()
            .expect("unable to resize bitvec");

        // We shrank past elements, set new len
        if self.len() > self.capacity() {
            self.len = self.capacity();
        }

        let layout = __alloc::Layout::new::<B>();

        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            self.store = __alloc::realloc(
                self.store as *mut _,
                layout,
                self.num_stores * mem::size_of::<B>(),
            ) as *mut _;
        }

        if self.store.is_null() {
            panic!("unable to grow (reallocate) bitvec");
        }
    }

    /// Double the block allocation.
    fn grow(&mut self) {
        self.resize(self.num_stores as isize);
    }

    /// Removes n store pages.
    pub fn shrink_blocks_by(&mut self, n: isize) {
        self.resize(-n);
    }

    /// Retrieve boolean within capacity bounds, this may
    /// return a default initilization of value `false`.
    pub fn get_unchecked(&self, index: usize) -> bool {
        let store_ptr = self.lookup_store(index);
        let index_mask = self.lookup_mask(index);
        let b = unsafe { *store_ptr } & index_mask;

        b > 0
    }

    /// Retrieve boolean within the current length.
    pub fn get(&self, index: usize) -> Option<bool> {
        if index < self.len() {
            Some(self.get_unchecked(index))
        } else {
            None
        }
    }

    /// Sets any boolean within capacity at index `i`,
    /// without changing the length representation of the bitvec.
    pub fn set_unchecked(&mut self, index: usize, element: bool) {
        let store_ptr_mut = self.lookup_store_mut(index);
        let index_mask = self.lookup_mask(index);

        if element {
            unsafe {
                *store_ptr_mut |= index_mask;
            }
        } else {
            let neg_index_mask = !index_mask;

            unsafe {
                *store_ptr_mut &= neg_index_mask;
            }
        }
    }

    /// Set boolean at index `i` within current length.
    pub fn set(&mut self, index: usize, element: bool) -> Result<(), Error> {
        if index < self.len() {
            self.set_unchecked(index, element);
            Ok(())
        } else {
            Err(Error::OutOfBounds)
        }
    }

    /// Push boolean bit onto bitvec, growing
    /// the bitvec if `len == capacity`.
    pub fn push(&mut self, val: bool) {
        if self.len() >= self.capacity() {
            self.grow();
        }

        self.len += 1;

        let index = self.len - 1;

        assert!(self.set(index, val).is_ok());
    }

    /// Pop boolean bit off the bitvec.
    pub fn pop(&mut self) -> Option<bool> {
        if self.len > 0 {
            self.len -= 1;
            Some(self.get_unchecked(self.len))
        } else {
            None
        }
    }
}

impl ops::Drop for BitVec {
    fn drop(&mut self) {
        let layout = __alloc::Layout::new::<B>();

        unsafe { __alloc::dealloc(self.store as *mut _, layout) };
    }
}

#[cfg(test)]
mod tests {
    use crate::BitVec;

    #[test]
    fn bitvec_alloc() {
        BitVec::new();
    }

    #[test]
    fn bitvec_initial_cap() {
        assert_eq!(128, BitVec::new().capacity());
    }

    #[test]
    fn bitvec_get_unchecked() {
        let b = BitVec::new();

        assert_eq!(false, b.get_unchecked(0));
        assert_eq!(false, b.get_unchecked(63));
    }

    #[test]
    fn bitvec_set_unchecked() {
        let mut b = BitVec::new();

        b.set_unchecked(63, true);
        b.set_unchecked(33, true);
        b.set_unchecked(31, true);

        b.set_unchecked(32, true);
        b.set_unchecked(32, false);

        assert_eq!(false, b.get_unchecked(0));

        assert_eq!(true, b.get_unchecked(63));
        assert_eq!(true, b.get_unchecked(33));
        assert_eq!(true, b.get_unchecked(31));

        assert_eq!(false, b.get_unchecked(32));
    }

    #[test]
    fn bitvec_set() {
        let mut b = BitVec::new();
        b.push(true);
        let r1 = b.set(0, false);
        let r2 = b.set(63, true);
        assert!(r1.is_ok());
        assert!(r2.is_err());
    }

    #[test]
    fn bitvec_grow() {
        let mut b = BitVec::new();
        let num_indices = 139;

        for _ in 0..num_indices {
            b.push(true);
        }

        for i in 0..num_indices {
            let val = b.get(i);
            assert_eq!(Some(true), val);
        }

        assert_eq!(256, b.capacity());
        assert_eq!(139, b.len());

        b.grow();

        assert_eq!(512, b.capacity());
        assert_eq!(139, b.len());
        assert_eq!(None, b.get(139));
        assert_eq!(Some(true), b.get(138));
    }

    #[test]
    fn bitvec_shrink() {
        let mut b = BitVec::new();
        let num_indices = 139;

        for _ in 0..num_indices {
            b.push(true);
        }

        for i in 0..num_indices {
            let val = b.get(i);
            assert_eq!(Some(true), val);
        }

        assert_eq!(256, b.capacity());
        assert_eq!(139, b.len());

        // test spurious false during pop
        let false_index = 128;
        let _ = b.set(false_index, false);

        let remove_indices = 100;

        for i in 0..remove_indices {
            let val = b.pop();

            // check inserted false val on pop
            if num_indices - i - 1 == false_index {
                assert_eq!(Some(false), val);
            } else {
                assert_eq!(Some(true), val);
            }
        }

        b.shrink_blocks_by(2);

        assert_eq!(128, b.capacity());
        assert_eq!(num_indices - remove_indices, b.len());
    }
}
