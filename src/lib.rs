//! This crate provides maps that store type-erased values, such as
//! `TypeTaggedBox<dyn Any>`, and hand out typed [`CastKey`]s, so that
//! `map.get(key)` returns a `&T` with no `downcast_ref` at the call site.
//!
//! [`CastMap`] checks every lookup against the type id stored with the value,
//! so a key of the wrong type returns `None`. [`UnsafeCastMap`] skips that
//! check, so its typed lookups are `unsafe`. Both keep their values in a
//! backing map, which is any type that implements [`Map`].
//!
//! # Cargo features
//!
//! - `slotmap` adds `slotmap::SlotMap` and `slotmap::DenseSlotMap` as backing
//!   maps, with aliases such as `BoxSlotCastMap<K, T>`.
//! - `gen_map` adds `gen_map::GenMap` as a backing map, with aliases such as
//!   `BoxGenCastMap<T, C>`.
//! - `coerce_unsized` adds the coercion from a `TypeTaggedBox<Dog>` to a
//!   `TypeTaggedBox<dyn Any>`, and the `insert_sized` and `insert_as` methods.
//!   Without it, [`TypeTaggedPtr::from_any`] and
//!   [`TypeTaggedPtr::from_any_haver`] wrap a pointer that is already unsized.
//!
//! None of them is on by default.
//!
//! # `AnyHaver` and key types
//! Checked lookups require `T: AnyHaver`, an **`unsafe` trait** that recovers a
//! concrete [`TypeId`](std::any::TypeId) from pointer metadata alone. All
//! `'static` sized types get it via a blanket impl; trait-object keys get it by
//! declaring it as a supertrait (`trait Foo: AnyHaver`), which puts the lookup
//! in `dyn Foo`'s vtable. `dyn Any` has no such supertrait, so
//! `map.get(dyn_any_key)` is a **compile error** — recover a typed key with
//! [`downcast_key`](CastMap::downcast_key) or read type-erased through
//! [`get_by_inner_key`](CastMap::get_by_inner_key) instead.
//!
//! # Dyn-dispatchable keys
//! [`DynKey`] (via [`CastKey::as_dyn`]) is a borrowed key that can be a trait
//! method receiver, so a method declared as `fn m(self: DynKey<Self, K>, ..)`
//! can be called through a `DynKey<dyn Trait, K>`. [`upcast_key!`] upcasts a
//! [`CastKey`] through a `DynKey`.
//!
//! # Nightly
//! Pointer-metadata reconstruction and the dyn-dispatchable key rely on the
//! unstable `ptr_metadata`, `derive_coerce_pointee` and `arbitrary_self_types`
//! features, so this crate requires a **nightly** toolchain. The crate's
//! `coerce_unsized` feature also turns on the unstable `coerce_unsized`
//! feature.
//!
//! # Example
//! This example needs the `slotmap` and `coerce_unsized` features.
//!
//! ```ignore
//! use cast_map::{BoxSlotCastMap, TypeTaggedBox, CastKey, DefaultKey};
//! use std::any::Any;
//!
//! struct Dog { name: String }
//!
//! let mut map: BoxSlotCastMap<DefaultKey, dyn Any> = BoxSlotCastMap::new();
//!
//! // Insert a concrete type into a `dyn Any` map; the key comes back typed.
//! let dog_key: CastKey<Dog, DefaultKey> = map.insert_sized(TypeTaggedBox::new(Dog { name: "Rex".into() }));
//!
//! assert_eq!(map.get(dog_key).unwrap().name, "Rex");
//!
//! // Or insert erased and recover the typed key later.
//! let dyn_key: CastKey<dyn Any, DefaultKey> = map.insert(TypeTaggedBox::new(Dog { name: "Ax".into() }));
//! let typed: CastKey<Dog, DefaultKey> = map.downcast_key::<Dog>(dyn_key.inner_key()).unwrap();
//! ```
#![feature(ptr_metadata)]
#![feature(derive_coerce_pointee)]
#![feature(arbitrary_self_types)]
#![cfg_attr(feature = "coerce_unsized", feature(coerce_unsized))]

pub mod any_haver;
pub mod cast_key;
pub mod cast_map;
pub mod dyn_key;
#[cfg(feature = "gen_map")]
mod gen_map_impl;
pub mod map;
pub mod retype_ptr;
#[cfg(feature = "slotmap")]
mod slotmap_impl;
#[cfg(feature = "coerce_unsized")]
pub mod stable_coerce;
pub mod type_tagged_ptr;
pub mod unsafe_cast_map;

// Re-export the slotmap items callers need so they don't have to depend on
// `slotmap` directly for the common path.
#[cfg(feature = "slotmap")]
pub use slotmap::{new_key_type, DefaultKey, Key, KeyData};

#[doc(inline)]
pub use any_haver::{type_id_from_metadata, AnyHaver};
#[doc(inline)]
pub use cast_key::CastKey;
#[doc(inline)]
pub use cast_map::CastMap;
#[cfg(feature = "slotmap")]
#[doc(inline)]
pub use cast_map::{BoxDenseSlotCastMap, BoxSlotCastMap, DenseSlotCastMap, SlotCastMap};
#[cfg(feature = "gen_map")]
#[doc(inline)]
pub use cast_map::{BoxGenCastMap, GenCastMap};
#[doc(inline)]
pub use dyn_key::DynKey;
#[doc(inline)]
pub use map::{
    Capacity, Clear, Detach, Drain, GetDisjointMut, InsertWithKey, Iterate, IterateKeys,
    IterateValues, Map, Remove, Reserve, Retain,
};
#[doc(inline)]
pub use retype_ptr::RetypePtr;
#[cfg(feature = "coerce_unsized")]
#[doc(inline)]
pub use stable_coerce::StableCoerce;
#[doc(no_inline)]
pub use stable_deref_trait::StableDeref;
#[doc(inline)]
pub use type_tagged_ptr::{ConcreteTypeId, DynAny, TypeTaggedBox, TypeTaggedPtr};
#[doc(inline)]
pub use unsafe_cast_map::UnsafeCastMap;
#[cfg(feature = "slotmap")]
#[doc(inline)]
pub use unsafe_cast_map::{
    UnsafeBoxDenseSlotCastMap, UnsafeBoxSlotCastMap, UnsafeDenseSlotCastMap, UnsafeSlotCastMap,
};
#[cfg(feature = "gen_map")]
#[doc(inline)]
pub use unsafe_cast_map::{UnsafeBoxGenCastMap, UnsafeGenCastMap};

#[cfg(test)]
mod tests;
