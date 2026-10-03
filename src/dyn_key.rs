//! [`DynKey`] is a borrowed [`CastKey`] that can be the **method receiver of a
//! trait object**.
//!
//! A receiver for dyn dispatch must be the size of a pointer, and a `CastKey`
//! holds a backing key of any size. A `DynKey` is a single fat pointer instead.
//! Its metadata is the key's metadata, and its address points at the backing
//! key inside the borrowed `CastKey`.

use std::any::TypeId;
use std::marker::{CoercePointee, PhantomData};
use std::ops::Receiver;
use std::ptr::{NonNull, Pointee};

use crate::any_haver::{type_id_from_metadata, AnyHaver};
use crate::cast_key::CastKey;

/// A borrowed, dyn-dispatchable form of a [`CastKey`].
///
/// Obtain one with [`CastKey::as_dyn`] (or `From<&CastKey>`); recover the key
/// with [`DynKey::key`] (or `Into<CastKey>`). Use it as a trait-method
/// receiver:
///
/// ```ignore
/// trait Component {
///     fn tick(self: DynKey<'_, Self, DefaultKey>, world: &mut World);
/// }
/// let dk: DynKey<'_, dyn Component, DefaultKey> = key.as_dyn();
/// dk.tick(&mut world); // virtual call through the key's metadata
/// ```
// `CoercePointee` gives `DynKey` the unsizing coercions and the dyn dispatch of
// a pointer.
#[derive(CoercePointee)]
#[repr(transparent)]
pub struct DynKey<'a, #[pointee] T: ?Sized, K: Copy> {
    /// Points at the `K` of the borrowed `CastKey` and carries its metadata.
    /// Nothing reads it as a `T`.
    ptr: NonNull<T>,
    _borrow: PhantomData<&'a K>,
}

impl<'a, T: ?Sized, K: Copy> Clone for DynKey<'a, T, K> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, T: ?Sized, K: Copy> Copy for DynKey<'a, T, K> {}

// SAFETY: a `DynKey` acts as a `&'a CastKey<T, K>`, so it can be sent and
// shared when the `CastKey` is `Sync`.
unsafe impl<'a, T: ?Sized + Pointee, K: Copy> Send for DynKey<'a, T, K>
where
    <T as Pointee>::Metadata: Copy,
    CastKey<T, K>: Sync,
{
}
unsafe impl<'a, T: ?Sized + Pointee, K: Copy> Sync for DynKey<'a, T, K>
where
    <T as Pointee>::Metadata: Copy,
    CastKey<T, K>: Sync,
{
}

// A receiver without `Deref`: the key alone cannot reach the value (that needs
// the map), so only dispatch — not `*dyn_key` — is offered.
impl<'a, T: ?Sized, K: Copy> Receiver for DynKey<'a, T, K> {
    type Target = T;
}

impl<'a, T: ?Sized + Pointee, K: Copy> DynKey<'a, T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    /// Borrows a [`CastKey`] into its dyn-dispatchable form.
    #[inline]
    pub fn new(key: &'a CastKey<T, K>) -> Self {
        // `K` has the same type for every `T`, so `key()` can read it back
        // after an unsizing coercion changes `T`.
        let thin: NonNull<()> = NonNull::from(&key.key).cast();
        Self {
            ptr: NonNull::from_raw_parts(thin, key.metadata()),
            _borrow: PhantomData,
        }
    }

    /// Recovers the [`CastKey`] this `DynKey` was made from.
    #[inline]
    pub fn key(self) -> CastKey<T, K> {
        let (thin, metadata) = self.ptr.to_raw_parts();
        // SAFETY: `thin` points at the `K` of the `CastKey` that `new`
        // borrowed for 'a, and `K` is `Copy`.
        let key = unsafe { thin.cast::<K>().read() };
        CastKey::from_raw_parts(key, metadata)
    }
}

impl<'a, T: ?Sized + AnyHaver + Pointee, K: Copy> DynKey<'a, T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    /// Downcasts to a sized `Concrete` using only the key's metadata; no map
    /// involved. Returns `None` on a type mismatch. See [`CastKey::downcast`].
    #[inline]
    pub fn downcast<Concrete: 'static>(self) -> Option<DynKey<'a, Concrete, K>> {
        let (thin, metadata) = self.ptr.to_raw_parts();
        (type_id_from_metadata::<T>(metadata) == TypeId::of::<Concrete>()).then(|| {
            DynKey {
                // `Concrete` is sized, so its metadata is `()`.
                ptr: NonNull::from_raw_parts(thin, ()),
                _borrow: PhantomData,
            }
        })
    }
}

impl<'a, T: ?Sized + Pointee, K: Copy> From<&'a CastKey<T, K>> for DynKey<'a, T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    #[inline]
    fn from(key: &'a CastKey<T, K>) -> Self {
        Self::new(key)
    }
}

impl<'a, T: ?Sized + Pointee, K: Copy> From<DynKey<'a, T, K>> for CastKey<T, K>
where
    <T as Pointee>::Metadata: Copy,
{
    #[inline]
    fn from(key: DynKey<'a, T, K>) -> Self {
        key.key()
    }
}
