//! [`Map`] is the core trait of a backing map, and each of the other traits
//! here adds a capability. A cast map only has the methods whose capability its
//! backing map implements.
//!
//! - [`InsertWithKey`] adds the `*_with_key` inserts.
//! - [`GetDisjointMut`] adds the `get_disjoint_*` methods.
//! - [`Detach`] adds detaching and reattaching.
//! - [`Capacity`] adds `capacity`.
//! - [`Reserve`] adds `with_capacity`, `reserve` and `try_reserve`.
//! - `IntoIterator` adds `into_iter` to an owned cast map.
//!
//! The `slotmap` maps implement all of them. A `GenMap` implements [`Reserve`]
//! only when its storage can grow, and `IntoIterator` only when its storage
//! does.

use std::ops::Deref;

/// The type that a map `M` stores behind each of its smart pointers.
pub(crate) type MTarget<M> = <<M as Map>::Value as Deref>::Target;

/// The core of a backing map. It hands out a key for each value it inserts,
/// and the key can be any `Copy` type, such as a `usize` index into a `Vec`.
///
/// # Safety
///
/// The cast maps rebuild typed references from the metadata that a key caches,
/// so the map must track its values the way a map does. These rules cover the
/// methods of this trait and of the capability traits.
///
/// - A value stays under the key it was inserted with until a method takes it
///   out, and no method moves it to another key or replaces it.
/// - `insert` returns the key of the new value, `try_insert_with_key` passes
///   its closure the key that it returns, and a successful `reattach` puts the
///   value under the key it was given.
/// - The lookups, `remove` and `detach` find the same value for the same key.
/// - The iterators, `retain` and `IntoIterator` pair each value with its key.
/// - `get_disjoint_mut` returns `Some` only when every key has a value and no
///   two keys refer to the same value, and it keeps the order of the keys.
pub unsafe trait Map: Sized {
    /// The type of the keys the map hands out.
    type Key: Copy;

    /// The type of the values. A cast map stores smart pointers here.
    type Value;

    /// The iterator that [`iter`](Self::iter) returns.
    type Iter<'a>: Iterator<Item = (Self::Key, &'a Self::Value)>
    where
        Self: 'a,
        Self::Value: 'a;

    /// The iterator that [`iter_mut`](Self::iter_mut) returns.
    type IterMut<'a>: Iterator<Item = (Self::Key, &'a mut Self::Value)>
    where
        Self: 'a,
        Self::Value: 'a;

    /// The iterator that [`keys`](Self::keys) returns.
    type Keys<'a>: Iterator<Item = Self::Key>
    where
        Self: 'a,
        Self::Value: 'a;

    /// The iterator that [`values`](Self::values) returns.
    type Values<'a>: Iterator<Item = &'a Self::Value>
    where
        Self: 'a,
        Self::Value: 'a;

    /// The iterator that [`values_mut`](Self::values_mut) returns.
    type ValuesMut<'a>: Iterator<Item = &'a mut Self::Value>
    where
        Self: 'a,
        Self::Value: 'a;

    /// The iterator that [`drain`](Self::drain) returns.
    type Drain<'a>: Iterator<Item = (Self::Key, Self::Value)>
    where
        Self: 'a;

    /// Creates an empty map.
    fn empty() -> Self;

    /// Returns the number of values in the map.
    fn len(&self) -> usize;

    /// Returns `true` if the map holds no values.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Removes every value.
    fn clear(&mut self);

    /// Returns `true` if a value is under `key`.
    #[inline]
    fn contains_key(&self, key: Self::Key) -> bool {
        self.get(key).is_some()
    }

    /// Returns a reference to the value under `key`.
    fn get(&self, key: Self::Key) -> Option<&Self::Value>;

    /// Like [`get`](Self::get), but without checking that a value is under
    /// `key`.
    ///
    /// # Safety
    ///
    /// A value must be under `key`.
    #[inline]
    unsafe fn get_unchecked(&self, key: Self::Key) -> &Self::Value {
        // SAFETY: the caller promises that a value is under `key`, so `get`
        // returns `Some`.
        unsafe { self.get(key).unwrap_unchecked() }
    }

    /// Returns a mutable reference to the value under `key`.
    fn get_mut(&mut self, key: Self::Key) -> Option<&mut Self::Value>;

    /// Like [`get_mut`](Self::get_mut), but without checking that a value is
    /// under `key`.
    ///
    /// # Safety
    ///
    /// A value must be under `key`.
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, key: Self::Key) -> &mut Self::Value {
        // SAFETY: the caller promises that a value is under `key`, so
        // `get_mut` returns `Some`.
        unsafe { self.get_mut(key).unwrap_unchecked() }
    }

    /// Removes and returns the value under `key`.
    fn remove(&mut self, key: Self::Key) -> Option<Self::Value>;

    /// Keeps only the values for which `f` returns `true`.
    fn retain<F: FnMut(Self::Key, &mut Self::Value) -> bool>(&mut self, f: F);

    /// Inserts a value and returns its key.
    fn insert(&mut self, value: Self::Value) -> Self::Key;

    /// Iterates over every key.
    fn keys(&self) -> Self::Keys<'_>;

    /// Iterates over every value.
    fn values(&self) -> Self::Values<'_>;

    /// Iterates over every value mutably.
    fn values_mut(&mut self) -> Self::ValuesMut<'_>;

    /// Iterates over every value together with its key.
    fn iter(&self) -> Self::Iter<'_>;

    /// Iterates over every value mutably together with its key.
    fn iter_mut(&mut self) -> Self::IterMut<'_>;

    /// Removes every value, yielding each with its key.
    fn drain(&mut self) -> Self::Drain<'_>;
}

