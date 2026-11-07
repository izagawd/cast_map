//! This module implements [`Map`] and the capability traits for
//! `gen_map::GenMap` and `gen_map::DenseGenMap`. [`Empty`](map::Empty) and
//! [`WithCapacity`](map::WithCapacity) need storages that implement
//! `gen_map`'s traits of the same name, and [`Reserve`] needs storages that
//! implement `ReserveStorage` or `ReservePairStorage`. [`Len`](map::Len) needs
//! a `GenMap` config whose `Len` is `KeepLen`.

use gen_map::{
    DenseDrain, DenseGenMap, DenseGenMapConfig, DenseIter, DenseIterMut, DenseKeys, DenseMapSlot,
    DenseStorageError, DenseValues, DenseValuesMut, Drain, GenMap, GenMapConfig, InsertWithError,
    Iter, IterMut, KeepLen, Key, Keys, MapKeyConfig, MapSlot, ReservePairStorage, ReserveStorage,
    StorageError, Values, ValuesMut,
};

use crate::cast_map::CastMap;
use crate::map::{
    self, Capacity, Clear, Detach, GetDisjointMut, InsertWithKey, Iterate, IterateKeys,
    IterateValues, Map, Remove, Reserve, Retain,
};
use crate::unsafe_cast_map::UnsafeCastMap;

/// The key a `GenMap<V, C>` or a `DenseGenMap<V, C>` hands out.
type GenKey<C> = Key<MapKeyConfig<C>>;

// ─── GenMap ──────────────────────────────────────────────────────────────────

// SAFETY: a `GenMap` keeps each value in its slot until the value is taken
// out, and all of its methods agree on which value a key has.
unsafe impl<V, C: GenMapConfig> Map for GenMap<V, C> {
    type Key = GenKey<C>;
    type Value = V;

    #[inline]
    fn contains_key(&self, key: GenKey<C>) -> bool {
        GenMap::contains_key(self, key)
    }
    #[inline]
    fn get(&self, key: GenKey<C>) -> Option<&V> {
        GenMap::get(self, key)
    }
    #[inline]
    unsafe fn get_unchecked(&self, key: GenKey<C>) -> &V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { GenMap::get_unchecked(self, key) }
    }
    #[inline]
    fn get_mut(&mut self, key: GenKey<C>) -> Option<&mut V> {
        GenMap::get_mut(self, key)
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, key: GenKey<C>) -> &mut V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { GenMap::get_unchecked_mut(self, key) }
    }
    /// Inserts a value and returns its key.
    ///
    /// # Panics
    ///
    /// Panics if the `GenMap` is full.
    #[inline]
    fn insert(&mut self, value: V) -> GenKey<C> {
        GenMap::insert(self, value)
    }
}

impl<V, C: GenMapConfig> map::Empty for GenMap<V, C>
where
    <C as GenMapConfig>::Storage<MapSlot<V, C>>: gen_map::Empty,
{
    #[inline]
    fn empty() -> Self {
        GenMap::new_with_config()
    }
}

impl<V, C: GenMapConfig<Len = KeepLen>> map::Len for GenMap<V, C> {
    #[inline]
    fn len(&self) -> usize {
        GenMap::len(self)
    }
    #[inline]
    fn is_empty(&self) -> bool {
        GenMap::is_empty(self)
    }
}

impl<V, C: GenMapConfig> Remove for GenMap<V, C> {
    #[inline]
    fn remove(&mut self, key: GenKey<C>) -> Option<V> {
        GenMap::remove(self, key)
    }
}

impl<V, C: GenMapConfig> Clear for GenMap<V, C> {
    #[inline]
    fn clear(&mut self) {
        GenMap::clear(self);
    }
}

impl<V, C: GenMapConfig> Retain for GenMap<V, C> {
    #[inline]
    fn retain<F: FnMut(GenKey<C>, &mut V) -> bool>(&mut self, f: F) {
        GenMap::retain(self, f);
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
        GenMap::keys(self)
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
        GenMap::values(self)
    }
    #[inline]
    fn values_mut(&mut self) -> ValuesMut<'_, V, C> {
        GenMap::values_mut(self)
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
        GenMap::iter(self)
    }
    #[inline]
    fn iter_mut(&mut self) -> IterMut<'_, V, C> {
        GenMap::iter_mut(self)
    }
}

