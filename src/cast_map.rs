//! Safe wrapper around [`UnsafeCastMapG`](crate::unsafe_cast_map::UnsafeCastMapG)
//! whose keyed lookups are checked against each slot's stored concrete type id.
//!
//! Where the low-level map's `get` / `get_mut` / `remove` are `unsafe` (the
//! caller must promise the key's metadata fits the slot), [`CastMapG`] makes
//! them safe: every value sits behind a stored pointer that records its
//! concrete [`TypeId`] (see [`ConcreteTypeId`]), and a lookup recovers the type id
//! implied by the key's metadata (via [`type_id_from_metadata`]) and compares it to
//! the slot's. A mismatch — wrong type or recycled slot — returns `None`
//! instead of risking UB.
//!
//! The backing map finds the value, and the type id check proves that the
//! key's metadata fits it, so rebuilding `&T` is sound. The [`Map`] contract
//! makes the methods that check the type and the methods that hand out the
//! value find the same value.
//!
//! The checked lookups need `M::Value: ConcreteTypeId`, which [`TypeTaggedBox`]
//! meets. A plain `Box` does not, because `type_id` on a `Box<dyn Foo>` reports
//! `dyn Foo` unless `Foo` is an `Any` subtrait. Plain `Box` works with
//! [`UnsafeCastMapG`].
//!
//! On the key side, lookups require `T: AnyHaver`: sized types always qualify;
//! trait objects qualify when the trait declares `AnyHaver` as a supertrait.
//! `dyn Any` does not, so `get::<dyn Any>` is a compile error — use
//! [`downcast_key`](CastMapG::downcast_key) or
//! [`get_by_inner_key`](CastMapG::get_by_inner_key) for erased access.
//!
//! [`CastMapG`] is generic over its backing map `M`. The aliases at the end of
//! this module pick `M` for the `slotmap` and `gen_map` features.
use std::any::TypeId;
#[cfg(feature = "coerce_unsized")]
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::Pointee;

#[cfg(feature = "gen_map")]
use gen_map::{DefaultMapConfig, GenMap};
#[cfg(feature = "slotmap")]
use slotmap::{DenseSlotMap, SlotMap};
use stable_deref_trait::StableDeref;

use crate::any_haver::{type_id_from_metadata, AnyHaver};
use crate::cast_key::CastKey;
use crate::map::{Capacity, Detach, GetDisjointMut, InsertWithKey, MTarget, Map, Reserve};
use crate::retype_ptr::RetypePtr;
use crate::type_tagged_ptr::ConcreteTypeId;
#[cfg(any(doc, feature = "slotmap", feature = "gen_map"))]
use crate::type_tagged_ptr::TypeTaggedBox;
use crate::unsafe_cast_map::{self, UnsafeCastMapG};

// ─── CastMapG ────────────────────────────────────────────────────────────────

/// A safe wrapper around [`UnsafeCastMapG`] that validates keyed lookups
/// against each slot's stored concrete [`TypeId`].
///
/// `M` is the backing map, and its values are smart pointers such as
/// `TypeTaggedBox<dyn Any>`.
pub struct CastMapG<M> {
    inner: UnsafeCastMapG<M>,
}

// ─── Clone ───────────────────────────────────────────────────────────────────

impl<M> Clone for CastMapG<M>
where
    M: Map + Clone,
{
    /// Clones the map. Keys of the original work on the clone when the backing
    /// map keeps its keys in its clones, as the `slotmap` maps and `GenMap` do.
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.inner.clone_from(&source.inner);
    }
}

impl<M> Default for CastMapG<M>
where
    M: Map,
    M::Value: StableDeref,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ─── Basic methods ───────────────────────────────────────────────────────────

