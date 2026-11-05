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
//! The method takes a [`DynKey`] whose backing key is `()`. That key holds no
//! data, so [`type_id_from_metadata`] can build the `DynKey` from pointer
//! metadata alone and call the method without a value.
//!
//! Dispatch summary for `type_id_from_metadata::<T>(meta)`:
//! - `T` sized          → `TypeId::of::<T>()`              (static, metadata is `()`)
//! - `T = dyn AnyHaver` → the *concrete* type's `TypeId`   (virtual, via the vtable)
//! - `T = dyn Foo` where `Foo: AnyHaver` → the concrete type's `TypeId` through
//!   `dyn Foo`'s vtable (supertrait methods live in the vtable).

use std::any::TypeId;
use std::ptr::Pointee;

use crate::dyn_key::DynKey;

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
    fn haver_type_id(self: DynKey<'_, Self, ()>) -> TypeId {
        TypeId::of::<Self>()
    }
}

// SAFETY: for a sized `T`, `TypeId::of::<Self>()` *is* the concrete type id.
unsafe impl<T: 'static> AnyHaver for T {}

#[inline]
pub fn type_id_from_metadata<T: ?Sized + AnyHaver + Pointee>(
    metadata: <T as Pointee>::Metadata,
) -> TypeId {
    DynKey::<T, ()>::from_metadata(metadata).haver_type_id()
}