impl<V, C: GenMapConfig> map::Drain for GenMap<V, C> {
    type Drain<'a>
        = Drain<'a, V, C>
    where
        Self: 'a;

    #[inline]
    fn drain(&mut self) -> Drain<'_, V, C> {
        GenMap::drain(self)
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
        match GenMap::try_insert_with_key(self, f) {
            Ok(key) => Ok(key),
            Err(InsertWithError::Rejected(error)) => Err(error),
            Err(InsertWithError::Full(full)) => panic!("GenMap is full: {full:?}"),
        }
    }
}

impl<V, C: GenMapConfig> GetDisjointMut for GenMap<V, C> {
    #[inline]
    fn get_disjoint_mut<const N: usize>(&mut self, keys: [GenKey<C>; N]) -> Option<[&mut V; N]> {
        GenMap::get_disjoint_mut(self, keys).ok()
    }
    #[inline]
    unsafe fn get_disjoint_unchecked_mut<const N: usize>(
        &mut self,
        keys: [GenKey<C>; N],
    ) -> [&mut V; N] {
        // SAFETY: the caller promises that every key has a value and that no
        // two keys point at the same slot.
        unsafe { GenMap::get_disjoint_mut_unchecked(self, keys) }
    }
}

impl<V, C: GenMapConfig> Detach for GenMap<V, C> {
    #[inline]
    fn detach(&mut self, key: GenKey<C>) -> Option<V> {
        GenMap::detach(self, key)
    }
    #[inline]
    fn attach(&mut self, key: GenKey<C>, value: V) -> Result<(), V> {
        GenMap::attach(self, key, value)
    }
}

impl<V, C: GenMapConfig> Capacity for GenMap<V, C> {
    #[inline]
    fn capacity(&self) -> usize {
        GenMap::capacity(self)
    }
}

impl<V, C: GenMapConfig> map::WithCapacity for GenMap<V, C>
where
    <C as GenMapConfig>::Storage<MapSlot<V, C>>: gen_map::WithCapacity,
{
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        GenMap::with_capacity_and_config(capacity)
    }
}

impl<V, C: GenMapConfig> Reserve for GenMap<V, C>
where
    <C as GenMapConfig>::Storage<MapSlot<V, C>>: ReserveStorage,
{
    type ReserveError = StorageError<V, C>;

    #[inline]
    fn try_reserve(&mut self, additional: usize) -> Result<(), StorageError<V, C>> {
        GenMap::try_reserve(self, additional)
    }
}

impl<V, C: GenMapConfig> From<CastMap<GenMap<V, C>>> for GenMap<V, C> {
    /// Returns the backing map of `map`.
    #[inline]
    fn from(map: CastMap<GenMap<V, C>>) -> Self {
        map.inner.inner
    }
}

impl<V, C: GenMapConfig> From<UnsafeCastMap<GenMap<V, C>>> for GenMap<V, C> {
    /// Returns the backing map of `map`.
    #[inline]
    fn from(map: UnsafeCastMap<GenMap<V, C>>) -> Self {
        map.inner
    }
}

// ─── DenseGenMap ─────────────────────────────────────────────────────────────

// SAFETY: a `DenseGenMap` keeps each value under its key until the value is
// taken out, and all of its methods agree on which value a key has. A value
// that moves to fill a gap keeps its key, and `keys` yields the key of each
// value and no other key.
unsafe impl<V, C: DenseGenMapConfig> Map for DenseGenMap<V, C> {
    type Key = GenKey<C>;
    type Value = V;