impl<M> CastMapG<M>
where
    M: Map,
    M::Value: StableDeref,
{
    /// Creates a new, empty map.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: UnsafeCastMapG::new(),
        }
    }

    /// Creates a new map with the given pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self
    where
        M: Reserve,
    {
        Self {
            inner: UnsafeCastMapG::with_capacity(capacity),
        }
    }

    /// Returns true if the map is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of occupied elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns how many slots the backing storage can hold before reallocating.
    #[inline]
    pub fn capacity(&self) -> usize
    where
        M: Capacity,
    {
        self.inner.capacity()
    }

    /// Reserves capacity for at least `additional` more elements.
    #[inline]
    pub fn reserve(&mut self, additional: usize)
    where
        M: Reserve,
    {
        self.inner.reserve(additional);
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), M::ReserveError>
    where
        M: Reserve,
    {
        self.inner.try_reserve(additional)
    }

    /// Removes every value from the map.
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    // ── inner accessors ───────────────────────────────────────────────────

    /// Consumes this map and returns the underlying [`UnsafeCastMapG`].
    #[inline]
    pub fn inner(self) -> UnsafeCastMapG<M> {
        self.inner
    }

    /// Returns a shared reference to the underlying [`UnsafeCastMapG`].
    #[inline]
    pub fn inner_ref(&self) -> &UnsafeCastMapG<M> {
        &self.inner
    }

    /// Returns a mutable reference to the underlying [`UnsafeCastMapG`].
    #[inline]
    pub fn inner_mut(&mut self) -> &mut UnsafeCastMapG<M> {
        &mut self.inner
    }

    // ── backing-key access (no type check needed: output-typed) ────────────

    /// Returns a shared reference to the value under a backing key.
    #[inline]
    pub fn get_by_inner_key(&self, key: M::Key) -> Option<&MTarget<M>> {
        self.inner.get_by_inner_key(key)
    }

    /// Removes the value under a backing key and returns its pointer.
    #[inline]
    pub fn remove_by_inner_key(&mut self, key: M::Key) -> Option<M::Value> {
        self.inner.remove_by_inner_key(key)
    }

    /// Removes and returns the value under a backing key, and keeps the key
    /// reserved for [`reattach_by_inner_key`](Self::reattach_by_inner_key).
    #[inline]
    pub fn detach_by_inner_key(&mut self, key: M::Key) -> Option<M::Value>
    where
        M: Detach,
    {
        self.inner.detach_by_inner_key(key)
    }

    /// Puts `value` back under a key whose value was detached.
    ///
    /// # Errors
    /// Hands `value` back if `key` is not detached. The `slotmap` maps panic
    /// instead.
    #[inline]
    pub fn reattach_by_inner_key(&mut self, key: M::Key, value: M::Value) -> Result<(), M::Value>
    where
        M: Detach,
    {
        self.inner.reattach_by_inner_key(key, value)
    }

    /// Shared iterator over output references only.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &MTarget<M>> + '_ {
        self.inner.values()
    }
}

// ── backing-key access requiring `&mut Output` ───────────────────────────────

impl<M> CastMapG<M>
where
    M: Map,
    M::Value: StableDeref + DerefMut,
{
    /// Returns a mutable reference to the value under a backing key.
    #[inline]
    pub fn get_by_inner_key_mut(&mut self, key: M::Key) -> Option<&mut MTarget<M>> {
        self.inner.get_by_inner_key_mut(key)
    }

    /// Mutable iterator over output references only.
    #[inline]
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut MTarget<M>> + '_ {
        self.inner.values_mut()
    }

    /// Returns a mutable reference to the value under each backing key, or
    /// `None` if a key has no value or two of the keys refer to the same
    /// value.
    #[inline]
    pub fn get_disjoint_mut_by_inner_key<const N: usize>(
        &mut self,
        keys: [M::Key; N],
    ) -> Option<[&mut MTarget<M>; N]>
    where
        M: GetDisjointMut,
    {
        self.inner.get_disjoint_mut_by_inner_key(keys)
    }

    /// Like [`get_disjoint_mut_by_inner_key`](Self::get_disjoint_mut_by_inner_key)
    /// but without validity or uniqueness checks.
    ///
    /// # Safety
    /// Every key must address a live slot, and no two keys may alias one slot.
    #[inline]
    pub unsafe fn get_disjoint_unchecked_mut_by_inner_key<const N: usize>(
        &mut self,
        keys: [M::Key; N],
    ) -> [&mut MTarget<M>; N]
    where
        M: GetDisjointMut,
    {
        self.inner.get_disjoint_unchecked_mut_by_inner_key(keys)
    }
}

