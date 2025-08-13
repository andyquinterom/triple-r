use std::thread;
use triple_r::vec::ReusableVec;

fn main() {
    let mut vec = ReusableVec::<String>::default();
    let guard = vec.recycle::<String>();

    // This closure requires `guard` to be `Send`.
    let handle = thread::spawn(move || {
        // The compiler should prevent this because `ReusableVecGuard` is not `Send`.
        // It holds a raw pointer to the vector's data, which is not safe to move across
        // threads.
        println!("Length: {}", guard.len());
    });

    handle.join().unwrap();
}
