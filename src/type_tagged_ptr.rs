//! [`TypeTaggedPtr`]: a smart pointer paired with the concrete [`TypeId`] of
//! its pointee, plus the [`ConcreteTypeId`] extension trait
//! [`CastMapG`](crate::cast_map::CastMapG)'s checked lookups validate
//! against. [`TypeTaggedBox`] is its owning-[`Box`] alias.
//!
//! [`CastMapG`](crate::cast_map::CastMapG)'s safety comes from comparing a
//! key's metadata-implied type id against the type id stored next to the
//! value. Something has to *store* that type id: [`TypeTaggedPtr`] does, for any
//! wrapped pointer (`Box`, `Rc`, `Arc`, `&T`, `&mut T`, ...). Custom stored
//! pointer types can participate by implementing [`ConcreteTypeId`].

use std::any::{Any, TypeId};
#[cfg(feature = "coerce_unsized")]
use std::ops::CoerceUnsized;
use std::ops::{Deref, DerefMut};
use std::ptr::Pointee;

use stable_deref_trait::StableDeref;

use crate::any_haver::{type_id_from_metadata, AnyHaver};
use crate::retype_ptr::RetypePtr;
#[cfg(feature = "coerce_unsized")]
use crate::stable_coerce::StableCoerce;

// ─── TypeTaggedPtr ───────────────────────────────────────────────────────────

/// A smart pointer `P` (`Box<T>`, `Rc<T>`, `Arc<T>`, `&T`, `&mut T`, ...)
/// paired with the concrete [`TypeId`] of the value it points at, kept even
/// after the pointer is coerced to a trait object.
///
/// The type id is captured at construction ([`from_ptr`](Self::from_ptr) /
/// `TypeTaggedBox::new`) and preserved across unsizing coercions
/// (`TypeTaggedPtr<Box<Dog>> -> TypeTaggedPtr<Box<dyn Animal>>`), since
/// unsizing only touches the inner pointer.
///
/// Those coercions need the `coerce_unsized` feature. Without it,
/// [`from_any`](Self::from_any) and [`from_any_haver`](Self::from_any_haver)
/// wrap a pointer that is already unsized.
///
/// # Invariant
/// `type_id` is always the [`TypeId`] of the concrete type of `ptr`'s
/// pointee. [`CastMapG`](crate::cast_map::CastMapG)'s checked lookups
/// rebuild typed references from it (see [`ConcreteTypeId`]); the `unsafe` escape hatches
/// ([`from_raw_parts`](Self::from_raw_parts), [`inner_mut`](Self::inner_mut))
/// make the caller responsible for keeping it true.
pub struct TypeTaggedPtr<P> {
    ptr: P,
    type_id: TypeId,
}

/// [`TypeTaggedPtr`] wrapping a [`Box`]: an owning, pointer-stable form
/// that remembers the concrete [`TypeId`] of the value it was constructed
/// from.
pub type TypeTaggedBox<T> = TypeTaggedPtr<Box<T>>;

impl<T: 'static> TypeTaggedPtr<Box<T>> {
    /// Boxes `value`, recording `TypeId::of::<T>()`.
    #[inline]
    pub fn new(value: T) -> Self {
        Self::from_ptr(Box::new(value))
    }
}

impl<P> TypeTaggedPtr<P> {
    /// Wraps `ptr`, recording `TypeId::of::<P::Target>()`. For an
    /// already-erased pointer whose concrete type id you know, use
    /// [`from_raw_parts`](Self::from_raw_parts).
    #[inline]
    pub fn from_ptr(ptr: P) -> Self
    where
        P: Deref,
        P::Target: Sized + 'static,
    {
        Self {
            type_id: TypeId::of::<P::Target>(),
            ptr,
        }
    }

    /// Assembles a `TypeTaggedPtr` from a pointer and an already-known type id.
    ///
    /// # Safety
    /// `type_id` must be the [`TypeId`] of the concrete type of `ptr`'s
    /// pointee (the struct invariant). A wrong type id lets
    /// [`CastMapG`](crate::cast_map::CastMapG)'s checked lookups reinterpret
    /// the value as another type, which is undefined behavior.
    #[inline]
    pub unsafe fn from_raw_parts(ptr: P, type_id: TypeId) -> Self {
        Self { ptr, type_id }
    }

