//! Recovering a concrete [`TypeId`] from pointer metadata, with no live value.
//!
//! [`AnyHaver`] is an **`unsafe` trait**, blanket-implemented for every
//! `'static` **sized** type. Unsized types get it only through a supertrait
//! bound (`trait Foo: AnyHaver`), which places `haver_type_id` in `dyn Foo`'s
//! vtable so the call dispatches virtually to the concrete type's impl.
//! Consequently `dyn Any` (no such supertrait) simply does not implement
//! `AnyHaver` — asking the checked map for `&dyn Any` is a compile error
//! rather than a silent miss; use `downcast_key` / `get_by_inner_key` for
//! erased access.
//!
//! The method takes a [`MetadataPtr`], which carries only pointer metadata, so
//! [`type_id_from_metadata`] can call it without a value.
//!
//! Dispatch summary for `type_id_from_metadata::<T>(meta)`:
//! - `T` sized          → `TypeId::of::<T>()`              (static, metadata is `()`)
//! - `T = dyn AnyHaver` → the *concrete* type's `TypeId`   (virtual, via the vtable)
//! - `T = dyn Foo` where `Foo: AnyHaver` → the concrete type's `TypeId` through
//!   `dyn Foo`'s vtable (supertrait methods live in the vtable).

use std::any::TypeId;
use std::marker::CoercePointee;
use std::ops::Receiver;
use std::ptr::{NonNull, Pointee};

/// A pointer that carries only the pointer metadata of a `T` and points at no
/// value. It is the receiver of [`AnyHaver::haver_type_id`].
// `CoercePointee` gives `MetadataPtr` the unsizing coercions and the dyn
// dispatch of a pointer.
#[derive(CoercePointee)]
#[repr(transparent)]
pub struct MetadataPtr<#[pointee] T: ?Sized> {
    ptr: NonNull<T>,
}

impl<T: ?Sized> Clone for MetadataPtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for MetadataPtr<T> {}

// `MetadataPtr` is a receiver without `Deref`, because no value sits behind it.
impl<T: ?Sized> Receiver for MetadataPtr<T> {
    type Target = T;
}

impl<T: ?Sized + Pointee> MetadataPtr<T> {
    /// Builds a pointer with a dangling data address and the given metadata.
    #[inline]
    pub(crate) fn new(metadata: <T as Pointee>::Metadata) -> Self {
        Self {
            ptr: NonNull::from_raw_parts(NonNull::<()>::dangling(), metadata),
        }
    }
}

/// Reports the concrete [`TypeId`] from pointer metadata alone. Every
/// `'static` sized type implements it, and a trait object gets it when its
/// trait has `AnyHaver` as a supertrait.
///
/// # Safety
/// [`CastMapG`](crate::cast_map::CastMapG)'s checked lookups (`get`,
/// `get_mut`, `remove`, `get_disjoint_mut`) rebuild typed references based
/// on this value: `haver_type_id` must return the [`TypeId`] of the true
/// concrete `Self`. A lying implementation makes those lookups unsound.
pub unsafe trait AnyHaver: 'static {
    /// Returns the [`TypeId`] of the (possibly type-erased) `Self`.
    #[inline]
    fn haver_type_id(self: MetadataPtr<Self>) -> TypeId {
        TypeId::of::<Self>()
    }
}

// SAFETY: for a sized `T`, `TypeId::of::<Self>()` *is* the concrete type id.
unsafe impl<T: 'static> AnyHaver for T {}

#[inline]
pub fn type_id_from_metadata<T: ?Sized + AnyHaver + Pointee>(
    metadata: <T as Pointee>::Metadata,
) -> TypeId {
    MetadataPtr::<T>::new(metadata).haver_type_id()
}
