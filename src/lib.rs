//! # triple-r: Recycle, Reuse, Reduce
//!
//! [![Crates.io](https://img.shields.io/crates/v/triple-r.svg)](https://crates.io/crates/triple-r)
//! [![Docs.rs](https://docs.rs/triple-r/badge.svg)](https://docs.rs/triple-r)
//! [![CI](https://github.com/andyquinterom/triple-r/actions/workflows/ci.yml/badge.svg)](https://github.com/andyquinterom/triple-r/actions/workflows/ci.yml)
//!
//! `triple-r` is a high-performance Rust library that provides wrappers around standard library collections to enable the reuse of their memory allocations. By recycling the underlying memory of collections like `HashMap` and `Vec`, `triple-r` helps reduce allocation overhead in performance-critical applications.
//!
//! ---
//!
//! ## The Problem
//!
//! In many applications, especially those processing data in loops (e.g., servers, game engines, data processing pipelines), collections are often created, populated, used, and then discarded. This pattern can lead to frequent memory allocations and deallocations, which can become a significant performance bottleneck.
//!
//! ```rust
//! # use std::collections::HashMap;
//! # let data_stream: Vec<i32> = vec![];
//! // A typical pattern that causes repeated allocations.
//! for item in data_stream {
//!     let mut map = HashMap::new(); // Allocates on every iteration
//!     // ... populate and use the map ...
//!     # map.insert(item, item);
//! } // map is dropped, and its memory is deallocated.
//! ```
//!
//! ## The Solution: `triple-r`
//!
//! `triple-r` provides "reusable" versions of standard collections that solve this problem. It wraps collections like `HashMap` and `Vec` in a container that preserves their allocation when they are no longer needed.
//!
//! The core of the library is a RAII guard pattern. You `recycle` a reusable container to get a temporary guard object. You use this guard just like a regular collection. When the guard goes out of scope, it clears the collection but keeps the underlying memory allocation, making it ready for the next cycle.
//!
//! ```rust
//! use triple_r::ReusableHashMap;
//! # let data_stream: Vec<i32> = vec![];
//!
//! // Create the reusable container once.
//! let mut reusable_map = ReusableHashMap::<i32, i32>::default();
//!
//! for item in data_stream {
//!     // Recycle the allocation. This is fast and avoids a new allocation.
//!     let mut map_guard = reusable_map.recycle();
//!     // ... populate and use the guard ...
//!     # map_guard.insert(item, item);
//! } // guard is dropped, the map is cleared, but the allocation is kept.
//! ```
//!
//! ## Key Features
//!
//! - **Allocation Reuse:** Provides [`ReusableHashMap`] and [`ReusableVec`] to avoid repeated memory allocations.
//! - **Type Casting:** Safely cast the types of the stored elements between uses. For example, a `ReusableHashMap<&'static str, _>` can be recycled into a guard for a `HashMap<&'a str, _>`.
//! - **Compile-Time Safety:** The API is designed to prevent common misuses at compile time, such as having multiple mutable references to the same underlying collection.
//! - **Safety Assured:** The internal use of `unsafe` code is minimal and has been carefully designed and verified with `cargo miri` to ensure it is free of undefined behavior.
//! - **Drop-In Replacement:** The guard objects implement `Deref` and `DerefMut`, so you can use them just like standard `HashMap` and `Vec`.
//!
//! ## Usage
//!
//! ### ReusableHashMap
//!
//! ```rust
//! use triple_r::ReusableHashMap;
//!
//! let mut reusable_map = ReusableHashMap::<String, i32>::default();
//! let mut last_capacity = 0;
//!
//! for i in 0..3 {
//!     // Obtain a guard to the map.
//!     let mut map_guard = reusable_map.recycle();
//!
//!     // The capacity is preserved from the previous iteration.
//!     assert_eq!(map_guard.capacity(), last_capacity);
//!
//!     map_guard.insert(format!("key-{}", i), i);
//!     assert_eq!(map_guard.get(&format!("key-{}", i)), Some(&i));
//!     assert_eq!(map_guard.len(), 1);
//!
//!     last_capacity = map_guard.capacity();
//!     // `map_guard` is dropped here, clearing the map.
//! }
//!
//! // After the loop, the map is empty, but the final capacity is retained.
//! let mut final_guard = reusable_map.recycle::<String, i32>();
//! assert!(final_guard.is_empty());
//! assert_eq!(final_guard.capacity(), last_capacity);
//! ```
//!
//! ### ReusableVec
//!
//! ```rust
//! use triple_r::ReusableVec;
//!
//! let mut reusable_vec = ReusableVec::<u8>::default();
//! let mut last_capacity = 0;
//!
//! for i in 0..5 {
//!     // Recycle the vector's allocation.
//!     let mut vec_guard = reusable_vec.recycle();
//!     assert!(vec_guard.is_empty());
//!     assert_eq!(vec_guard.capacity(), last_capacity);
//!
//!     vec_guard.extend(0..i as u8);
//!     assert_eq!(vec_guard.len(), i);
//!     last_capacity = vec_guard.capacity();
//! }
//! ```
//!
//! ### Reusing with Different Lifetimes
//!
//! A powerful feature is the ability to change the lifetime of references within the collection. This is useful when you have a long-lived `ReusableHashMap` but need to use it with short-lived data.
//!
//! ```rust
//! use triple_r::ReusableHashMap;
//!
//! // The reusable map can hold static string slices.
//! let mut reusable_map = ReusableHashMap::<&'static str, i32>::default();
//!
//! {
//!     // But we can use it with a short-lived string.
//!     let short_lived_key = "hello".to_string();
//!     let mut map_guard = reusable_map.recycle::<&str, i32>(); // Note the type hint
//!     map_guard.insert(&short_lived_key, 123);
//!     assert_eq!(map_guard.get("hello"), Some(&123));
//! } // `short_lived_key` and `map_guard` are dropped here.
//!
//! // The allocation is ready for another use.
//! let mut map_guard = reusable_map.recycle::<&str, i32>();
//! assert!(map_guard.is_empty());
//! ```
//!
//! ## Safety
//!
//! This library uses `unsafe` code to perform the type transmutation and to work with raw pointers inside the guard. The safety of this implementation is ensured by the following principles:
//!
//! 1.  **Exclusive Access:** The `recycle()` method requires a mutable reference (`&mut self`) to the [`ReusableHashMap`] or [`ReusableVec`]. This statically guarantees that only one guard can be active at a time, preventing data races.
//! 2.  **Lifetime Management:** The returned guard is tied to the lifetime of the `&mut self` borrow, ensuring it cannot outlive the container it references.
//! 3.  **Miri Verification:** The entire codebase is tested with `cargo miri`, a tool that detects undefined behavior in `unsafe` Rust code. All tests pass under Miri, giving strong confidence in the library's soundness.
pub mod hashmap;
pub mod string;
pub mod vec;
pub use hashmap::{ReusableHashMap, ReusableHashMapGuard};
pub use string::{ReusableString, ReusableStringGuard};
pub use vec::{ReusableVec, ReusableVecGuard};

