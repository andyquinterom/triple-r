use crate::ReuseCastInto;
use std::{
    cell::UnsafeCell,
    collections::{hash_map::RandomState, HashMap},
    hash::BuildHasher,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A wrapper around `HashMap` that allows for reusing its allocation across
/// different key and value types, provided they are compatible.
///
/// `ReusableHashMap` is designed for performance-critical scenarios where heap
/// allocations are a bottleneck. By recycling the `HashMap`'s allocation, you
/// can avoid the overhead of deallocating and reallocating memory between
/// operations.
///
/// The core of this mechanism is the `recycle` method, which returns a
/// [`ReusableHashMapGuard`]. This guard provides temporary, exclusive access to the
/// underlying `HashMap`. When the guard is dropped, the map is cleared, but its
/// allocation is preserved and ready for the next use.
///
/// The type parameters `K` and `V` must be `'static` to ensure that the
/// `ReusableHashMap` can hold any value type. The actual lifetime and type
/// constraints are enforced on the `recycle` method.
///
/// # Safety
///
/// This struct uses `UnsafeCell` to hold the `HashMap`, which allows for mutating
/// the map's contents even with a shared reference. The safety of this operation
/// is guaranteed by the `recycle` method, which requires a mutable reference to
/// `self`, ensuring that only one `ReusableHashMapGuard` can exist at a time.
/// This prevents concurrent access and data races.
///
/// # Examples
///
/// Basic reuse with the same types:
///
/// ```
/// use triple_r::ReusableHashMap;
///
/// let mut reusable_map = ReusableHashMap::<String, i32>::default();
///
/// // First use
/// {
///     let mut map_guard = reusable_map.recycle::<String, i32>();
///     map_guard.insert("one".to_string(), 1);
///     assert_eq!(map_guard.get("one"), Some(&1));
/// } // map_guard is dropped here, and the map is cleared.
///
/// // The map is now empty, but its allocation is ready for reuse.
/// // Second use
/// {
///     let mut map_guard = reusable_map.recycle::<String, i32>();
///     assert!(map_guard.is_empty());
///     map_guard.insert("two".to_string(), 2);
///     assert_eq!(map_guard.len(), 1);
/// }
/// ```
///
/// Reusing a map with different lifetimes:
///
/// ```
/// use triple_r::{ReusableHashMap, ReuseCastInto};
///
/// // This trait implementation is necessary to allow the cast.
/// // It is already implemented for references in the crate.
/// // unsafe impl<'l1, 'l2, T: ?Sized> ReuseCastInto<&'l2 T> for &'l1 T {}
///
/// let mut reusable_map = ReusableHashMap::<&'static str, i32>::default();
///
/// {
///     let key = "temporary";
///     let mut map_guard = reusable_map.recycle::<&str, i32>();
///     map_guard.insert(key, 100);
///     assert_eq!(map_guard.get(key), Some(&100));
/// } // The guard is dropped, and `key` can no longer be accessed through it.
/// ```
#[derive(Debug)]
pub struct ReusableHashMap<K: 'static, V: 'static, S: 'static + BuildHasher + Default = RandomState>
{
    inner: UnsafeCell<HashMap<K, V, S>>,
}

// The `ReusableHashMap` is safe to send across threads if its contents are `Send`.
// The `UnsafeCell` contains the data, and if the data `K`, `V`, `S` is `Send`,
// then the entire `ReusableHashMap` can be safely sent to another thread.
unsafe impl<K: Send, V: Send, S: 'static + Send + BuildHasher + Default> Send
    for ReusableHashMap<K, V, S>
{
}
// The `ReusableHashMap` is safe to share across threads if its contents are `Send`.
// This is because the `recycle` method, which provides access to the inner `HashMap`,
// requires a mutable borrow `&mut self`. This ensures that only one thread can
// access the map at a time when not protected by a `Mutex` or other lock.
// When you wrap `ReusableHashMap` in a `Mutex`, you can safely share it and
// call `recycle` from multiple threads, as the lock serializes access.
unsafe impl<K: Send, V: Send, S: 'static + Send + BuildHasher + Default> Sync
    for ReusableHashMap<K, V, S>
{
}

impl<K: 'static, V: 'static, S: 'static + BuildHasher + Default> Default
    for ReusableHashMap<K, V, S>
{
    /// Creates a new, empty `ReusableHashMap` with the default hasher.
    ///
    /// # Examples
    ///
    /// ```
    /// use triple_r::ReusableHashMap;
    ///
    /// let mut map = ReusableHashMap::<String, i32>::default();
    /// assert_eq!(map.recycle::<String, i32>().capacity(), 0);
    /// ```
    fn default() -> Self {
        Self {
            inner: UnsafeCell::new(HashMap::default()),
        }
    }
}

/// A RAII guard that provides temporary, exclusive access to a `HashMap`
/// retrieved from a [`ReusableHashMap`].
///
/// This guard is created by the [`ReusableHashMap::recycle`] method. It holds a
/// mutable pointer to the underlying `HashMap`, allowing it to be read from and
/// written to.
///
/// When `ReusableHashMapGuard` is dropped, it clears the `HashMap`, ensuring
/// that its contents are not carried over to the next use. However, the memory
/// allocation of the `HashMap` is preserved, which is the key to its efficiency.
///
/// The lifetime `'parent` ensures that the guard cannot outlive the `ReusableHashMap`
/// from which it was borrowed.
///
/// # Type Parameters
///
/// - `'parent`: The lifetime of the mutable borrow of the parent [`ReusableHashMap`].
/// - `K1`, `V1`: The original key and value types of the `ReusableHashMap`.
/// - `K2`, `V2`: The new key and value types for the current use.
/// - `S`: The `BuildHasher` used by the `HashMap`.
pub struct ReusableHashMapGuard<'parent, K1, V1, K2, V2, S>
where
    K1: 'static,
    V1: 'static,
    S: 'static + BuildHasher + Default,
{
    inner: *mut HashMap<K2, V2, S>,
    _parent: PhantomData<&'parent mut ReusableHashMap<K1, V1, S>>,
}

impl<'parent, K1, V1, K2, V2, S> Deref for ReusableHashMapGuard<'parent, K1, V1, K2, V2, S>
where
    K1: 'static,
    V1: 'static,
    S: 'static + BuildHasher + Default,
{
    type Target = HashMap<K2, V2, S>;

    /// Provides immutable access to the underlying `HashMap`.
    ///
    /// This allows you to call any of `HashMap`'s immutable methods directly
    /// on the guard.
    ///
    /// # Safety
    ///
    /// `self.inner` is a valid pointer for the lifetime `'parent`. This is
    /// enforced by the `_parent` phantom data and the signature of the `recycle`
    /// method. The parent `ReusableHashMap` is mutably borrowed for `'parent`,
    /// so no other access is possible during the guard's lifetime.
    fn deref(&self) -> &Self::Target {
        // SAFETY: `self.inner` is a valid pointer for the lifetime `'parent`.
        // This is enforced by `_parent` and the `reuse` method signature.
        // The parent `ReusableHashMap` is mutably borrowed for `'parent`,
        // so no other access is possible.
        unsafe { &*self.inner }
    }
}

impl<'parent, K1, V1, K2, V2, S> DerefMut for ReusableHashMapGuard<'parent, K1, V1, K2, V2, S>
where
    K1: 'static,
    V1: 'static,
    S: 'static + BuildHasher + Default,
{
    /// Provides mutable access to the underlying `HashMap`.
    ///
    /// This allows you to call any of `HashMap`'s mutable methods directly
    /// on the guard, such as `insert` or `clear`.
    ///
    /// # Safety
    ///
    /// The same safety guarantees as `deref` apply. Mutable access is safe
    /// because the `'parent` mutable borrow on the original map prevents any
    /// other code from accessing it.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The same guarantees as `deref` apply. We provide mutable access,
        // which is safe because the `'parent` mutable borrow on the original map
        // prevents any other code from accessing it.
        unsafe { &mut *self.inner }
    }
}

