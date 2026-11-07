//! This module implements [`Map`] and the capability traits for `slotmap`'s
//! maps.

use std::collections::TryReserveError;

use slotmap::{DenseSlotMap, Key, SlotMap};

use crate::cast_map::CastMap;
use crate::map::{
    Capacity, Clear, Detach, Drain, Empty, GetDisjointMut, InsertWithKey, Iterate, IterateKeys,
    IterateValues, Len, Map, Remove, Reserve, Retain, WithCapacity,
};
use crate::unsafe_cast_map::UnsafeCastMap;

// ─── SlotMap ─────────────────────────────────────────────────────────────────

// SAFETY: a `SlotMap` keeps each value under its key until the value is
// removed, and all of its methods agree on which value a key has.
unsafe impl<K: Key, V> Map for SlotMap<K, V> {
    type Key = K;
    type Value = V;

    #[inline]
    fn contains_key(&self, key: K) -> bool {
        SlotMap::contains_key(self, key)
    }
    #[inline]
    fn get(&self, key: K) -> Option<&V> {
        SlotMap::get(self, key)
    }
    #[inline]
    unsafe fn get_unchecked(&self, key: K) -> &V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { SlotMap::get_unchecked(self, key) }
    }
    #[inline]
    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        SlotMap::get_mut(self, key)
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, key: K) -> &mut V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { SlotMap::get_unchecked_mut(self, key) }
    }
    #[inline]
    fn insert(&mut self, value: V) -> K {
        SlotMap::insert(self, value)
    }
}

impl<K: Key, V> Empty for SlotMap<K, V> {
    #[inline]
    fn empty() -> Self {
        SlotMap::with_key()
    }
}

impl<K: Key, V> Len for SlotMap<K, V> {
    #[inline]
    fn len(&self) -> usize {
        SlotMap::len(self)
    }
    #[inline]
    fn is_empty(&self) -> bool {
        SlotMap::is_empty(self)
    }
}

impl<K: Key, V> Remove for SlotMap<K, V> {
    #[inline]
    fn remove(&mut self, key: K) -> Option<V> {
        SlotMap::remove(self, key)
    }
}

impl<K: Key, V> Clear for SlotMap<K, V> {
    #[inline]
    fn clear(&mut self) {
        SlotMap::clear(self);
    }
}

impl<K: Key, V> Retain for SlotMap<K, V> {
    #[inline]
    fn retain<F: FnMut(K, &mut V) -> bool>(&mut self, f: F) {
        SlotMap::retain(self, f);
    }
}

impl<K: Key, V> IterateKeys for SlotMap<K, V> {
    type Keys<'a>
        = slotmap::basic::Keys<'a, K, V>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        SlotMap::keys(self)
    }
}

impl<K: Key, V> IterateValues for SlotMap<K, V> {
    type Values<'a>
        = slotmap::basic::Values<'a, K, V>
    where
        Self: 'a,
        V: 'a;
    type ValuesMut<'a>
        = slotmap::basic::ValuesMut<'a, K, V>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn values(&self) -> Self::Values<'_> {
        SlotMap::values(self)
    }
    #[inline]
    fn values_mut(&mut self) -> Self::ValuesMut<'_> {
        SlotMap::values_mut(self)
    }
}

impl<K: Key, V> Iterate for SlotMap<K, V> {
    type Iter<'a>
        = slotmap::basic::Iter<'a, K, V>
    where
        Self: 'a,
        V: 'a;
    type IterMut<'a>
        = slotmap::basic::IterMut<'a, K, V>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        SlotMap::iter(self)
    }
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        SlotMap::iter_mut(self)
    }
}

impl<K: Key, V> Drain for SlotMap<K, V> {
    type Drain<'a>
        = slotmap::basic::Drain<'a, K, V>
    where
        Self: 'a;

    #[inline]
    fn drain(&mut self) -> Self::Drain<'_> {
        SlotMap::drain(self)
    }
}

impl<K: Key, V> InsertWithKey for SlotMap<K, V> {
    #[inline]
    fn try_insert_with_key<F, E>(&mut self, f: F) -> Result<K, E>
    where
        F: FnOnce(K) -> Result<V, E>,
    {
        SlotMap::try_insert_with_key(self, f)
    }
}

