//! [`Map`] is the core trait of a backing map, and each of the other traits
//! here adds a capability. A cast map only has the methods whose capability its
//! backing map implements.
//!
//! - [`Empty`] adds `new` and `Default`.
//! - [`Len`] adds `len` and `is_empty`.
//! - [`Remove`] adds `remove` and `remove_by_inner_key`.
//! - [`Clear`] adds `clear`.
//! - [`Retain`] adds `retain`.
//! - [`IterateKeys`] adds `keys` and `inner_keys`.
//! - [`IterateValues`] adds `values` and `values_mut`.
//! - [`Iterate`] adds `iter` and `iter_mut`, and lets a `for` loop iterate
//!   over a borrowed cast map.
//! - [`Drain`] adds `drain`.
//! - [`InsertWithKey`] adds the `*_with_key` inserts.
//! - [`GetDisjointMut`] adds the `get_disjoint_*` methods.
//! - [`Detach`] adds detaching and attaching.
//! - [`Capacity`] adds `capacity`.
//! - [`WithCapacity`] adds `with_capacity`.
//! - [`Reserve`] adds `try_reserve`.
//! - `IntoIterator` adds `into_iter` to an owned cast map.
//!
//! The `slotmap` maps implement all of them. `GenMap` and `DenseGenMap`
//! implement [`Empty`] and [`WithCapacity`] only when their storages can be
//! created empty or with a capacity, [`Reserve`] only when their storages can
//! grow, and `IntoIterator` only when their storages can be turned into
//! iterators. A `GenMap` implements [`Len`] only when its config's `Len` is
//! `KeepLen`.

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
///   its closure the key that it returns, and a successful `attach` puts the
///   value under the key it was given.
/// - The lookups, `remove` and `detach` find the same value for the same key.
/// - The iterators, `retain` and `IntoIterator` pair each value with its key.
/// - Every key that `keys` yields has a value.
/// - `get_disjoint_mut` returns `Some` only when every key has a value and no
///   two keys refer to the same value, and it keeps the order of the keys.
pub unsafe trait Map: Sized {
    /// The type of the keys the map hands out.
    type Key: Copy;

    /// The type of the values. A cast map stores smart pointers here.
    type Value;

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

    /// Inserts a value and returns its key.
    fn insert(&mut self, value: Self::Value) -> Self::Key;
}

// ─── Capabilities ────────────────────────────────────────────────────────────

/// A backing map that can be created empty.
pub trait Empty: Map {
    /// Creates an empty map.
    fn empty() -> Self;
}

/// A backing map that knows how many values it holds.
pub trait Len: Map {
    /// Returns the number of values in the map.
    fn len(&self) -> usize;

    /// Returns `true` if the map holds no values.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A backing map that can remove a single value.
pub trait Remove: Map {
    /// Removes and returns the value under `key`.
    fn remove(&mut self, key: Self::Key) -> Option<Self::Value>;
}

/// A backing map that can remove every value at once.
pub trait Clear: Map {
    /// Removes every value.
    fn clear(&mut self);
}

/// A backing map that can remove the values that a closure rejects.
pub trait Retain: Map {
    /// Keeps only the values for which `f` returns `true`.
    fn retain<F: FnMut(Self::Key, &mut Self::Value) -> bool>(&mut self, f: F);
}

/// A backing map that can iterate over its keys.
pub trait IterateKeys: Map {
    /// The iterator that [`keys`](Self::keys) returns.
    type Keys<'a>: Iterator<Item = Self::Key>
    where
        Self: 'a,
        Self::Value: 'a;

    /// Iterates over every key.
    fn keys(&self) -> Self::Keys<'_>;
}

/// A backing map that can iterate over its values.
pub trait IterateValues: Map {
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

    /// Iterates over every value.
    fn values(&self) -> Self::Values<'_>;

    /// Iterates over every value mutably.
    fn values_mut(&mut self) -> Self::ValuesMut<'_>;
}

/// A backing map that can iterate over its values together with their keys.
pub trait Iterate: Map {
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

    /// Iterates over every value together with its key.
    fn iter(&self) -> Self::Iter<'_>;

    /// Iterates over every value mutably together with its key.
    fn iter_mut(&mut self) -> Self::IterMut<'_>;
}

/// A backing map that can take out every value together with its key.
pub trait Drain: Map {
    /// The iterator that [`drain`](Self::drain) returns.
    type Drain<'a>: Iterator<Item = (Self::Key, Self::Value)>
    where
        Self: 'a;

    /// Removes every value, yielding each with its key.
    fn drain(&mut self) -> Self::Drain<'_>;
}

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
/// reserved for [`attach`](Self::attach).
pub trait Detach: Map {
    /// Removes and returns the value under `key`, and keeps `key` reserved
    /// until [`attach`](Self::attach) puts a value back under it.
    fn detach(&mut self, key: Self::Key) -> Option<Self::Value>;

    /// Puts `value` back under a key whose value [`detach`](Self::detach) took
    /// out.
    ///
    /// # Errors
    ///
    /// Hands `value` back if `key` is not detached. A map that cannot tell may
    /// panic instead, as the `slotmap` maps do.
    fn attach(&mut self, key: Self::Key, value: Self::Value) -> Result<(), Self::Value>;
}

/// A backing map that reports its capacity.
pub trait Capacity: Map {
    /// How many values the map can hold before it has to grow, or in total if
    /// it cannot grow.
    fn capacity(&self) -> usize;
}

/// A backing map that can be created with room for a given number of values.
pub trait WithCapacity: Map {
    /// Creates an empty map with room for at least `capacity` values.
    fn with_capacity(capacity: usize) -> Self;
}

/// A backing map that can make room for more values ahead of time.
pub trait Reserve: Capacity {
    /// Why [`try_reserve`](Self::try_reserve) could not make the room.
    type ReserveError;

    /// Makes room for at least `additional` more values.
    ///
    /// # Errors
    ///
    /// Returns an error if the room cannot be made.
    fn try_reserve(&mut self, additional: usize) -> Result<(), Self::ReserveError>;
}