impl<K1, V1, S> ReusableHashMap<K1, V1, S>
where
    K1: 'static,
    V1: 'static,
    S: 'static + BuildHasher + Default,
{
    /// Borrows the `HashMap` for temporary use, returning a guard that allows
    /// access to it.
    ///
    /// This method allows you to "cast" the key and value types of the `HashMap`
    /// to new types (`K2`, `V2`), provided that the original types (`K1`, `V1`)
    /// implement `ReuseCastInto<K2>` and `ReuseCastInto<V2>` respectively.
    ///
    /// Taking `&mut self` as a parameter is crucial for safety, as it ensures
    /// that only one `ReusableHashMapGuard` can exist at any given time for a
    /// specific `ReusableHashMap`. This prevents data races.
    ///
    /// When the returned guard is dropped, the map is cleared, but its memory
    /// allocation is preserved for future use.
    ///
    /// # Type Parameters
    ///
    /// - `'parent`: The lifetime of the returned guard, tied to the mutable
    ///   borrow of `self`.
    /// - `K2`, `V2`: The new key and value types to use for the `HashMap`.
    ///
    /// # Safety
    ///
    /// This method performs a transmutation of the `HashMap`'s generic types.
    /// It is safe because:
    /// 1. The `ReuseCastInto` trait bounds ensure that the type transmutation
    ///    is valid (e.g., `&'static str` to `&'a str`).
    /// 2. The borrow checker ensures the returned guard does not outlive `self`.
    /// 3. The `&mut self` receiver prevents multiple guards from being created
    ///    simultaneously.
    ///
    /// # Examples
    ///
    /// ```
    /// use triple_r::ReusableHashMap;
    ///
    /// let mut map = ReusableHashMap::<String, String>::default();
    /// {
    ///     let mut guard = map.recycle::<String, String>();
    ///     guard.insert("key".to_string(), "value".to_string());
    ///     assert_eq!(guard.len(), 1);
    /// } // Guard is dropped, map is cleared.
    ///
    /// assert!(map.recycle::<String, String>().is_empty());
    /// ```
    pub fn recycle<'parent, K2, V2>(
        &'parent mut self,
    ) -> ReusableHashMapGuard<'parent, K1, V1, K2, V2, S>
    where
        K1: ReuseCastInto<K2>,
        V1: ReuseCastInto<V2>,
    {
        // SAFETY: We use `get()` to obtain a raw pointer to the hash map.
        // This is safe because we have `&mut self`, guaranteeing exclusive
        // access. This avoids creating an intermediate `&mut` reference that
        // could be invalidated, which was the source of the Miri error.
        let inner_ptr = self.inner.get() as *mut HashMap<K2, V2, S>;

        ReusableHashMapGuard {
            inner: inner_ptr,
            _parent: PhantomData,
        }
    }
}

