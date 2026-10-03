//! This module implements [`Map`] and the capability traits for `slotmap`'s
//! maps.

use std::collections::TryReserveError;

use slotmap::{DenseSlotMap, Key, SlotMap};

use crate::map::{Capacity, Detach, GetDisjointMut, InsertWithKey, Map, Reserve};

// ─── SlotMap ─────────────────────────────────────────────────────────────────

// SAFETY: a `SlotMap` keeps each value under its key until the value is
// removed, and all of its methods agree on which value a key has.
unsafe impl<K: Key, V> Map for SlotMap<K, V> {
    type Key = K;
    type Value = V;
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
    type Keys<'a>
        = slotmap::basic::Keys<'a, K, V>
    where
        Self: 'a,
        V: 'a;
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
    type Drain<'a>
        = slotmap::basic::Drain<'a, K, V>
    where
        Self: 'a;

    #[inline]
    fn empty() -> Self {
        SlotMap::with_key()
    }
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
    #[inline]
    fn clear(&mut self) {
        self.clear();
    }
    #[inline]
    fn contains_key(&self, key: K) -> bool {
        self.contains_key(key)
    }
    #[inline]
    fn get(&self, key: K) -> Option<&V> {
        self.get(key)
    }
    #[inline]
    unsafe fn get_unchecked(&self, key: K) -> &V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { self.get_unchecked(key) }
    }
    #[inline]
    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.get_mut(key)
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, key: K) -> &mut V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { self.get_unchecked_mut(key) }
    }
    #[inline]
    fn remove(&mut self, key: K) -> Option<V> {
        self.remove(key)
    }
    #[inline]
    fn retain<F: FnMut(K, &mut V) -> bool>(&mut self, f: F) {
        self.retain(f);
    }
    #[inline]
    fn insert(&mut self, value: V) -> K {
        self.insert(value)
    }
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        self.keys()
    }
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        self.values()
    }
    #[inline]
    fn values_mut(&mut self) -> Self::ValuesMut<'_> {
        self.values_mut()
    }
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        self.iter()
    }
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.iter_mut()
    }
    #[inline]
    fn drain(&mut self) -> Self::Drain<'_> {
        self.drain()
    }
}

impl<K: Key, V> InsertWithKey for SlotMap<K, V> {
    #[inline]
    fn try_insert_with_key<F, E>(&mut self, f: F) -> Result<K, E>
    where
        F: FnOnce(K) -> Result<V, E>,
    {
        self.try_insert_with_key(f)
    }
}

impl<K: Key, V> GetDisjointMut for SlotMap<K, V> {
    #[inline]
    fn get_disjoint_mut<const N: usize>(&mut self, keys: [K; N]) -> Option<[&mut V; N]> {
        self.get_disjoint_mut(keys)
    }
    #[inline]
    unsafe fn get_disjoint_unchecked_mut<const N: usize>(&mut self, keys: [K; N]) -> [&mut V; N] {
        // SAFETY: the caller promises that every key has a value and that no
        // two keys refer to the same value.
        unsafe { self.get_disjoint_unchecked_mut(keys) }
    }
}

impl<K: Key, V> Detach for SlotMap<K, V> {
    #[inline]
    fn detach(&mut self, key: K) -> Option<V> {
        self.detach(key)
    }

    /// Puts `value` back under `key`.
    ///
    /// # Panics
    ///
    /// Panics if `key` is not detached, because a `SlotMap` cannot report it.
    #[inline]
    fn reattach(&mut self, key: K, value: V) -> Result<(), V> {
        self.reattach(key, value);
        Ok(())
    }
}

impl<K: Key, V> Capacity for SlotMap<K, V> {
    #[inline]
    fn capacity(&self) -> usize {
        self.capacity()
    }
}

impl<K: Key, V> Reserve for SlotMap<K, V> {
    type ReserveError = TryReserveError;

    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        SlotMap::with_capacity_and_key(capacity)
    }
    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
    #[inline]
    fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.try_reserve(additional)
    }
}

