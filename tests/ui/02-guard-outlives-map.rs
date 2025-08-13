use triple_r::hashmap::ReusableHashMap;

fn main() {
    let mut guard;
    {
        let mut map = ReusableHashMap::<String, String>::default();
        guard = map.recycle::<String, String>();
        // `map` is dropped at the end of this scope.
    }

    // The following line should fail to compile because `guard` holds a reference
    // to `map`'s data, but `map` has already been dropped. The borrow checker
    // should prevent `guard` from outliving `map`.
    let _ = guard.keys();
}
