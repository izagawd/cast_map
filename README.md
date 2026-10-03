# cast_slotmap

This crate provides maps that store erased values, like `TypeTaggedBox<dyn Any>`, and hand out **typed** keys, so `map.get(key)` returns a `&T` with no `downcast_ref` at the call site.

> **Nightly only.** This crate uses the unstable `ptr_metadata`, `derive_coerce_pointee` and `arbitrary_self_types` features. Its `coerce_unsized` feature also turns on the unstable `coerce_unsized` feature.

## Cargo features

None of these is on by default.

* `slotmap` adds the [`slotmap`](https://crates.io/crates/slotmap) crate's `SlotMap` and `DenseSlotMap` as backing maps.
* `gen_map` adds the [`gen_map`](https://crates.io/crates/gen_map) crate's `GenMap` as a backing map.
* `coerce_unsized` adds the coercion from `TypeTaggedBox<Dog>` to `TypeTaggedBox<dyn Any>`, and the `insert_sized` and `insert_as` methods.

## The maps

* **`CastMapG<M>`** is the safe map, and the recommended one. Each value sits behind a pointer that records its concrete `TypeId`, such as a `TypeTaggedBox`, and every lookup compares that type id with the one the key implies. A key of the wrong type returns `None`.
* **`UnsafeCastMapG<M>`** is the low level map. Its `get`, `get_mut` and `remove` are `unsafe`, because they trust the metadata that a `CastKey<T, K>` caches. If the value under the key has a different type now, its bytes get read as a `T`, which is undefined behavior and not a `None`.

`M` is the backing map, and the features add aliases for their backing maps.

| Backing map | Checked | Raw |
|---|---|---|
| `slotmap::SlotMap` | `CastMap<K, Ptr>`, `BoxCastMap<K, T>` | `UnsafeCastMap<K, Ptr>`, `UnsafeBoxCastMap<K, T>` |
| `slotmap::DenseSlotMap` | `DenseCastMap<K, Ptr>`, `BoxDenseCastMap<K, T>` | `UnsafeDenseCastMap<K, Ptr>`, `UnsafeBoxDenseCastMap<K, T>` |
| `gen_map::GenMap` | `GenCastMap<Ptr, C>`, `BoxGenCastMap<T, C>` | `UnsafeGenCastMap<Ptr, C>`, `UnsafeBoxGenCastMap<T, C>` |

The `Box` aliases store a `TypeTaggedBox` in the checked maps and a plain `Box` in the raw ones. `C` is the config of a `GenMap`.

This example needs the `slotmap` and `coerce_unsized` features.

```rust
#![feature(ptr_metadata, derive_coerce_pointee, arbitrary_self_types)]
use cast_slotmap::{BoxCastMap, TypeTaggedBox, CastKey, DefaultKey};
use std::any::Any;

struct Dog { name: String }

let mut map: BoxCastMap<DefaultKey, dyn Any> = BoxCastMap::new();

// Insert a concrete type into a `dyn Any` map; the key comes back typed.
let dog_key: CastKey<Dog, DefaultKey> = map.insert_sized(TypeTaggedBox::new(Dog { name: "Rex".into() }));
assert_eq!(map.get(dog_key).unwrap().name, "Rex");

// Or insert erased and recover the typed key later.
let dyn_key: CastKey<dyn Any, DefaultKey> = map.insert(TypeTaggedBox::new(Dog { name: "Ax".into() }));
let typed: CastKey<Dog, DefaultKey> = map.downcast_key::<Dog>(dyn_key.inner_key()).unwrap();
```

Without `coerce_unsized`, coerce the inner pointer and wrap it with `TypeTaggedBox::from_any` instead.

```rust
let dyn_key = map.insert(TypeTaggedBox::from_any(Box::new(Dog { name: "Rex".into() })));
let dog_key: CastKey<Dog, DefaultKey> = map.downcast_key(dyn_key.inner_key()).unwrap();
```

## Backing maps

`Map` is the core of a backing map, and its key can be any `Copy` type. The other traits add capabilities, and a cast map only has the methods whose capability its backing map implements.

* `InsertWithKey` adds the `*_with_key` inserts.
* `GetDisjointMut` adds the `get_disjoint_*` methods.
* `Detach` adds detaching and reattaching.
* `Capacity` adds `capacity`, and `Reserve` adds `with_capacity`, `reserve` and `try_reserve`.

The `slotmap` maps implement all of them. A `GenMap` implements `Reserve` only when its storage can grow.

## `AnyHaver`: the type check on the key side

Checked lookups need `T: AnyHaver`, an **`unsafe` trait** with one method that gets the concrete `TypeId` using only pointer metadata. Every `'static` **sized** type gets it from a blanket impl. Trait objects get it through a supertrait:

```rust
trait Component: AnyHaver { /* … */ }   // puts the lookup in dyn Component's vtable
```

`dyn Any` has no such supertrait, so `map.get(dyn_any_key)` fails to compile instead of silently missing. Use `downcast_key` to get a typed key back, or `get_by_inner_key` for erased access. Implementing `AnyHaver` by hand is `unsafe`: returning a wrong `TypeId` would break the safety of the checked lookups.

## `DynKey`: keys that work with dyn dispatch

A dyn dispatch receiver must be the size of a pointer, and a `CastKey`'s backing key can be any size. So `CastKey::as_dyn` borrows the key as a `DynKey<'_, T, K>`, a single fat pointer whose metadata is the key's vtable and whose address points at the backing key. That makes it a valid **method receiver** for trait objects.

```rust
trait Component: AnyHaver {
    fn tick(self: DynKey<'_, Self, DefaultKey>, world: &mut World);
}

let key: CastKey<dyn Component, DefaultKey> = upcast_key!(component_key);
key.as_dyn().tick(&mut world);   // virtual call through the key's own vtable
```

Inside the method, `self.key()` returns the `CastKey<Self, DefaultKey>` to look things up in the map. The dispatch itself never touches the map.

`upcast_key!` upcasts a key through a `DynKey`. Without a second argument, it upcasts to the type that the surrounding code expects, which here is the type of the `let` binding.

## License

MIT.