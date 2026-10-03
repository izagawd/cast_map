//! This module implements [`Map`] and the capability traits for
//! `gen_map::GenMap`. [`Reserve`] needs a storage that implements
//! `ReserveStorage`.

use gen_map::{
    Drain, GenMap, GenMapConfig, InsertWithError, Iter, IterMut, Key, Keys, MapConfigFor,
    MapKeyConfig, MapSlot, ReserveStorage, StorageError, Values, ValuesMut,
};

use crate::map::{Capacity, Detach, GetDisjointMut, InsertWithKey, Map, Reserve};

/// The key a `GenMap<V, C>` hands out.
type GenKey<C> = Key<MapKeyConfig<C>>;

// SAFETY: a `GenMap` keeps each value in its slot until the value is taken
// out, and all of its methods agree on which value a key has.
unsafe impl<V, C: MapConfigFor<V>> Map for GenMap<V, C> {
    type Key = GenKey<C>;
    type Value = V;
    type Iter<'a>
        = Iter<'a, V, C>
    where
        Self: 'a,
        V: 'a;
    type IterMut<'a>
        = IterMut<'a, V, C>
    where
        Self: 'a,
        V: 'a;
    type Keys<'a>
        = Keys<'a, V, C>
    where
        Self: 'a,
        V: 'a;
    type Values<'a>
        = Values<'a, V, C>
    where
        Self: 'a,
        V: 'a;
    type ValuesMut<'a>
        = ValuesMut<'a, V, C>
    where
        Self: 'a,
        V: 'a;
    type Drain<'a>
        = Drain<'a, V, C>
    where
        Self: 'a;

    #[inline]
    fn empty() -> Self {
        GenMap::new_with_config()
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
    fn contains_key(&self, key: GenKey<C>) -> bool {
        self.contains_key(key)
    }
    #[inline]
    fn get(&self, key: GenKey<C>) -> Option<&V> {
        self.get(key)
    }
    #[inline]
    unsafe fn get_unchecked(&self, key: GenKey<C>) -> &V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { self.get_unchecked(key) }
    }
    #[inline]
    fn get_mut(&mut self, key: GenKey<C>) -> Option<&mut V> {
        self.get_mut(key)
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, key: GenKey<C>) -> &mut V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { self.get_unchecked_mut(key) }
    }
    #[inline]
    fn remove(&mut self, key: GenKey<C>) -> Option<V> {
        self.remove(key)
    }
    #[inline]
    fn retain<F: FnMut(GenKey<C>, &mut V) -> bool>(&mut self, f: F) {
        self.retain(f);
    }
    /// Inserts a value and returns its key.
    ///
    /// # Panics
    ///
    /// Panics if the `GenMap` is full.
    #[inline]
    fn insert(&mut self, value: V) -> GenKey<C> {
        self.insert(value)
    }
    #[inline]
    fn keys(&self) -> Keys<'_, V, C> {
        self.keys()
    }
    #[inline]
    fn values(&self) -> Values<'_, V, C> {
        self.values()
    }
    #[inline]
    fn values_mut(&mut self) -> ValuesMut<'_, V, C> {
        self.values_mut()
    }
    #[inline]
    fn iter(&self) -> Iter<'_, V, C> {
        self.iter()
    }
    #[inline]
    fn iter_mut(&mut self) -> IterMut<'_, V, C> {
        self.iter_mut()
    }
    #[inline]
    fn drain(&mut self) -> Drain<'_, V, C> {
        self.drain()
    }
}

impl<V, C: MapConfigFor<V>> InsertWithKey for GenMap<V, C> {
    /// Calls `f` with the key that the new value will get, and inserts the
    /// value that `f` returns.
    ///
    /// # Panics
    ///
    /// Panics if the `GenMap` is full, before it calls `f`.
    #[inline]
    fn try_insert_with_key<F, E>(&mut self, f: F) -> Result<GenKey<C>, E>
    where
        F: FnOnce(GenKey<C>) -> Result<V, E>,
    {
        match self.try_insert_with_key(f) {
            Ok(key) => Ok(key),
            Err(InsertWithError::Rejected(error)) => Err(error),
            Err(InsertWithError::Full(full)) => panic!("GenMap is full: {full:?}"),
        }
    }
}

impl<V, C: MapConfigFor<V>> GetDisjointMut for GenMap<V, C> {
    #[inline]
    fn get_disjoint_mut<const N: usize>(&mut self, keys: [GenKey<C>; N]) -> Option<[&mut V; N]> {
        self.get_disjoint_mut(keys).ok()
    }
    #[inline]
    unsafe fn get_disjoint_unchecked_mut<const N: usize>(
        &mut self,
        keys: [GenKey<C>; N],
    ) -> [&mut V; N] {
        // SAFETY: the caller promises that every key has a value and that no
        // two keys point at the same slot.
        unsafe { self.get_disjoint_mut_unchecked(keys) }
    }
}

impl<V, C: MapConfigFor<V>> Detach for GenMap<V, C> {
    #[inline]
    fn detach(&mut self, key: GenKey<C>) -> Option<V> {
        self.detach(key)
    }
    #[inline]
    fn reattach(&mut self, key: GenKey<C>, value: V) -> Result<(), V> {
        self.reattach(key, value)
    }
}

impl<V, C: MapConfigFor<V>> Capacity for GenMap<V, C> {
    #[inline]
    fn capacity(&self) -> usize {
        self.capacity()
    }
}

impl<V, C: MapConfigFor<V>> Reserve for GenMap<V, C>
where
    <C as GenMapConfig<MapSlot<V, C>>>::Storage: ReserveStorage,
{
    type ReserveError = StorageError<V, C>;

    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        GenMap::with_capacity_and_config(capacity)
    }
    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
    #[inline]
    fn try_reserve(&mut self, additional: usize) -> Result<(), StorageError<V, C>> {
        self.try_reserve(additional)
    }
}