// ─── Core operations (require pointer metadata) ──────────────────────────────

impl<M> CastMapG<M>
where
    M: Map,
    M::Value: StableDeref,
    MTarget<M>: Pointee,
    <MTarget<M> as Pointee>::Metadata: Copy,
{
    // ── insert ───────────────────────────────────────────────────────────

    /// Inserts a value and returns its [`CastKey`].
    #[inline]
    pub fn insert(&mut self, value: M::Value) -> CastKey<MTarget<M>, M::Key> {
        self.inner.insert(value)
    }

    /// Inserts a value produced by `func`, which receives the backing key.
    #[inline]
    pub fn insert_with_key(
        &mut self,
        func: impl FnOnce(M::Key) -> M::Value,
    ) -> CastKey<MTarget<M>, M::Key>
    where
        M: InsertWithKey,
    {
        self.inner.insert_with_key(func)
    }

    /// Like [`insert_with_key`](Self::insert_with_key) but the closure may fail.
    #[inline]
    pub fn try_insert_with_key<E>(
        &mut self,
        func: impl FnOnce(M::Key) -> Result<M::Value, E>,
    ) -> Result<CastKey<MTarget<M>, M::Key>, E>
    where
        M: InsertWithKey,
    {
        self.inner.try_insert_with_key(func)
    }

    // ── insert_sized ─────────────────────────────────────────────────────

    /// Inserts a concrete-typed smart pointer (coerced into `M::Value` on the
    /// way in), returning a [`CastKey`] typed to the concrete
    /// `ConcretePtr::Target` rather than the map's output type.
    ///
    /// This method needs the `coerce_unsized` feature.
    #[cfg(feature = "coerce_unsized")]
    #[inline]
    pub fn insert_sized<ConcretePtr>(
        &mut self,
        value: ConcretePtr,
    ) -> CastKey<ConcretePtr::Target, M::Key>
    where
        ConcretePtr: std::ops::CoerceUnsized<M::Value> + Deref,
        ConcretePtr::Target: Sized,
    {
        self.inner.insert_sized(value)
    }

    /// Like [`insert_sized`](Self::insert_sized) but the closure receives the
    /// typed key the value will live under.
    ///
    /// This method needs the `coerce_unsized` feature.
    #[cfg(feature = "coerce_unsized")]
    #[inline]
    pub fn insert_sized_with_key<ConcretePtr>(
        &mut self,
        func: impl FnOnce(CastKey<ConcretePtr::Target, M::Key>) -> ConcretePtr,
    ) -> CastKey<ConcretePtr::Target, M::Key>
    where
        M: InsertWithKey,
        ConcretePtr: std::ops::CoerceUnsized<M::Value> + Deref,
        ConcretePtr::Target: Sized,
    {
        self.inner.insert_sized_with_key(func)
    }

    /// Fallible version of [`insert_sized_with_key`](Self::insert_sized_with_key).
    ///
    /// This method needs the `coerce_unsized` feature.
    #[cfg(feature = "coerce_unsized")]
    #[inline]
    pub fn try_insert_sized_with_key<ConcretePtr, E>(
        &mut self,
        func: impl FnOnce(CastKey<ConcretePtr::Target, M::Key>) -> Result<ConcretePtr, E>,
    ) -> Result<CastKey<ConcretePtr::Target, M::Key>, E>
    where
        M: InsertWithKey,
        ConcretePtr: std::ops::CoerceUnsized<M::Value> + Deref,
        ConcretePtr::Target: Sized,
    {
        self.inner.try_insert_sized_with_key(func)
    }

    // ── insert_as ────────────────────────────────────────────────────────

    /// Inserts a smart pointer whose (possibly unsized) target differs from
    /// the map's output type, returning a key typed with the *source* type.
    ///
    /// This method needs the `coerce_unsized` feature.
    #[cfg(feature = "coerce_unsized")]
    #[inline]
    pub fn insert_as<SourcePtr>(&mut self, value: SourcePtr) -> CastKey<SourcePtr::Target, M::Key>
    where
        SourcePtr: std::ops::CoerceUnsized<M::Value> + StableDeref,
        SourcePtr::Target: Pointee<Metadata: Copy>,
    {
        self.inner.insert_as(value)
    }

    /// Inserts a smart pointer produced by `func`, returning a key typed with
    /// the source `SourcePtr::Target`. The closure receives the backing key.
    ///
    /// This method needs the `coerce_unsized` feature.
    #[cfg(feature = "coerce_unsized")]
    #[inline]
    pub fn insert_as_with_key<SourcePtr>(
        &mut self,
        func: impl FnOnce(M::Key) -> SourcePtr,
    ) -> CastKey<SourcePtr::Target, M::Key>
    where
        M: InsertWithKey,
        SourcePtr: std::ops::CoerceUnsized<M::Value> + StableDeref,
        SourcePtr::Target: Pointee<Metadata: Copy>,
    {
        self.inner.insert_as_with_key(func)
    }

    /// Fallible version of [`insert_as_with_key`](Self::insert_as_with_key).
    ///
    /// This method needs the `coerce_unsized` feature.
    #[cfg(feature = "coerce_unsized")]
    #[inline]
    pub fn try_insert_as_with_key<SourcePtr, E>(
        &mut self,
        func: impl FnOnce(M::Key) -> Result<SourcePtr, E>,
    ) -> Result<CastKey<SourcePtr::Target, M::Key>, E>
    where
        M: InsertWithKey,
        SourcePtr: std::ops::CoerceUnsized<M::Value> + StableDeref,
        SourcePtr::Target: Pointee<Metadata: Copy>,
    {
        self.inner.try_insert_as_with_key(func)
    }

    // ── cast_key_of ──────────────────────────────────────────────────────

    /// Builds a [`CastKey`] for the value under a backing key.
    #[inline]
    pub fn cast_key_of(&self, key: M::Key) -> Option<CastKey<MTarget<M>, M::Key>> {
        self.inner.cast_key_of(key)
    }

    // ── iterators ────────────────────────────────────────────────────────

    /// Lazy iterator over all [`CastKey`]s.
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = CastKey<MTarget<M>, M::Key>> + '_ {
        self.inner.keys()
    }

    /// Shared iterator over all occupied `(CastKey, &output)` pairs.
    #[inline]
    pub fn iter(&self) -> Iter<'_, M> {
        self.inner.iter()
    }

    /// Draining iterator. Removes all elements and yields them.
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, M> {
        self.inner.drain()
    }
}

