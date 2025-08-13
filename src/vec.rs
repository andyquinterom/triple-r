use crate::ReuseCastInto;
use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A wrapper around [`Vec`] that allows for reusing its allocation.
///
/// In performance-sensitive applications, frequent creation and destruction of
/// vectors can lead to significant overhead from memory allocations. `ReusableVec`
/// mitigates this by providing a mechanism to recycle a vector's allocation.
///
/// The [`recycle`](ReusableVec::recycle) method returns a [`ReusableVecGuard`], which provides temporary, exclusive access
/// to the underlying vector. When the guard is dropped, the vector is cleared,
/// but its underlying memory allocation is preserved for subsequent use.
///
/// # Type Parameters
///
/// - `T`: The type of elements in the `Vec`. This type must be `'static` because
///   the `ReusableVec` itself holds onto the allocation indefinitely.
///
/// # Safety
///
/// This struct uses an [`UnsafeCell`] to hold the `Vec`, which allows for mutating
/// its contents through a shared reference. The safety of this pattern is
/// guaranteed by the `recycle` method, which requires a mutable reference (`&mut self`).
/// This ensures that only one `ReusableVecGuard` can exist at a time for a given
/// `ReusableVec`, thereby preventing data races.
///
/// # Examples
///
/// ## Basic Reuse
///
/// ```
/// use triple_r::ReusableVec;
///
/// let mut reusable_vec = ReusableVec::<i32>::default();
///
/// // First use: populate the vector
/// {
///     let mut vec_guard = reusable_vec.recycle();
///     vec_guard.push(10);
///     vec_guard.push(20);
///     assert_eq!(*vec_guard, vec![10, 20]);
/// } // Guard is dropped, vector is cleared, but allocation is kept.
///
/// // Second use: the vector is empty but has retained its capacity.
/// {
///     let mut vec_guard = reusable_vec.recycle();
///     assert!(vec_guard.is_empty());
///     assert!(vec_guard.capacity() >= 2);
///     vec_guard.push(30);
///     assert_eq!(*vec_guard, vec![30]);
/// }
/// ```
#[derive(Debug)]
pub struct ReusableVec<T: 'static> {
    inner: UnsafeCell<Vec<T>>,
}

// The `ReusableVec` is safe to send across threads if `T` is `Send`.
unsafe impl<T: Send> Send for ReusableVec<T> {}

// The `ReusableVec` is safe to share across threads if `T` is `Send`.
// The `recycle` method requires `&mut self`, which prevents concurrent access
// without external synchronization (like a `Mutex`).
unsafe impl<T: Send> Sync for ReusableVec<T> {}

impl<T: 'static> Default for ReusableVec<T> {
    /// Creates a new, empty `ReusableVec` with no allocation.
    fn default() -> Self {
        Self {
            inner: UnsafeCell::new(Vec::new()),
        }
    }
}

/// A RAII guard that provides temporary, exclusive access to a `Vec` from a
/// [`ReusableVec`].
///
/// This guard is created by [`ReusableVec::recycle`]. It allows the underlying
/// `Vec` to be accessed and modified. When the guard goes out of scope, it
/// automatically clears the `Vec`, preserving its allocation for the next cycle.
///
/// The lifetime `'parent` ensures that this guard cannot outlive the
/// [`ReusableVec`] from which it was borrowed.
pub struct ReusableVecGuard<'parent, T1, T2>
where
    T1: 'static,
{
    inner: *mut Vec<T2>,
    _parent: PhantomData<&'parent mut ReusableVec<T1>>,
}

impl<'parent, T1, T2> Deref for ReusableVecGuard<'parent, T1, T2>
where
    T1: 'static,
{
    type Target = Vec<T2>;

    /// Provides immutable access to the underlying `Vec`.
    fn deref(&self) -> &Self::Target {
        // SAFETY: `self.inner` is a valid pointer for the lifetime `'parent`.
        // This is enforced by `_parent` and the `recycle` method signature,
        // which takes `&mut self` on the parent `ReusableVec`.
        unsafe { &*self.inner }
    }
}

