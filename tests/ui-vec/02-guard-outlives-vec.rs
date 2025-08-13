use triple_r::vec::ReusableVec;

fn main() {
    let mut guard;
    {
        let mut vec = ReusableVec::<String>::default();
        guard = vec.recycle::<String>();
        // `vec` is dropped at the end of this scope.
    }

    // The following line should fail to compile because `guard` holds a reference
    // to `vec`'s data, but `vec` has already been dropped. The borrow checker
    // should prevent `guard` from outliving `vec`.
    let _ = guard.len();
}