// ─── Checked typed lookups (safe — type-id validated) ────────────────────────

impl<M> CastMapG<M>
where
    M: Map,
    M::Value: StableDeref + ConcreteTypeId,
    MTarget<M>: Pointee,
    <MTarget<M> as Pointee>::Metadata: Copy,
{
    /// Builds a `Concrete`-typed key from a backing key, or returns `None` if
    /// the value under it is not a `Concrete`.
    ///
    /// Takes the backing key directly (get one from any `CastKey` via
    /// [`inner_key`](CastKey::inner_key)): the check only needs the slot, so
    /// no pointer metadata is required.
    #[inline]
    pub fn downcast_key<Concrete: 'static>(
        &self,
        key: M::Key,
    ) -> Option<CastKey<Concrete, M::Key>> {
        let stored = self.inner.inner.get(key)?;
        if stored.concrete_type_id() == TypeId::of::<Concrete>() {
            Some(CastKey::from_raw_parts(key, ()))
        } else {
            None
        }
    }

    /// Returns whether the key still resolves in this map: its slot is live
    /// *and* holds a value of the key's type.
    #[inline]
    pub fn contains_key<T: ?Sized + AnyHaver + Pointee>(&self, key: CastKey<T, M::Key>) -> bool
    where
        <T as Pointee>::Metadata: Copy,
    {
        self.get(key).is_some()
    }

    /// Typed lookup by [`CastKey`]. Returns `None` if the slot is
    /// vacant, the key is stale, or the key's type does not match the value at
    /// that slot.
    #[inline]
    pub fn get<T: ?Sized + AnyHaver + Pointee>(&self, key: CastKey<T, M::Key>) -> Option<&T>
    where
        <T as Pointee>::Metadata: Copy,
    {
        let stored = self.inner.inner.get(key.inner_key())?;
        let stored_tid = stored.concrete_type_id();
        let base: &MTarget<M> = stored;
        if stored_tid != type_id_from_metadata::<T>(key.metadata()) {
            return None;
        }
        let data: *const () = (base as *const MTarget<M>).cast();
        // SAFETY: the value under the key has the concrete type that the key's
        // metadata implies, so the metadata fits the value.
        Some(unsafe { &*std::ptr::from_raw_parts::<T>(data, key.metadata()) })
    }

    /// Like [`get`](Self::get), but without any checks.
    ///
    /// # Safety
    /// A value must be under the key, and the key's metadata must fit it.
    #[inline]
    pub unsafe fn get_unchecked<T: ?Sized + Pointee>(&self, key: CastKey<T, M::Key>) -> &T
    where
        <T as Pointee>::Metadata: Copy,
    {
        self.inner.get_unchecked(key)
    }

    /// Removes an element by its [`CastKey`], returning the owned smart
    /// pointer re-typed to `T`. Returns `None` if the key is stale or its type
    /// does not match the slot.
    #[inline]
    pub fn remove<'a, T: ?Sized + AnyHaver + Pointee>(
        &mut self,
        key: CastKey<T, M::Key>,
    ) -> Option<<M::Value as RetypePtr<'a>>::Retyped<T>>
    where
        <T as Pointee>::Metadata: Copy,
        M::Value: RetypePtr<'a>,
    {
        let stored = self.inner.inner.get(key.inner_key())?;
        if stored.concrete_type_id() != type_id_from_metadata::<T>(key.metadata()) {
            return None;
        }
        // SAFETY: the type id check proved that the key's metadata fits the
        // value, and the `Map` contract makes `remove` take out that value.
        unsafe { self.inner.remove(key) }
    }

    /// Detaches an element by its [`CastKey`], returning the owned smart
    /// pointer re-typed to `T`. Unlike [`remove`](Self::remove) the slot stays
    /// reservable: the same key can be reused with
    /// [`reattach_by_inner_key`](Self::reattach_by_inner_key). Returns
    /// `None`, and detaches nothing, if the key is stale or its type does not
    /// match the slot.
    #[inline]
    pub fn detach<'a, T: ?Sized + AnyHaver + Pointee>(
        &mut self,
        key: CastKey<T, M::Key>,
    ) -> Option<<M::Value as RetypePtr<'a>>::Retyped<T>>
    where
        M: Detach,
        <T as Pointee>::Metadata: Copy,
        M::Value: RetypePtr<'a>,
    {
        let stored = self.inner.inner.get(key.inner_key())?;
        if stored.concrete_type_id() != type_id_from_metadata::<T>(key.metadata()) {
            return None;
        }
        // SAFETY: the type id check proved that the key's metadata fits the
        // value, and the `Map` contract makes `detach` take out that value.
        unsafe { self.inner.detach(key) }
    }
}