impl<K: Key, V> GetDisjointMut for SlotMap<K, V> {
    #[inline]
    fn get_disjoint_mut<const N: usize>(&mut self, keys: [K; N]) -> Option<[&mut V; N]> {
        SlotMap::get_disjoint_mut(self, keys)
    }
    #[inline]
    unsafe fn get_disjoint_unchecked_mut<const N: usize>(&mut self, keys: [K; N]) -> [&mut V; N] {
        // SAFETY: the caller promises that every key has a value and that no
        // two keys refer to the same value.
        unsafe { SlotMap::get_disjoint_unchecked_mut(self, keys) }
    }
}

impl<K: Key, V> Detach for SlotMap<K, V> {
    #[inline]
    fn detach(&mut self, key: K) -> Option<V> {
        SlotMap::detach(self, key)
    }

    /// Puts `value` back under `key`.
    ///
    /// # Panics
    ///
    /// Panics if `key` is not detached, because a `SlotMap` cannot report it.
    #[inline]
    fn attach(&mut self, key: K, value: V) -> Result<(), V> {
        SlotMap::reattach(self, key, value);
        Ok(())
    }
}

impl<K: Key, V> Capacity for SlotMap<K, V> {
    #[inline]
    fn capacity(&self) -> usize {
        SlotMap::capacity(self)
    }
}

impl<K: Key, V> WithCapacity for SlotMap<K, V> {
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        SlotMap::with_capacity_and_key(capacity)
    }
}

impl<K: Key, V> Reserve for SlotMap<K, V> {
    type ReserveError = TryReserveError;

    #[inline]
    fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        SlotMap::try_reserve(self, additional)
    }
}

impl<K: Key, V> From<CastMap<SlotMap<K, V>>> for SlotMap<K, V> {
    /// Returns the backing map of `map`.
    #[inline]
    fn from(map: CastMap<SlotMap<K, V>>) -> Self {
        map.inner.inner
    }
}

impl<K: Key, V> From<UnsafeCastMap<SlotMap<K, V>>> for SlotMap<K, V> {
    /// Returns the backing map of `map`.
    #[inline]
    fn from(map: UnsafeCastMap<SlotMap<K, V>>) -> Self {
        map.inner
    }
}

// ─── DenseSlotMap ────────────────────────────────────────────────────────────

// SAFETY: a `DenseSlotMap` keeps each value under its key until the value is
// removed, and all of its methods agree on which value a key has.
unsafe impl<K: Key, V> Map for DenseSlotMap<K, V> {
    type Key = K;
    type Value = V;

    #[inline]
    fn contains_key(&self, key: K) -> bool {
        DenseSlotMap::contains_key(self, key)
    }
    #[inline]
    fn get(&self, key: K) -> Option<&V> {
        DenseSlotMap::get(self, key)
    }
    #[inline]
    unsafe fn get_unchecked(&self, key: K) -> &V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { DenseSlotMap::get_unchecked(self, key) }
    }
    #[inline]
    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        DenseSlotMap::get_mut(self, key)
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, key: K) -> &mut V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { DenseSlotMap::get_unchecked_mut(self, key) }
    }
    #[inline]
    fn insert(&mut self, value: V) -> K {
        DenseSlotMap::insert(self, value)
    }
}

impl<K: Key, V> Empty for DenseSlotMap<K, V> {
    #[inline]
    fn empty() -> Self {
        DenseSlotMap::with_key()
    }
}

impl<K: Key, V> Len for DenseSlotMap<K, V> {
    #[inline]
    fn len(&self) -> usize {
        DenseSlotMap::len(self)
    }
    #[inline]
    fn is_empty(&self) -> bool {
        DenseSlotMap::is_empty(self)
    }
}

impl<K: Key, V> Remove for DenseSlotMap<K, V> {
    #[inline]
    fn remove(&mut self, key: K) -> Option<V> {
        DenseSlotMap::remove(self, key)
    }
}

impl<K: Key, V> Clear for DenseSlotMap<K, V> {
    #[inline]
    fn clear(&mut self) {
        DenseSlotMap::clear(self);
    }
}

impl<K: Key, V> Retain for DenseSlotMap<K, V> {
    #[inline]
    fn retain<F: FnMut(K, &mut V) -> bool>(&mut self, f: F) {
        DenseSlotMap::retain(self, f);
    }
}