    #[inline]
    fn contains_key(&self, key: GenKey<C>) -> bool {
        DenseGenMap::contains_key(self, key)
    }
    #[inline]
    fn get(&self, key: GenKey<C>) -> Option<&V> {
        DenseGenMap::get(self, key)
    }
    #[inline]
    unsafe fn get_unchecked(&self, key: GenKey<C>) -> &V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { DenseGenMap::get_unchecked(self, key) }
    }
    #[inline]
    fn get_mut(&mut self, key: GenKey<C>) -> Option<&mut V> {
        DenseGenMap::get_mut(self, key)
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, key: GenKey<C>) -> &mut V {
        // SAFETY: the caller promises that a value is under `key`.
        unsafe { DenseGenMap::get_unchecked_mut(self, key) }
    }
    /// Inserts a value and returns its key.
    ///
    /// # Panics
    ///
    /// Panics if the `DenseGenMap` is full.
    #[inline]
    fn insert(&mut self, value: V) -> GenKey<C> {
        DenseGenMap::insert(self, value)
    }
}

impl<V, C: DenseGenMapConfig> map::Empty for DenseGenMap<V, C>
where
    <C as DenseGenMapConfig>::SlotStorage<DenseMapSlot<C>>: gen_map::Empty,
    <C as DenseGenMapConfig>::PairStorage<GenKey<C>, V>: gen_map::Empty,
{
    #[inline]
    fn empty() -> Self {
        DenseGenMap::new_with_config()
    }
}

impl<V, C: DenseGenMapConfig> map::Len for DenseGenMap<V, C> {
    #[inline]
    fn len(&self) -> usize {
        DenseGenMap::len(self)
    }
    #[inline]
    fn is_empty(&self) -> bool {
        DenseGenMap::is_empty(self)
    }
}

impl<V, C: DenseGenMapConfig> Remove for DenseGenMap<V, C> {
    #[inline]
    fn remove(&mut self, key: GenKey<C>) -> Option<V> {
        DenseGenMap::remove(self, key)
    }
}

impl<V, C: DenseGenMapConfig> Clear for DenseGenMap<V, C> {
    #[inline]
    fn clear(&mut self) {
        DenseGenMap::clear(self);
    }
}

impl<V, C: DenseGenMapConfig> Retain for DenseGenMap<V, C> {
    #[inline]
    fn retain<F: FnMut(GenKey<C>, &mut V) -> bool>(&mut self, f: F) {
        DenseGenMap::retain(self, f);
    }
}

impl<V, C: DenseGenMapConfig> IterateKeys for DenseGenMap<V, C> {
    type Keys<'a>
        = DenseKeys<'a, MapKeyConfig<C>>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn keys(&self) -> DenseKeys<'_, MapKeyConfig<C>> {
        DenseGenMap::keys(self)
    }
}

impl<V, C: DenseGenMapConfig> IterateValues for DenseGenMap<V, C> {
    type Values<'a>
        = DenseValues<'a, V>
    where
        Self: 'a,
        V: 'a;
    type ValuesMut<'a>
        = DenseValuesMut<'a, V>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn values(&self) -> DenseValues<'_, V> {
        DenseGenMap::values(self)
    }
    #[inline]
    fn values_mut(&mut self) -> DenseValuesMut<'_, V> {
        DenseGenMap::values_mut(self)
    }
}

impl<V, C: DenseGenMapConfig> Iterate for DenseGenMap<V, C> {
    type Iter<'a>
        = DenseIter<'a, V, MapKeyConfig<C>>
    where
        Self: 'a,
        V: 'a;
    type IterMut<'a>
        = DenseIterMut<'a, V, MapKeyConfig<C>>
    where
        Self: 'a,
        V: 'a;

    #[inline]
    fn iter(&self) -> DenseIter<'_, V, MapKeyConfig<C>> {
        DenseGenMap::iter(self)
    }
    #[inline]
    fn iter_mut(&mut self) -> DenseIterMut<'_, V, MapKeyConfig<C>> {
        DenseGenMap::iter_mut(self)
    }
}

impl<V, C: DenseGenMapConfig> map::Drain for DenseGenMap<V, C> {
    type Drain<'a>
        = DenseDrain<'a, V, C>
    where
        Self: 'a;