// ─── Checked operations requiring `&mut Output` ──────────────────────────────

impl<M> CastMapG<M>
where
    M: Map,
    M::Value: StableDeref + DerefMut + ConcreteTypeId,
    MTarget<M>: Pointee,
    <MTarget<M> as Pointee>::Metadata: Copy,
{
    /// Mutable typed lookup by [`CastKey`].
    /// Type-id validated, like [`get`](Self::get).
    #[inline]
    pub fn get_mut<T: ?Sized + AnyHaver + Pointee>(
        &mut self,
        key: CastKey<T, M::Key>,
    ) -> Option<&mut T>
    where
        <T as Pointee>::Metadata: Copy,
    {
        let stored = self.inner.inner.get_mut(key.inner_key())?;
        if stored.concrete_type_id() != type_id_from_metadata::<T>(key.metadata()) {
            return None;
        }
        let base: &mut MTarget<M> = stored;
        let data: *mut () = (base as *mut MTarget<M>).cast();
        // SAFETY: as in `get`.
        Some(unsafe { &mut *std::ptr::from_raw_parts_mut::<T>(data, key.metadata()) })
    }

    /// Like [`get_mut`](Self::get_mut), but without any checks.
    ///
    /// # Safety
    /// A value must be under the key, and the key's metadata must fit it.
    #[inline]
    pub unsafe fn get_unchecked_mut<T: ?Sized + Pointee>(
        &mut self,
        key: CastKey<T, M::Key>,
    ) -> &mut T
    where
        <T as Pointee>::Metadata: Copy,
    {
        self.inner.get_unchecked_mut(key)
    }

    /// Retains only elements for which `f(key, &mut output)` returns `true`.
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(CastKey<MTarget<M>, M::Key>, &mut MTarget<M>) -> bool,
    {
        self.inner.retain(f);
    }

    /// Mutable iterator over all occupied `(CastKey, &mut output)` pairs.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, M> {
        self.inner.iter_mut()
    }

    /// Mutable disjoint lookup typed by the keys' `T`, which may differ from
    /// the map's output type. All keys must share the pointee type `T`.
    /// Returns `None` if any key is stale, mistyped for its slot, or two keys
    /// alias the same slot.
    #[inline]
    pub fn get_disjoint_mut<T: ?Sized + AnyHaver + Pointee, const N: usize>(
        &mut self,
        keys: [CastKey<T, M::Key>; N],
    ) -> Option<[&mut T; N]>
    where
        M: GetDisjointMut,
        <T as Pointee>::Metadata: Copy,
    {
        // Validate every key's type against its slot first (shared borrows),
        // then hand off to the unsafe disjoint lookup, which enforces
        // liveness and pairwise disjointness.
        for key in &keys {
            let stored = self.inner.inner.get(key.inner_key())?;
            if stored.concrete_type_id() != type_id_from_metadata::<T>(key.metadata()) {
                return None;
            }
        }
        // SAFETY: the loop proved that each key's metadata fits its value, and
        // the `Map` contract makes `get_disjoint_mut` hand out those values in
        // the order of the keys.
        unsafe { self.inner.get_disjoint_mut(keys) }
    }

    /// Like [`get_disjoint_mut`](Self::get_disjoint_mut) but without validity,
    /// uniqueness, or type checks.
    ///
    /// # Safety
    /// - Every key must address a live slot, and no two keys may alias one slot.
    /// - Each key's pointer metadata must be valid for the data in its slot.
    #[inline]
    pub unsafe fn get_disjoint_unchecked_mut<T: ?Sized + Pointee, const N: usize>(
        &mut self,
        keys: [CastKey<T, M::Key>; N],
    ) -> [&mut T; N]
    where
        M: GetDisjointMut,
        <T as Pointee>::Metadata: Copy,
    {
        self.inner.get_disjoint_unchecked_mut(keys)
    }
}

