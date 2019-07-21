#![no_std]

use cevtib;

fn main() {
    let mut b = cevtib::BitVec::new();

    for i in 0..10 {
        b.push(i % 2 == 0);
    }

    assert_eq!(Some(true), b.get(2));
    assert_eq!(Some(false), b.get(3));
}
