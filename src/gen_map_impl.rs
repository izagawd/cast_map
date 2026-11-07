//! This module implements [`Map`] and the capability traits for
//! `gen_map::GenMap`. [`Reserve`] needs a storage that implements
//! `ReserveStorage`.

use gen_map::{Drain, GenMap, GenMapConfig, GenSlotItem, InsertWithError, Iter, IterMut, Key, Keys, MapKeyConfig, MapSlot, ReserveStorage, StorageError, Values, ValuesMut};
use gen_map::Slot;
use gen_map::KeyConfig;
use gen_map::MapConfig;
use crate::map::{
    self, Capacity, Clear, Detach, GetDisjointMut, InsertWithKey, Iterate, IterateKeys,
    IterateValues, Map, Remove, Reserve, Retain,
};

/// The key a `GenMap<V, C>` hands out.
type GenKey<C> = Key<MapKeyConfig<C>>;

// SAFETY: a `GenMap` keeps each value in its slot until the value is taken
// out, and all of its methods agree on which value a key has.
unsafe impl<V, C: GenMapConfig> Map for GenMap<V, C> {
    type Key = GenKey<C>;
    type Value = V;

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
    /// Inserts a value and returns its key.
    ///
    /// # Panics
    ///
    /// Panics if the `GenMap` is full.
    #[inline]
    fn insert(&mut self, value: V) -> GenKey<C> {
        self.insert(value)
    }
}

impl<V, C: GenMapConfig> Remove for GenMap<V, C> {
    #[inline]
    fn remove(&mut self, key: GenKey<C>) -> Option<V> {
        self.remove(key)
    }
}

impl<V, C: GenMapConfig> Clear for GenMap<V, C> {
    #[inline]
    fn clear(&mut self) {
        self.clear();
    }
}

impl<V, C: GenMapConfig> Retain for GenMap<V, C> {
    #[inline]
    fn retain<F: FnMut(GenKey<C>, &mut V) -> bool>(&mut self, f: F) {
        self.retain(f);
    }
}

impl<V, C: GenMapConfig> IterateKeys for GenMap<V, C> {
    type Keys<'a>
        = Keys<'a, V, C>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn keys(&self) -> Keys<'_, V, C> {
        self.keys()
    }
}

impl<V, C: GenMapConfig> IterateValues for GenMap<V, C> {
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

    #[inline]
    fn values(&self) -> Values<'_, V, C> {
        self.values()
    }
    #[inline]
    fn values_mut(&mut self) -> ValuesMut<'_, V, C> {
        self.values_mut()
    }
}

impl<V, C: GenMapConfig> Iterate for GenMap<V, C> {
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

    #[inline]
    fn iter(&self) -> Iter<'_, V, C> {
        self.iter()
    }
    #[inline]
    fn iter_mut(&mut self) -> IterMut<'_, V, C> {
        self.iter_mut()
    }
}

impl<V, C: GenMapConfig> map::Drain for GenMap<V, C> {
    type Drain<'a>
        = Drain<'a, V, C>
    where
        Self: 'a;

    #[inline]
    fn drain(&mut self) -> Drain<'_, V, C> {
        self.drain()
    }
}

impl<V, C: GenMapConfig> InsertWithKey for GenMap<V, C> {
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

impl<V, C: GenMapConfig> GetDisjointMut for GenMap<V, C> {
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

impl<V, C: GenMapConfig> Detach for GenMap<V, C> {
    #[inline]
    fn detach(&mut self, key: GenKey<C>) -> Option<V> {
        self.detach(key)
    }
    #[inline]
    fn attach(&mut self, key: GenKey<C>, value: V) -> Result<(), V> {
        self.attach(key, value)
    }
}

impl<V, C: GenMapConfig> Capacity for GenMap<V, C> {
    #[inline]
    fn capacity(&self) -> usize {
        self.capacity()
    }
}

impl<V, C: GenMapConfig> Reserve for GenMap<V, C>
where
     <C as GenMapConfig>::Storage<Slot<<<C as MapConfig>::KeyConfig as KeyConfig>::Gen, V, <<C as MapConfig>::KeyConfig as KeyConfig>::Idx>>: ReserveStorage,
{
    type ReserveError = StorageError<V, C>;

    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        GenMap::with_capacity_and_config(capacity)
    }
    #[inline]
    fn try_reserve(&mut self, additional: usize) -> Result<(), StorageError<V, C>> {
        self.try_reserve(additional)
    }
}