// ─── Index / IndexMut ────────────────────────────────────────────────────────

impl<M, T: ?Sized + AnyHaver + Pointee> std::ops::Index<CastKey<T, M::Key>> for CastMapG<M>
where
    M: Map,
    M::Value: StableDeref + ConcreteTypeId,
    MTarget<M>: Pointee,
    <MTarget<M> as Pointee>::Metadata: Copy,
    <T as Pointee>::Metadata: Copy,
{
    type Output = T;

    #[inline]
    fn index(&self, key: CastKey<T, M::Key>) -> &T {
        self.get(key).expect("invalid CastKey for this map")
    }
}

impl<M, T: ?Sized + AnyHaver + Pointee> std::ops::IndexMut<CastKey<T, M::Key>> for CastMapG<M>
where
    M: Map,
    M::Value: StableDeref + DerefMut + ConcreteTypeId,
    MTarget<M>: Pointee,
    <MTarget<M> as Pointee>::Metadata: Copy,
    <T as Pointee>::Metadata: Copy,
{
    #[inline]
    fn index_mut(&mut self, key: CastKey<T, M::Key>) -> &mut T {
        self.get_mut(key).expect("invalid CastKey for this map")
    }
}

// ─── Iterators ───────────────────────────────────────────────────────────────

// The checked map's iterators are the unsafe map's iterators: keys need no
// re-wrapping.