impl<'parent, T1, T2> DerefMut for ReusableVecGuard<'parent, T1, T2>
where
    T1: 'static,
{
    /// Provides mutable access to the underlying `Vec`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The same guarantees as `deref` apply. Mutable access is safe
        // because the `&mut self` borrow on the parent `ReusableVec` prevents
        // any other access.
        unsafe { &mut *self.inner }
    }
}

impl<T1> ReusableVec<T1>
where
    T1: 'static,
{
    /// Reuses the `Vec`'s allocation, returning a guard for temporary access.
    ///
    /// This method allows the `Vec`'s element type to be "cast" to a new type `T2`,
    /// as long as the original type `T1` implements [`ReuseCastInto<T2>`].
    ///
    /// The `&mut self` requirement is a key safety feature, as it ensures that
    /// only one guard can be active at any given time.
    pub fn recycle<'parent, T2>(&'parent mut self) -> ReusableVecGuard<'parent, T1, T2>
    where
        T1: ReuseCastInto<T2>,
    {
        // SAFETY: We use `get()` to obtain a raw pointer to the vector.
        // This is safe because `&mut self` guarantees exclusive access.
        let inner_ptr = self.inner.get() as *mut Vec<T2>;

        ReusableVecGuard {
            inner: inner_ptr,
            _parent: PhantomData,
        }
    }
}

impl<'parent, T1, T2> Drop for ReusableVecGuard<'parent, T1, T2>
where
    T1: 'static,
{
    /// Clears the `Vec` when the guard is dropped.
    fn drop(&mut self) {
        // SAFETY: The pointer `self.inner` is guaranteed to be valid because
        // the guard's lifetime is tied to the parent `ReusableVec`.
        // Clearing the vector prepares it for the next reuse cycle.
        unsafe {
            (*self.inner).clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_reuse_works() {
        let mut vec = ReusableVec::<i32>::default();
        {
            let mut r_vec = vec.recycle::<i32>();
            r_vec.push(1);
            r_vec.push(2);
            assert_eq!(r_vec.as_slice(), &[1, 2]);
        }
        let r_vec = unsafe { &*vec.inner.get() };
        assert!(r_vec.is_empty());
        assert!(r_vec.capacity() >= 2);
    }

    #[test]
    fn reference_reuse_works() {
        let mut vec = ReusableVec::<&'static str>::default();
        let capacity;
        {
            let hello = String::from("Hello");
            let world = String::from("World");
            let mut r_vec = vec.recycle();
            r_vec.push(hello.as_str());
            r_vec.push(world.as_str());
            assert_eq!(r_vec.as_slice(), &["Hello", "World"]);
            capacity = r_vec.capacity();
        }
        let r_vec = unsafe { &*vec.inner.get() };
        assert!(r_vec.is_empty());
        assert_eq!(r_vec.capacity(), capacity);
    }

    #[test]
    fn sequential_reuse_works() {
        let mut vec = ReusableVec::<String>::default();
        let mut last_capacity;
        {
            let mut r_vec = vec.recycle::<String>();
            r_vec.push("one".to_string());
            assert_eq!(r_vec.len(), 1);
            last_capacity = r_vec.capacity();
        }

        let r_vec = unsafe { &*vec.inner.get() };
        assert!(r_vec.is_empty());
        assert_eq!(r_vec.capacity(), last_capacity);

        {
            let mut r_vec = vec.recycle::<String>();
            r_vec.push("two".to_string());
            r_vec.push("three".to_string());
            assert_eq!(r_vec.len(), 2);
            last_capacity = r_vec.capacity();
        }

        let r_vec = unsafe { &*vec.inner.get() };
        assert!(r_vec.is_empty());
        assert_eq!(r_vec.capacity(), last_capacity);
    }
}