// ─── DenseSlotMap ────────────────────────────────────────────────────────────

// SAFETY: a `DenseSlotMap` keeps each value under its key until the value is
// removed, and all of its methods agree on which value a key has.
unsafe impl<K: Key, V> Map for DenseSlotMap<K, V> {
    type Key = K;
    type Value = V;
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
    type Keys<'a>
        = slotmap::dense::Keys<'a, K, V>
    where
        Self: 'a,
        V: 'a;
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
    type Drain<'a>
        = slotmap::dense::Drain<'a, K, V>
    where
        Self: 'a;

    #[inline]
    fn empty() -> Self {
        DenseSlotMap::with_key()
    }
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
    #[inline]
    fn clear(&mut self) {
        self.clear();
    }
    #[inline]
    fn contains_key(&self, key: K) -> bool {
        self.contains_key(key)
    }
    #[inline]
    fn get(&self, key: K) -> Option<&V> {
        self.get(key)
    }
    #[inline]
    unsafe fn get_unchecked(&self, key: K) -> &V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { self.get_unchecked(key) }
    }
    #[inline]
    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.get_mut(key)
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, key: K) -> &mut V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { self.get_unchecked_mut(key) }
    }
    #[inline]
    fn remove(&mut self, key: K) -> Option<V> {
        self.remove(key)
    }
    #[inline]
    fn retain<F: FnMut(K, &mut V) -> bool>(&mut self, f: F) {
        self.retain(f);
    }
    #[inline]
    fn insert(&mut self, value: V) -> K {
        self.insert(value)
    }
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        self.keys()
    }
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        self.values()
    }
    #[inline]
    fn values_mut(&mut self) -> Self::ValuesMut<'_> {
        self.values_mut()
    }
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        self.iter()
    }
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.iter_mut()
    }
    #[inline]
    fn drain(&mut self) -> Self::Drain<'_> {
        self.drain()
    }
}

impl<K: Key, V> InsertWithKey for DenseSlotMap<K, V> {
    #[inline]
    fn try_insert_with_key<F, E>(&mut self, f: F) -> Result<K, E>
    where
        F: FnOnce(K) -> Result<V, E>,
    {
        self.try_insert_with_key(f)
    }
}

impl<K: Key, V> GetDisjointMut for DenseSlotMap<K, V> {
    #[inline]
    fn get_disjoint_mut<const N: usize>(&mut self, keys: [K; N]) -> Option<[&mut V; N]> {
        self.get_disjoint_mut(keys)
    }
    #[inline]
    unsafe fn get_disjoint_unchecked_mut<const N: usize>(&mut self, keys: [K; N]) -> [&mut V; N] {
        // SAFETY: the caller promises that every key has a value and that no
        // two keys refer to the same value.
        unsafe { self.get_disjoint_unchecked_mut(keys) }
    }
}

impl<K: Key, V> Detach for DenseSlotMap<K, V> {
    #[inline]
    fn detach(&mut self, key: K) -> Option<V> {
        self.detach(key)
    }

    /// Puts `value` back under `key`.
    ///
    /// # Panics
    ///
    /// Panics if `key` is not detached, because a `DenseSlotMap` cannot report
    /// it, or if the map is full.
    #[inline]
    fn reattach(&mut self, key: K, value: V) -> Result<(), V> {
        self.reattach(key, value);
        Ok(())
    }
}

impl<K: Key, V> Capacity for DenseSlotMap<K, V> {
    #[inline]
    fn capacity(&self) -> usize {
        self.capacity()
    }
}

impl<K: Key, V> Reserve for DenseSlotMap<K, V> {
    type ReserveError = TryReserveError;

    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        DenseSlotMap::with_capacity_and_key(capacity)
    }
    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
    #[inline]
    fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.try_reserve(additional)
    }
}
