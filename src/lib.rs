use std::alloc;
use std::ops;
use std::ptr;

/// Allocate in blocks of type `B`.
type B = u64;

#[derive(Debug, PartialEq)]
struct BitVec {
    /// Byte sequence used to store bits
    store: *mut B,
    /// Number of byte stores of size B
    num_stores: usize,
    /// Internal index of last unset bit
    index: usize,
}

#[derive(Debug)]
pub enum Error {
    OutOfBounds,
}

impl BitVec {
    fn new() -> BitVec {
        let layout = alloc::Layout::new::<B>();
        let ptr = unsafe { alloc::alloc_zeroed(layout) };

        if ptr.is_null() {
            panic!("unable to initialize (allocate) bitvec");
        }

        #[allow(clippy::cast_ptr_alignment)]
        BitVec {
            store: ptr as *mut _,
            num_stores: 1,
            index: 0,
        }
    }

    #[inline]
    fn store_size() -> usize {
        std::mem::size_of::<B>() * 8
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.num_stores * Self::store_size()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.index + 1
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

    /// Grow number of stores, reallocating with an additional store
    fn grow(&mut self) {
        let layout = alloc::Layout::new::<B>();

        self.num_stores += 1;

        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            self.store = alloc::realloc(
                self.store as *mut _,
                layout,
                self.num_stores * std::mem::size_of::<B>(),
            ) as *mut _;
        }

        if self.store.is_null() {
            panic!("unable to grow (reallocate) bitvec");
        }
    }

    /// Retrieve boolean at unchecked index `i` where
    /// `i` is assumed to be within bounds.
    pub fn get_unchecked(&self, index: usize) -> bool {
        // out of memory bounds
        assert!(index < self.capacity());

        let store_ptr = self.lookup_store(index);
        let index_mask = self.lookup_mask(index);
        let b = unsafe { *store_ptr } & index_mask;

        b > 0
    }

    /// Retrieve boolean at checked index `i`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<bool> {
        if index < self.len() {
            Some(self.get_unchecked(index))
        } else {
            None
        }
    }

    /// Sets boolean at unchecked index `i`.
    pub fn set_unchecked(&mut self, index: usize, element: bool) {
        // out of memory bounds
        assert!(index < self.capacity());

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

    /// Set boolean at index `i`.
    #[must_use]
    pub fn set(&mut self, index: usize, element: bool) -> Result<(), Error> {
        if index < self.len() {
            self.set_unchecked(index, element);
            Ok(())
        } else {
            Err(Error::OutOfBounds)
        }
    }

    /// Push boolean bit onto bit vector.
    pub fn push(&mut self, val: bool) {
        assert!(self.set(self.index, val).is_ok());

        self.index += 1;

        if self.len() >= self.capacity() {
            self.grow();
        }
    }
}

impl ops::Drop for BitVec {
    fn drop(&mut self) {
        let layout = alloc::Layout::new::<B>();

        unsafe { alloc::dealloc(self.store as *mut _, layout) };
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
        assert_eq!(64, BitVec::new().capacity());
    }

    #[test]
    fn bitvec_get_unchecked() {
        let b = BitVec::new();

        assert_eq!(false, b.get_unchecked(0));
        assert_eq!(false, b.get_unchecked(63));
    }

    #[test]
    #[should_panic]
    fn bitvec_get_unchecked_panic() {
        let b = BitVec::new();

        b.get_unchecked(64);
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
    }
}