    /// Shared access to the wrapped pointer itself (for the pointee, use
    /// [`Deref`]).
    #[inline]
    pub fn inner_ref(&self) -> &P {
        &self.ptr
    }

    /// Exclusive access to the wrapped pointer itself.
    ///
    /// # Safety
    /// `&mut P` can replace or re-point the pointer — e.g. swap in a
    /// different `Box<dyn Any>` — while the recorded type id stays put. When the
    /// borrow ends, `type_id` must still be the concrete type id of the (possibly
    /// new) pointee, per the struct invariant.
    #[inline]
    pub unsafe fn inner_mut(&mut self) -> &mut P {
        &mut self.ptr
    }

    /// Unwraps the pointer, discarding the recorded type id.
    #[inline]
    pub fn inner(self) -> P {
        self.ptr
    }
}

impl<P> TypeTaggedPtr<P>
where
    P: StableDeref,
    P::Target: DynAny,
{
    /// Wraps a pointer to a [`DynAny`], recording the type id that the value
    /// reports.
    ///
    /// ```
    /// use std::any::{Any, TypeId};
    /// use cast_map::{ConcreteTypeId, TypeTaggedBox};
    ///
    /// let tagged: TypeTaggedBox<dyn Any> =
    ///     TypeTaggedBox::from_any(Box::new(5u32));
    /// assert_eq!(tagged.concrete_type_id(), TypeId::of::<u32>());
    /// ```
    #[inline]
    pub fn from_any(ptr: P) -> Self {
        let type_id = DynAny::value_type_id(&*ptr);
        Self { ptr, type_id }
    }
}

impl<P> TypeTaggedPtr<P>
where
    P: StableDeref,
    P::Target: AnyHaver + Pointee,
{
    /// Wraps a pointer to an [`AnyHaver`], recording the type id that its
    /// pointer metadata implies.
    ///
    /// ```
    /// use std::any::TypeId;
    /// use cast_map::{AnyHaver, ConcreteTypeId, TypeTaggedBox};
    ///
    /// trait Pet: AnyHaver {}
    ///
    /// struct Dog;
    /// impl Pet for Dog {}
    ///
    /// let tagged: TypeTaggedBox<dyn Pet> =
    ///     TypeTaggedBox::from_any_haver(Box::new(Dog));
    /// assert_eq!(tagged.concrete_type_id(), TypeId::of::<Dog>());
    /// ```
    #[inline]
    pub fn from_any_haver(ptr: P) -> Self {
        let metadata = std::ptr::metadata(&*ptr as *const P::Target);
        let type_id = type_id_from_metadata::<P::Target>(metadata);
        Self { ptr, type_id }
    }
}

impl<P: Deref> Deref for TypeTaggedPtr<P> {
    type Target = P::Target;
    #[inline]
    fn deref(&self) -> &P::Target {
        &self.ptr
    }
}

impl<P: DerefMut> DerefMut for TypeTaggedPtr<P> {
    #[inline]
    fn deref_mut(&mut self) -> &mut P::Target {
        &mut self.ptr
    }
}

// A `TypeTaggedPtr<P>` coerces to a `TypeTaggedPtr<Q>` when `P` coerces to `Q`.
// The `StableCoerce` bound requires `P` to keep pointing at the same value
// through that coercion, so the recorded type id stays correct.
#[cfg(feature = "coerce_unsized")]
impl<P: CoerceUnsized<Q> + StableCoerce, Q> CoerceUnsized<TypeTaggedPtr<Q>> for TypeTaggedPtr<P> {}

unsafe impl<P: StableDeref> StableDeref for TypeTaggedPtr<P> {}