    #[inline]
    fn drain(&mut self) -> DenseDrain<'_, V, C> {
        DenseGenMap::drain(self)
    }
}

impl<V, C: DenseGenMapConfig> InsertWithKey for DenseGenMap<V, C> {
    /// Calls `f` with the key that the new value will get, and inserts the
    /// value that `f` returns.
    ///
    /// # Panics
    ///
    /// Panics if the `DenseGenMap` is full, before it calls `f`.
    #[inline]
    fn try_insert_with_key<F, E>(&mut self, f: F) -> Result<GenKey<C>, E>
    where
        F: FnOnce(GenKey<C>) -> Result<V, E>,
    {
        match DenseGenMap::try_insert_with_key(self, f) {
            Ok(key) => Ok(key),
            Err(InsertWithError::Rejected(error)) => Err(error),
            Err(InsertWithError::Full(full)) => panic!("DenseGenMap is full: {full:?}"),
        }
    }
}

impl<V, C: DenseGenMapConfig> GetDisjointMut for DenseGenMap<V, C> {
    #[inline]
    fn get_disjoint_mut<const N: usize>(&mut self, keys: [GenKey<C>; N]) -> Option<[&mut V; N]> {
        DenseGenMap::get_disjoint_mut(self, keys).ok()
    }
    #[inline]
    unsafe fn get_disjoint_unchecked_mut<const N: usize>(
        &mut self,
        keys: [GenKey<C>; N],
    ) -> [&mut V; N] {
        // SAFETY: the caller promises that every key has a value and that no
        // two keys point at the same slot.
        unsafe { DenseGenMap::get_disjoint_mut_unchecked(self, keys) }
    }
}

impl<V, C: DenseGenMapConfig> Detach for DenseGenMap<V, C> {
    #[inline]
    fn detach(&mut self, key: GenKey<C>) -> Option<V> {
        DenseGenMap::detach(self, key)
    }

    /// Puts `value` back under `key`.
    ///
    /// # Errors
    ///
    /// Hands `value` back if `key` is not detached, or if the `DenseGenMap`
    /// cannot make room for the value.
    #[inline]
    fn attach(&mut self, key: GenKey<C>, value: V) -> Result<(), V> {
        DenseGenMap::attach(self, key, value)
    }
}

impl<V, C: DenseGenMapConfig> Capacity for DenseGenMap<V, C> {
    #[inline]
    fn capacity(&self) -> usize {
        DenseGenMap::capacity(self)
    }
}

impl<V, C: DenseGenMapConfig> map::WithCapacity for DenseGenMap<V, C>
where
    <C as DenseGenMapConfig>::SlotStorage<DenseMapSlot<C>>: gen_map::WithCapacity,
    <C as DenseGenMapConfig>::PairStorage<GenKey<C>, V>: gen_map::WithCapacity,
{
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        DenseGenMap::with_capacity_and_config(capacity)
    }
}

impl<V, C: DenseGenMapConfig> Reserve for DenseGenMap<V, C>
where
    <C as DenseGenMapConfig>::SlotStorage<DenseMapSlot<C>>: ReserveStorage,
    <C as DenseGenMapConfig>::PairStorage<GenKey<C>, V>: ReservePairStorage,
{
    type ReserveError = DenseStorageError<V, C>;

    #[inline]
    fn try_reserve(&mut self, additional: usize) -> Result<(), DenseStorageError<V, C>> {
        DenseGenMap::try_reserve(self, additional)
    }
}

impl<V, C: DenseGenMapConfig> From<CastMap<DenseGenMap<V, C>>> for DenseGenMap<V, C> {
    /// Returns the backing map of `map`.
    #[inline]
    fn from(map: CastMap<DenseGenMap<V, C>>) -> Self {
        map.inner.inner
    }
}

impl<V, C: DenseGenMapConfig> From<UnsafeCastMap<DenseGenMap<V, C>>> for DenseGenMap<V, C> {
    /// Returns the backing map of `map`.
    #[inline]
    fn from(map: UnsafeCastMap<DenseGenMap<V, C>>) -> Self {
        map.inner
    }
}
