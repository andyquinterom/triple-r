use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A wrapper around `String` that allows for reusing its allocation.
///
/// This is particularly useful in loops where strings are built and discarded,
/// as it helps to avoid frequent memory allocations. When the guard returned
/// by `recycle` is dropped, the `String` is cleared, but its capacity is
/// retained.
///
/// # Examples
///
/// ```
/// use triple_r::ReusableString;
///
/// let mut reusable_string = ReusableString::default();
/// let mut last_capacity = 0;
///
/// for i in 0..3 {
///     let mut string_guard = reusable_string.recycle();
///     assert!(string_guard.is_empty());
///     assert_eq!(string_guard.capacity(), last_capacity);
///
///     string_guard.push_str("hello world");
///     last_capacity = string_guard.capacity();
/// }
///
/// let final_guard = reusable_string.recycle();
/// assert!(final_guard.is_empty());
/// assert_eq!(final_guard.capacity(), last_capacity);
/// ```
#[derive(Debug)]
pub struct ReusableString {
    inner: UnsafeCell<String>,
}

// A `ReusableString` can be sent across threads.
unsafe impl Send for ReusableString {}

// A `ReusableString` can be shared across threads because the `recycle`
// method requires `&mut self`, preventing data races.
unsafe impl Sync for ReusableString {}

impl Default for ReusableString {
    /// Creates a new, empty `ReusableString`.
    fn default() -> Self {
        Self {
            inner: UnsafeCell::new(String::new()),
        }
    }
}

/// A RAII guard that provides temporary, exclusive access to a `String` from a
/// [`ReusableString`].
///
/// When this guard is dropped, it clears the underlying `String`, preserving its
/// allocation for future use.
pub struct ReusableStringGuard<'parent> {
    inner: *mut String,
    _parent: PhantomData<&'parent mut ReusableString>,
}

impl<'parent> Deref for ReusableStringGuard<'parent> {
    type Target = String;

    /// Provides immutable access to the underlying `String`.
    fn deref(&self) -> &Self::Target {
        // SAFETY: `self.inner` is a valid pointer for the lifetime `'parent`.
        // This is enforced by `_parent` and the `recycle` method signature.
        unsafe { &*self.inner }
    }
}

impl<'parent> DerefMut for ReusableStringGuard<'parent> {
    /// Provides mutable access to the underlying `String`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The same guarantees as `deref` apply.
        unsafe { &mut *self.inner }
    }
}

impl ReusableString {
    /// Reuses the `String`'s allocation, returning a guard for temporary access.
    ///
    /// The `&mut self` requirement ensures that only one guard can be active
    /// at a time.
    pub fn recycle<'parent>(&'parent mut self) -> ReusableStringGuard<'parent> {
        // SAFETY: We use `get()` to obtain a raw pointer, which is safe
        // because `&mut self` guarantees exclusive access.
        ReusableStringGuard {
            inner: self.inner.get(),
            _parent: PhantomData,
        }
    }
}

impl<'parent> Drop for ReusableStringGuard<'parent> {
    /// Clears the `String` when the guard is dropped.
    fn drop(&mut self) {
        // SAFETY: The pointer is guaranteed to be valid for the lifetime
        // of the guard. Clearing the string prepares it for the next reuse.
        unsafe {
            (*self.inner).clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_reuse_works() {
        let mut s = ReusableString::default();
        {
            let mut guard = s.recycle();
            guard.push_str("hello");
            assert_eq!(*guard, "hello");
        }
        let guard = s.recycle();
        assert!(guard.is_empty());
        assert!(guard.capacity() >= 5);
    }

    #[test]
    fn capacity_is_preserved() {
        let mut s = ReusableString::default();
        let last_capacity;
        {
            let mut guard = s.recycle();
            guard.push_str("some long string to ensure allocation");
            last_capacity = guard.capacity();
        }
        assert!(last_capacity > 0);
        let guard = s.recycle();
        assert_eq!(guard.capacity(), last_capacity);
    }

    #[test]
    fn empty_reuse_is_still_empty() {
        let mut s = ReusableString::default();
        {
            let _guard = s.recycle();
        }
        let guard = s.recycle();
        assert!(guard.is_empty());
        assert_eq!(guard.capacity(), 0);
    }
}
