use std::hash::{BuildHasherDefault, RandomState};
use triple_r::hashmap::{ReusableHashMap, ReusableHashMapGuard};
use twox_hash::XxHash64;

// A custom hasher type for the test.
type CustomHasher = BuildHasherDefault<XxHash64>;

fn main() {
    // Create a map that uses a custom hasher.
    let mut map = ReusableHashMap::<String, String, CustomHasher>::default();

    // The following line should fail to compile.
    // We are trying to reuse the map, but we are explicitly annotating the
    // resulting guard with a different hasher (`RandomState`) than the one
    // the map was created with (`CustomHasher`).
    //
    // The compiler should catch this type mismatch, demonstrating that the
    // hasher type is correctly preserved and cannot be changed during reuse.
    let _: ReusableHashMapGuard<'_, String, String, String, String, RandomState> =
        map.recycle::<String, String>();
}
