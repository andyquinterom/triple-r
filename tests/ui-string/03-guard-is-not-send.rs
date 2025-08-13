use std::thread;
use triple_r::ReusableString;

fn main() {
    let mut s = ReusableString::default();
    let guard = s.recycle();

    // This closure requires `guard` to be `Send`.
    let handle = thread::spawn(move || {
        // The compiler should prevent this because `ReusableStringGuard` is not `Send`.
        // It holds a raw pointer to the string's data, which is not safe to move across
        // threads.
        println!("Length: {}", guard.len());
    });

    handle.join().unwrap();
}
