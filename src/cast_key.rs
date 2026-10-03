//! [`CastKey<T, K>`] is the key of the cast maps. It holds a key of the backing
//! map and the pointer metadata of `T`.
//!
//! For dyn dispatch on a key, see [`DynKey`]. To upcast a key, use
//! [`upcast_key!`](crate::upcast_key).

use std::any::TypeId;
use std::ops::Receiver;
use std::ptr::Pointee;

use crate::any_haver::{type_id_from_metadata, AnyHaver};
use crate::dyn_key::DynKey;

// ─── CastKey<T, K> ───────────────────────────────────────────────────────────

/// A key of the backing map, of type `K`, together with the pointer metadata of
/// `T`.
///
/// # Sizes (64-bit)
/// - `CastKey<SizedType, K>` is the size of `K`, because its metadata is `()`.
/// - `CastKey<dyn Trait, K>` holds a `K` and a vtable pointer.
pub struct CastKey<T: ?Sized + Pointee, K: Copy>
where
    <T as Pointee>::Metadata: Copy,
{
    pub(crate) key: K,
    pub(crate) metadata: <T as Pointee>::Metadata,
}

impl<T: ?Sized + Pointee, K: Copy> Clone for CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Pointee, K: Copy> Copy for CastKey<T, K> where <T as Pointee>::Metadata: Copy {}

impl<T: ?Sized + Pointee, K: Copy + std::fmt::Debug> std::fmt::Debug for CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CastKey").field("key", &self.key).finish()
    }
}

impl<T: ?Sized + Pointee, K: Copy + PartialEq> PartialEq for CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    /// Equality is on the backing key only; pointer metadata is not compared.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

// A trait method can take `self: CastKey<Self, K>`, but only for static
// dispatch. Dyn dispatch needs the pointer-shaped `DynKey`.
impl<T: ?Sized + Pointee, K: Copy> Receiver for CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    type Target = T;
}


impl<T: ?Sized + Pointee, K: Copy + Eq> Eq for CastKey<T, K> where <T as Pointee>::Metadata: Copy {}

impl<T: ?Sized + Pointee, K: Copy + std::hash::Hash> std::hash::Hash for CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T: ?Sized + Pointee, K: Copy> CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    /// Returns the pointer metadata for `T`.
    #[inline]
    pub fn metadata(&self) -> <T as Pointee>::Metadata {
        self.metadata
    }

    /// Returns the key of the backing map.
    #[inline]
    pub fn inner_key(&self) -> K {
        self.key
    }

    /// Borrows this key into its dyn-dispatchable form, usable as a trait
    /// method receiver (`fn m(self: DynKey<Self, K>, ..)`).
    #[inline]
    pub fn as_dyn(&self) -> DynKey<'_, T, K> {
        DynKey::new(self)
    }

    /// Builds a cast key from raw parts.
    #[inline]
    pub fn from_raw_parts(key: K, metadata: <T as Pointee>::Metadata) -> Self {
        Self { key, metadata }
    }
}

#[cfg(feature = "slotmap")]
impl<T: ?Sized + Pointee, K: slotmap::Key> CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    /// Returns the backing `slotmap` [`KeyData`](slotmap::KeyData).
    #[inline]
    pub fn key_data(&self) -> slotmap::KeyData {
        self.key.data()
    }
}

impl<T: ?Sized + AnyHaver + Pointee, K: Copy> CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    /// Downcasts to a sized `Concrete` using only the key's cached metadata;
    /// no map involved. Returns `None` on a type mismatch.
    #[inline]
    pub fn downcast<Concrete: 'static>(self) -> Option<CastKey<Concrete, K>> {
        (type_id_from_metadata::<T>(self.metadata) == TypeId::of::<Concrete>())
            .then(|| CastKey::from_raw_parts(self.key, ()))
    }
}

// ─── upcast_key! ─────────────────────────────────────────────────────────────

/// Upcasts a [`CastKey`], such as from a `CastKey<Dog, K>` to a
/// `CastKey<dyn Pet, K>`. It borrows the key as a [`DynKey`], coerces the
/// `DynKey` and turns it back into a `CastKey`.
///
/// `upcast_key!(key, Type)` upcasts to `Type`. `upcast_key!(key)` and
/// `upcast_key!(key, _)` upcast to the type that the surrounding code expects,
/// such as the type of a `let` binding or of a parameter. Where nothing expects
/// a type, the key keeps its own.
///
/// ```
/// use cast_slotmap::{upcast_key, AnyHaver, CastKey};
///
/// trait Pet: AnyHaver {}
///
/// struct Dog;
/// impl Pet for Dog {}
///
/// let dog: CastKey<Dog, usize> = CastKey::from_raw_parts(7, ());
/// let pet = upcast_key!(dog, dyn Pet);
/// let same_pet: CastKey<dyn Pet, usize> = upcast_key!(dog);
/// assert_eq!(pet, same_pet);
/// assert!(pet.downcast::<Dog>().is_some());
/// ```
#[macro_export]
macro_rules! upcast_key {
    ($key:expr $(,)?) => {
        // The compiler coerces the argument of `DynKey::key` to the `DynKey`
        // that matches the `CastKey` the surrounding code expects.
        $crate::DynKey::key(($key).as_dyn())
    };
    ($key:expr, _ $(,)?) => {
        $crate::upcast_key!($key)
    };
    ($key:expr, $target:ty $(,)?) => {{
        let key = $key;
        let dyn_key: $crate::DynKey<'_, $target, _> = key.as_dyn();
        dyn_key.key()
    }};
}