// ─── Capabilities ────────────────────────────────────────────────────────────

/// A backing map that passes the new value's key to the closure that builds the
/// value. The `sized` and `as` forms of the `*_with_key` inserts also need the
/// `coerce_unsized` feature.
pub trait InsertWithKey: Map {
    /// Calls `f` with the key that the new value will get, and inserts the
    /// value that `f` returns. If `f` fails, nothing is inserted.
    fn try_insert_with_key<F, E>(&mut self, f: F) -> Result<Self::Key, E>
    where
        F: FnOnce(Self::Key) -> Result<Self::Value, E>;
}

/// A backing map that hands out mutable references to several values at once.
pub trait GetDisjointMut: Map {
    /// Returns a mutable reference to the value under each key, or `None` if a
    /// key has no value or two keys refer to the same value.
    fn get_disjoint_mut<const N: usize>(
        &mut self,
        keys: [Self::Key; N],
    ) -> Option<[&mut Self::Value; N]>;

    /// Like [`get_disjoint_mut`](Self::get_disjoint_mut), but without its
    /// checks.
    ///
    /// # Safety
    ///
    /// A value must be under every key, and no two keys may refer to the same
    /// value.
    #[inline]
    unsafe fn get_disjoint_unchecked_mut<const N: usize>(
        &mut self,
        keys: [Self::Key; N],
    ) -> [&mut Self::Value; N] {
        // SAFETY: the caller promises that every key has a value and that no
        // two keys refer to the same value, so `get_disjoint_mut` returns
        // `Some`.
        unsafe { self.get_disjoint_mut(keys).unwrap_unchecked() }
    }
}

/// A backing map that can take a value out while it keeps the value's key
/// reserved for [`reattach`](Self::reattach).
pub trait Detach: Map {
    /// Removes and returns the value under `key`, and keeps `key` reserved
    /// until [`reattach`](Self::reattach) puts a value back under it.
    fn detach(&mut self, key: Self::Key) -> Option<Self::Value>;

    /// Puts `value` back under a key whose value [`detach`](Self::detach) took
    /// out.
    ///
    /// # Errors
    ///
    /// Hands `value` back if `key` is not detached. A map that cannot tell may
    /// panic instead, as the `slotmap` maps do.
    fn reattach(&mut self, key: Self::Key, value: Self::Value) -> Result<(), Self::Value>;
}

/// A backing map that reports its capacity.
pub trait Capacity: Map {
    /// How many values the map can hold before it has to grow, or in total if
    /// it cannot grow.
    fn capacity(&self) -> usize;
}

/// A backing map that can make room for more values ahead of time.
pub trait Reserve: Capacity {
    /// Why [`try_reserve`](Self::try_reserve) could not make the room.
    type ReserveError;

    /// Creates an empty map with room for at least `capacity` values.
    fn with_capacity(capacity: usize) -> Self;

    /// Makes room for at least `additional` more values.
    ///
    /// # Panics
    ///
    /// Panics or aborts if the room cannot be made, as `Vec::reserve` does.
    fn reserve(&mut self, additional: usize);

    /// The fallible form of [`reserve`](Self::reserve).
    fn try_reserve(&mut self, additional: usize) -> Result<(), Self::ReserveError>;
}