unsafe impl<'a, P: RetypePtr<'a>> RetypePtr<'a> for TypeTaggedPtr<P> {
    type Retyped<U: ?Sized + 'a> = TypeTaggedPtr<P::Retyped<U>>;
    #[inline]
    unsafe fn retype<U: ?Sized>(self, meta: <U as Pointee>::Metadata) -> Self::Retyped<U> {
        TypeTaggedPtr {
            ptr: self.ptr.retype(meta),
            type_id: self.type_id,
        }
    }
}

// ─── DynAny ──────────────────────────────────────────────────────────────────

/// A `dyn Any`, `dyn Any + Send` or `dyn Any + Send + Sync`, which
/// [`TypeTaggedPtr::from_any`] takes. It is sealed, so it cannot be implemented
/// outside this crate.
pub trait DynAny: sealed::Sealed {
    /// Returns the type id of the concrete type of the value.
    fn value_type_id(&self) -> TypeId;
}

mod sealed {
    use std::any::Any;

    /// Keeps [`DynAny`](super::DynAny) from being implemented outside this
    /// crate.
    pub trait Sealed {}

    impl Sealed for dyn Any {}
    impl Sealed for dyn Any + Send {}
    impl Sealed for dyn Any + Send + Sync {}
}

// `type_id` on a `dyn Any` dispatches to the concrete type.
impl DynAny for dyn Any {
    #[inline]
    fn value_type_id(&self) -> TypeId {
        self.type_id()
    }
}

impl DynAny for dyn Any + Send {
    #[inline]
    fn value_type_id(&self) -> TypeId {
        self.type_id()
    }
}

impl DynAny for dyn Any + Send + Sync {
    #[inline]
    fn value_type_id(&self) -> TypeId {
        self.type_id()
    }
}

// ─── ConcreteTypeId ──────────────────────────────────────────────────────────

/// A stored pointer that knows the concrete [`TypeId`] of its pointee.
///
/// This is the extension point for [`CastMapG`](crate::cast_map::CastMapG):
/// its checked lookups read it to validate a key's type. The crate implements
/// it for [`TypeTaggedPtr`] (and thus [`TypeTaggedBox`]), but it is deliberately a
/// public, standalone trait — to use your own stored pointer type with
/// [`CastMapG`](crate::cast_map::CastMapG), implement `ConcreteTypeId` for it
/// (alongside `Deref` + [`StableDeref`], which any stored pointer needs).
/// Nothing here assumes `TypeTaggedPtr` specifically.
///
/// Why store the type id instead of asking the value? Not every stored
/// pointer answers correctly: `Box<dyn Any>` could, but for a `Box<dyn Foo>`
/// where `Foo` is not an `Any` subtrait, `type_id` resolves statically to
/// `TypeId::of::<dyn Foo>()` — not the underlying type's — and
/// special-casing the stored pointers that answer correctly would make
/// [`CastMapG`](crate::cast_map::CastMapG)'s behavior depend confusingly on
/// the stored pointer's type. An
/// explicitly stored type id works uniformly — and is also a performance win: a
/// plain field read per lookup instead of a virtual call to ask the value.
///
/// # Safety
/// [`CastMapG`](crate::cast_map::CastMapG)'s checked lookups (`get`,
/// `get_mut`, `remove`, `get_disjoint_mut`, `downcast_key`) rebuild typed
/// references based on this value: `concrete_type_id` must return the
/// [`TypeId`] of the concrete type of its current pointee. A
/// wrong answer lets one of those safe lookups reinterpret the value as
/// another type, which is undefined behavior.
pub unsafe trait ConcreteTypeId {
    /// The concrete type id of this pointer's pointee.
    fn concrete_type_id(&self) -> TypeId;
}

// SAFETY: every constructor records the concrete type id. `from_ptr` takes it
// from the sized target, `from_any` from `Any` and `from_any_haver` from
// `AnyHaver`, and the `StableDeref` bound of the last two keeps the pointee in
// place. The caller of `from_raw_parts` vouches for it. Unsizing and `retype`
// keep the value's type, and `inner_mut`'s contract makes a caller who swaps
// the pointer keep the type id accurate.
unsafe impl<P> ConcreteTypeId for TypeTaggedPtr<P> {
    #[inline]
    fn concrete_type_id(&self) -> TypeId {
        self.type_id
    }
}
