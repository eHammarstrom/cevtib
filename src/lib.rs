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
    /// Length of current sequence, index = len - 1
    len: usize,
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
            len: 0,
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

    /// Retrieve boolean within capacity bounds, this may
    /// return a default initilization of value `false`.
    pub fn get_unchecked(&self, index: usize) -> bool {
        // out of memory bounds
        assert!(index < self.capacity());

        let store_ptr = self.lookup_store(index);
        let index_mask = self.lookup_mask(index);
        let b = unsafe { *store_ptr } & index_mask;

        b > 0
    }

    /// Retrieve boolean within the current length.
    #[must_use]
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

    /// Set boolean at index `i` within current length.
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
        self.len += 1;

        let index = self.len - 1;

        assert!(self.set(index, val).is_ok());

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

        assert_eq!(192, b.capacity());
        assert_eq!(139, b.len());

        b.grow();

        assert_eq!(256, b.capacity());
        assert_eq!(139, b.len());
        assert_eq!(None, b.get(139));
        assert_eq!(Some(true), b.get(138));
    }
}