/// Shared iterator over `(CastKey, &Target)` pairs.
pub type Iter<'a, M> = unsafe_cast_map::Iter<'a, M>;
/// Mutable iterator over `(CastKey, &mut Target)` pairs.
pub type IterMut<'a, M> = unsafe_cast_map::IterMut<'a, M>;
/// Draining iterator over `(CastKey, value)`, emptying the map.
pub type Drain<'a, M> = unsafe_cast_map::Drain<'a, M>;
/// Owning iterator over `(CastKey, value)` pairs.
pub type IntoIter<M> = unsafe_cast_map::IntoIter<M>;

impl<M> IntoIterator for CastMapG<M>
where
    M: Map + IntoIterator<Item = (<M as Map>::Key, <M as Map>::Value)>,
    M::Value: StableDeref,
    MTarget<M>: Pointee,
    <MTarget<M> as Pointee>::Metadata: Copy,
{
    type Item = (CastKey<MTarget<M>, M::Key>, M::Value);
    type IntoIter = IntoIter<M>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, M> IntoIterator for &'a CastMapG<M>
where
    M: Map + 'a,
    M::Value: StableDeref + 'a,
    MTarget<M>: Pointee,
    <MTarget<M> as Pointee>::Metadata: Copy,
{
    type Item = (CastKey<MTarget<M>, M::Key>, &'a MTarget<M>);
    type IntoIter = Iter<'a, M>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, M> IntoIterator for &'a mut CastMapG<M>
where
    M: Map + 'a,
    M::Value: StableDeref + DerefMut + 'a,
    MTarget<M>: Pointee,
    <MTarget<M> as Pointee>::Metadata: Copy,
{
    type Item = (CastKey<MTarget<M>, M::Key>, &'a mut MTarget<M>);
    type IntoIter = IterMut<'a, M>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

// ─── Type aliases ────────────────────────────────────────────────────────────

/// Safe castable-key map backed by [`slotmap::SlotMap`] (sparse storage).
#[cfg(feature = "slotmap")]
pub type CastMap<K, Ptr> = CastMapG<SlotMap<K, Ptr>>;

/// Safe castable-key map backed by [`slotmap::DenseSlotMap`] (contiguous
/// storage, fast iteration).
#[cfg(feature = "slotmap")]
pub type DenseCastMap<K, Ptr> = CastMapG<DenseSlotMap<K, Ptr>>;

/// Convenience alias: [`CastMap`] storing [`TypeTaggedBox<T>`] (e.g. `dyn Any`).
/// `TypeTaggedBox` implements [`ConcreteTypeId`], which the checked lookups require.
#[cfg(feature = "slotmap")]
pub type BoxCastMap<K, T> = CastMap<K, TypeTaggedBox<T>>;

/// Convenience alias: [`DenseCastMap`] storing [`TypeTaggedBox<T>`] (e.g. `dyn Any`).
#[cfg(feature = "slotmap")]
pub type BoxDenseCastMap<K, T> = DenseCastMap<K, TypeTaggedBox<T>>;

/// Safe castable-key map backed by a [`gen_map::GenMap`] with config `C`.
#[cfg(feature = "gen_map")]
pub type GenCastMap<Ptr, C = DefaultMapConfig> = CastMapG<GenMap<Ptr, C>>;

/// A [`GenCastMap`] that stores [`TypeTaggedBox<T>`].
#[cfg(feature = "gen_map")]
pub type BoxGenCastMap<T, C = DefaultMapConfig> = GenCastMap<TypeTaggedBox<T>, C>;