/// A trait that indicates that a type can be safely cast into another type for the
/// purpose of reusing a collection's allocation.
///
/// # Safety
///
/// This trait is unsafe to implement because it allows for type transmutation
/// through pointer casting. Implementers must guarantee that it is safe to
/// transmute a container of `Self` (e.g., `Vec<Self>`) into a container of `T`
/// (e.g., `Vec<T>`).
///
/// For this library, this is primarily used to change the lifetimes of references
/// (e.g., from `&'static str` to `&'a str`), which is safe because the
/// collection is cleared before it is used with the new type, and the new
/// lifetimes are constrained by the guard's lifetime.
///
/// For types with the same memory layout (e.g., primitive integers), this is
/// also safe.
pub unsafe trait ReuseCastInto<T> {}

// This implementation allows reusing a map of references with a shorter lifetime.
// For example, a `HashMap<&'static str, _>` can be reused as a `HashMap<&'a str, _>`.
unsafe impl<T: ?Sized> ReuseCastInto<&T> for &T {}

macro_rules! impl_reuse_cast_into_for_primitive {
    ($($t:ty),*) => {
        $(
            unsafe impl ReuseCastInto<$t> for $t {}
        )*
    };
}

unsafe impl<T> ReuseCastInto<Vec<T>> for Vec<T> {}

impl_reuse_cast_into_for_primitive!(
    // Signed integers
    i8, i16, i32, i64, i128, isize, // Unsigned integers
    u8, u16, u32, u64, u128, usize, // Floating point numbers
    f32, f64, // Other primitives
    bool, char, String
);
