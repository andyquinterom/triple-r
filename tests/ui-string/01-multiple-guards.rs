use triple_r::ReusableString;

fn main() {
    let mut s = ReusableString::default();
    // First mutable borrow occurs here.
    let _guard1 = s.recycle();
    // This second mutable borrow should fail because `s` is already borrowed.
    let _guard2 = s.recycle();
}