impl<K: Key, V> IterateKeys for DenseSlotMap<K, V> {
    type Keys<'a>
        = slotmap::dense::Keys<'a, K, V>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        DenseSlotMap::keys(self)
    }
}

impl<K: Key, V> IterateValues for DenseSlotMap<K, V> {
    type Values<'a>
        = slotmap::dense::Values<'a, K, V>
    where
        Self: 'a,
        V: 'a;
    type ValuesMut<'a>
        = slotmap::dense::ValuesMut<'a, K, V>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn values(&self) -> Self::Values<'_> {
        DenseSlotMap::values(self)
    }
    #[inline]
    fn values_mut(&mut self) -> Self::ValuesMut<'_> {
        DenseSlotMap::values_mut(self)
    }
}

impl<K: Key, V> Iterate for DenseSlotMap<K, V> {
    type Iter<'a>
        = slotmap::dense::Iter<'a, K, V>
    where
        Self: 'a,
        V: 'a;
    type IterMut<'a>
        = slotmap::dense::IterMut<'a, K, V>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        DenseSlotMap::iter(self)
    }
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        DenseSlotMap::iter_mut(self)
    }
}

impl<K: Key, V> Drain for DenseSlotMap<K, V> {
    type Drain<'a>
        = slotmap::dense::Drain<'a, K, V>
    where
        Self: 'a;

    #[inline]
    fn drain(&mut self) -> Self::Drain<'_> {
        DenseSlotMap::drain(self)
    }
}

impl<K: Key, V> InsertWithKey for DenseSlotMap<K, V> {
    #[inline]
    fn try_insert_with_key<F, E>(&mut self, f: F) -> Result<K, E>
    where
        F: FnOnce(K) -> Result<V, E>,
    {
        DenseSlotMap::try_insert_with_key(self, f)
    }
}

impl<K: Key, V> GetDisjointMut for DenseSlotMap<K, V> {
    #[inline]
    fn get_disjoint_mut<const N: usize>(&mut self, keys: [K; N]) -> Option<[&mut V; N]> {
        DenseSlotMap::get_disjoint_mut(self, keys)
    }
    #[inline]
    unsafe fn get_disjoint_unchecked_mut<const N: usize>(&mut self, keys: [K; N]) -> [&mut V; N] {
        // SAFETY: the caller promises that every key has a value and that no
        // two keys refer to the same value.
        unsafe { DenseSlotMap::get_disjoint_unchecked_mut(self, keys) }
    }
}

impl<K: Key, V> Detach for DenseSlotMap<K, V> {
    #[inline]
    fn detach(&mut self, key: K) -> Option<V> {
        DenseSlotMap::detach(self, key)
    }

    /// Puts `value` back under `key`.
    ///
    /// # Panics
    ///
    /// Panics if `key` is not detached, because a `DenseSlotMap` cannot report
    /// it, or if the map is full.
    #[inline]
    fn attach(&mut self, key: K, value: V) -> Result<(), V> {
        DenseSlotMap::reattach(self, key, value);
        Ok(())
    }
}

impl<K: Key, V> Capacity for DenseSlotMap<K, V> {
    #[inline]
    fn capacity(&self) -> usize {
        DenseSlotMap::capacity(self)
    }
}

impl<K: Key, V> WithCapacity for DenseSlotMap<K, V> {
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        DenseSlotMap::with_capacity_and_key(capacity)
    }
}

impl<K: Key, V> Reserve for DenseSlotMap<K, V> {
    type ReserveError = TryReserveError;

    #[inline]
    fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        DenseSlotMap::try_reserve(self, additional)
    }
}

impl<K: Key, V> From<CastMap<DenseSlotMap<K, V>>> for DenseSlotMap<K, V> {
    /// Returns the backing map of `map`.
    #[inline]
    fn from(map: CastMap<DenseSlotMap<K, V>>) -> Self {
        map.inner.inner
    }
}

impl<K: Key, V> From<UnsafeCastMap<DenseSlotMap<K, V>>> for DenseSlotMap<K, V> {
    /// Returns the backing map of `map`.
    #[inline]
    fn from(map: UnsafeCastMap<DenseSlotMap<K, V>>) -> Self {
        map.inner
    }
}
