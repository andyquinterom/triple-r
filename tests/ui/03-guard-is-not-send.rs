use std::thread;
use triple_r::hashmap::ReusableHashMap;

fn main() {
    let mut map = ReusableHashMap::<String, String>::default();
    let guard = map.recycle::<String, String>();

    // This closure requires `guard` to be `Send`.
    let handle = thread::spawn(move || {
        // The compiler should prevent this because `ReusableHashMapGuard` is not `Send`.
        // It holds a mutable reference to `map`, which is not safe to share across
        // threads without synchronization.
        println!("Length: {}", guard.len());
    });

    handle.join().unwrap();
}