impl<'parent, K1, V1, K2, V2, S> Drop for ReusableHashMapGuard<'parent, K1, V1, K2, V2, S>
where
    K1: 'static,
    V1: 'static,
    S: 'static + BuildHasher + Default,
{
    /// Clears the underlying `HashMap` upon being dropped.
    ///
    /// This is the core of the reuse mechanism. By clearing the map instead of
    /// dropping it, we preserve its memory allocation (capacity) for the next
    /// user. This avoids the cost of deallocation and reallocation.
    ///
    /// # Safety
    ///
    /// The pointer `self.inner` is guaranteed to be valid because the lifetime
    /// `'parent` ensures the guard does not outlive the `ReusableHashMap` it
    /// was created from.
    fn drop(&mut self) {
        // SAFETY: The pointer `self.inner` is guaranteed to be valid.
        // We get a mutable reference and clear the map, making it ready for
        // the next reuse.
        unsafe {
            (*self.inner).clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::BuildHasherDefault;
    use std::sync::Mutex;
    use twox_hash::XxHash64;

    #[test]
    fn reference_reuse_works() {
        let mut map = ReusableHashMap::<&'static str, &'static str>::default();
        {
            let hello = String::from("Hello");
            let world = String::from("World");
            let mut r_map = map.recycle();
            r_map.insert(hello.as_str(), world.as_str());
            assert_eq!(r_map.get("Hello"), Some(&world.as_str()));
        }
        assert!(unsafe { (*map.inner.get()).is_empty() });
    }

    #[test]
    fn string_identity_reuse_works() {
        let mut map = ReusableHashMap::<String, String>::default();
        {
            let mut r_map = map.recycle::<String, String>();
            r_map.insert("hello".to_string(), "world".to_string());
            assert_eq!(r_map.get("hello"), Some(&"world".to_string()));
        }
        assert!(unsafe { (*map.inner.get()).is_empty() });
    }

    #[test]
    fn primitive_reuse_works() {
        let mut map = ReusableHashMap::<i32, i32>::default();
        {
            let mut r_map = map.recycle::<i32, i32>();
            r_map.insert(1, 2);
            assert_eq!(r_map.get(&1), Some(&2));
        }
        assert!(unsafe { (*map.inner.get()).is_empty() });
    }

    #[test]
    fn empty_reuse_is_still_empty_after_drop() {
        let mut map = ReusableHashMap::<String, String>::default();
        {
            let _r_map = map.recycle::<String, String>();
        }
        assert!(unsafe { (*map.inner.get()).is_empty() });
    }

    #[test]
    fn sequential_reuse_works() {
        let mut map = ReusableHashMap::<String, String>::default();
        {
            let mut r_map = map.recycle::<String, String>();
            r_map.insert("one".to_string(), "1".to_string());
            assert_eq!(r_map.len(), 1);
        }
        assert!(unsafe { (*map.inner.get()).is_empty() });
        {
            let mut r_map = map.recycle::<String, String>();
            r_map.insert("two".to_string(), "2".to_string());
            r_map.insert("three".to_string(), "3".to_string());
            assert_eq!(r_map.len(), 2);
        }
        assert!(unsafe { (*map.inner.get()).is_empty() });
    }

    #[test]
    fn custom_hasher_reuse_works() {
        type CustomHasher = BuildHasherDefault<XxHash64>;
        let mut map = ReusableHashMap::<i32, i32, CustomHasher>::default();
        {
            let mut r_map = map.recycle::<i32, i32>();
            r_map.insert(1, 2);
            assert_eq!(r_map.get(&1), Some(&2));
        }
        assert!(unsafe { (*map.inner.get()).is_empty() });
    }

    #[test]
    fn mutex_reuse_works() {
        let reusable_map = Mutex::new(ReusableHashMap::<i32, i32>::default());
        {
            let mut map_guard = reusable_map.lock().unwrap();
            let mut r_map = map_guard.recycle::<i32, i32>();
            r_map.insert(1, 2);
            assert_eq!(r_map.get(&1), Some(&2));
        }
        let map_guard = reusable_map.lock().unwrap();
        assert!(unsafe { (*map_guard.inner.get()).is_empty() });
    }
}
