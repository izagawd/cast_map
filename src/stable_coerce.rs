//! [`StableCoerce`] marks a pointer that still points at the same value after
//! an unsizing coercion. It needs the `coerce_unsized` feature.

use std::rc::Rc;
use std::sync::Arc;

/// Marks a pointer whose [`CoerceUnsized`](std::ops::CoerceUnsized) coercions
/// change only its static type, so the coerced pointer still points at the
/// same value. `Box`, `Rc`, `Arc`, `&T` and `&mut T` implement it.
///
/// # Safety
/// After `Self` coerces to another pointer type, the coerced pointer must deref
/// to the same value, at the same address, as `Self` did. Otherwise
/// [`CastMapG`](crate::cast_map::CastMapG)'s coercing inserts record the type
/// id of one value, and its lookups then read a different value as that type,
/// which is undefined behavior.
pub unsafe trait StableCoerce {}

// SAFETY: these pointers unsize by changing only their pointer metadata, so a
// coerced pointer still derefs to the value it was built from.
unsafe impl<T: ?Sized> StableCoerce for Box<T> {}
unsafe impl<T: ?Sized> StableCoerce for Rc<T> {}
unsafe impl<T: ?Sized> StableCoerce for Arc<T> {}
unsafe impl<'a, T: ?Sized> StableCoerce for &'a T {}
unsafe impl<'a, T: ?Sized> StableCoerce for &'a mut T {}
