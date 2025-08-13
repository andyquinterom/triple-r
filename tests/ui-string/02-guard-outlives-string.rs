use triple_r::ReusableString;

fn main() {
    let mut guard;
    {
        let mut s = ReusableString::default();
        // The guard borrows `s`, which has a shorter lifetime than `guard`.
        guard = s.recycle();
        // `s` is dropped at the end of this scope.
    }

    // This line should fail to compile because `guard` now holds a dangling
    // reference to the data from `s`, which has been dropped.
    println!("Length: {}", guard.len());
}
