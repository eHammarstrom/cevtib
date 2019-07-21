use cevtib;

fn main() {
    let mut b = cevtib::BitVec::<u8>::new();

    for i in 0..10 {
        b.push(i % 2 == 0);
    }

    assert_eq!(Some(true), b.get(2));
    assert_eq!(Some(false), b.get(3));

    for i in 0..10 {
        print!("{}, ", b.get(i).unwrap());
    }
    println!();

    println!("{}", b);
}
