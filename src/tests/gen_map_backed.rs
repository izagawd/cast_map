//! Tests for the cast maps over `gen_map::GenMap`.

use std::any::Any;

use gen_map::{GenMap, GenMapConfig, GenSlotItem, Key, MapConfig, MapKeyConfig, Packed, Split};

use super::{Cat, Dog};
use crate::any_haver::AnyHaver;
use crate::cast_key::CastKey;
use crate::cast_map::{BoxGenCastMap, CastMapG};
use crate::dyn_key::DynKey;
use crate::type_tagged_ptr::TypeTaggedBox;
use crate::unsafe_cast_map::{UnsafeBoxGenCastMap, UnsafeGenCastMap};
use crate::upcast_key;

type AnyMap = BoxGenCastMap<dyn Any>;

/// Maps with this config hand out four byte keys with 24 bits of index and 8
/// bits of generation.
struct Compact;

impl MapConfig for Compact {
    type KeyConfig = Packed<u32, 8>;
}

impl<S: GenSlotItem> GenMapConfig<S> for Compact {
    type Storage = Vec<S>;
}

/// Maps with this config hand out sixteen byte keys.
struct Wide;

impl MapConfig for Wide {
    type KeyConfig = Split<u64, u64>;
}

impl<S: GenSlotItem> GenMapConfig<S> for Wide {
    type Storage = Vec<S>;
}

/// Maps with this config keep their slots in an `ArrayVec`, which has room for
/// four slots and never grows.
struct Four;

impl MapConfig for Four {
    type KeyConfig = Split<u8, u8>;
}

impl<S: GenSlotItem> GenMapConfig<S> for Four {
    type Storage = arrayvec::ArrayVec<S, 4>;
}

trait Pet: AnyHaver + Any {
    fn speak(self: DynKey<'_, Self, Key>, map: &AnyMap) -> String;
}

impl Pet for Dog {
    fn speak(self: DynKey<'_, Self, Key>, map: &AnyMap) -> String {
        format!("woof {}", map.get(self.key()).unwrap().name)
    }
}

impl Pet for Cat {
    fn speak(self: DynKey<'_, Self, Key>, map: &AnyMap) -> String {
        format!("meow x{}", map.get(self.key()).unwrap().lives)
    }
}

#[test]
fn typed_lookups_check_the_type() {
    let mut map = AnyMap::new();
    let dog: CastKey<Dog, Key> = map.insert_sized(TypeTaggedBox::new(Dog { name: "Rex".into() }));
    let cat_erased = map.insert(TypeTaggedBox::new(Cat { lives: 9 }));
    let cat: CastKey<Cat, Key> = map.downcast_key(cat_erased.inner_key()).unwrap();
    assert!(map.downcast_key::<Dog>(cat_erased.inner_key()).is_none());

    assert_eq!(map.get(dog).unwrap().name, "Rex");
    map.get_mut(cat).unwrap().lives -= 1;
    assert_eq!(map[cat].lives, 8);
    assert!(map.contains_key(cat));

    // A key whose type does not match its value is rejected everywhere.
    let wrong: CastKey<Cat, Key> = CastKey::from_raw_parts(dog.inner_key(), ());
    assert!(map.get(wrong).is_none());
    assert!(map.get_mut(wrong).is_none());
    assert!(!map.contains_key(wrong));
    assert!(map.remove(wrong).is_none());
    assert!(map.detach(wrong).is_none());
    assert_eq!(map.len(), 2);

    // `CastKey::downcast` works on the key alone.
    let pet: CastKey<dyn Pet, Key> = upcast_key!(dog);
    assert_eq!(pet.downcast::<Dog>(), Some(dog));
    assert!(pet.downcast::<Cat>().is_none());
}

#[test]
fn remove_makes_the_key_stale() {
    let mut map = AnyMap::new();
    let first: CastKey<Cat, Key> = map.insert_sized(TypeTaggedBox::new(Cat { lives: 1 }));
    let removed: TypeTaggedBox<Cat> = map.remove(first).unwrap();
    assert_eq!(removed.lives, 1);

    // The new value takes the freed slot under a newer generation, so the old
    // key does not find it.
    let second: CastKey<Cat, Key> = map.insert_sized(TypeTaggedBox::new(Cat { lives: 2 }));
    assert_eq!(second.inner_key().idx(), first.inner_key().idx());
    assert!(map.get(first).is_none());
    assert_eq!(map.get(second).unwrap().lives, 2);
    assert!(map.cast_key_of(first.inner_key()).is_none());

    map.clear();
    assert!(map.get(second).is_none());
    assert!(map.is_empty());
}

#[test]
fn detach_and_reattach() {
    let mut map = AnyMap::new();
    let key: CastKey<Dog, Key> = map.insert_sized(TypeTaggedBox::new(Dog { name: "Rex".into() }));

    let mut dog: TypeTaggedBox<Dog> = map.detach(key).unwrap();
    assert!(map.get(key).is_none());
    assert!(map.is_empty());

    // An insert does not take the detached slot.
    let other = map.insert(TypeTaggedBox::new(Cat { lives: 3 }));
    assert_ne!(other.inner_key().idx(), key.inner_key().idx());

    dog.name = "Max".into();
    assert!(map.reattach_by_inner_key(key.inner_key(), dog).is_ok());
    assert_eq!(map.get(key).unwrap().name, "Max");

    // `GenMap` hands the value back for a key that is not detached.
    let refused =
        map.reattach_by_inner_key(other.inner_key(), TypeTaggedBox::new(Cat { lives: 7 }));
    assert_eq!(
        refused.err().unwrap().downcast_ref::<Cat>(),
        Some(&Cat { lives: 7 })
    );

    // A value of another type can go back under the key, and the old typed
    // key then stops resolving.
    let erased: TypeTaggedBox<dyn Any> = map.detach_by_inner_key(key.inner_key()).unwrap();
    assert!(erased.is::<Dog>());
    assert!(map
        .reattach_by_inner_key(key.inner_key(), TypeTaggedBox::new(Cat { lives: 9 }))
        .is_ok());
    assert!(map.get(key).is_none());
    let cat: CastKey<Cat, Key> = map.downcast_key(key.inner_key()).unwrap();
    assert_eq!(map.get(cat).unwrap().lives, 9);
}

#[test]
fn disjoint_access() {
    let mut map = BoxGenCastMap::<u32>::new();
    let a: CastKey<u32, Key> = map.insert(TypeTaggedBox::new(1));
    let b: CastKey<u32, Key> = map.insert(TypeTaggedBox::new(2));

    let [x, y] = map.get_disjoint_mut([a, b]).unwrap();
    std::mem::swap(x, y);
    assert_eq!((map[a], map[b]), (2, 1));
    assert!(map.get_disjoint_mut([a, a]).is_none());

    let [z] = map.get_disjoint_mut_by_inner_key([b.inner_key()]).unwrap();
    *z += 10;
    assert_eq!(map[b], 11);

    // A key whose type does not match its value is rejected.
    let wrong: CastKey<Cat, Key> = CastKey::from_raw_parts(a.inner_key(), ());
    let mut any_map = AnyMap::new();
    any_map.insert(TypeTaggedBox::new(5u32));
    assert!(any_map.get_disjoint_mut([wrong]).is_none());
}

#[test]
fn capacity_and_reserve() {
    let mut map = AnyMap::with_capacity(8);
    assert!(map.capacity() >= 8);
    map.reserve(32);
    assert!(map.capacity() >= 32);
    map.try_reserve(64).unwrap();
    assert!(map.capacity() >= 64);
    assert!(map.try_reserve(usize::MAX).is_err());
}

#[test]
fn insert_with_key_sees_its_key() {
    let mut map = AnyMap::new();
    let mut seen = None;
    let key = map.insert_sized_with_key(|key: CastKey<Dog, Key>| {
        seen = Some(key);
        TypeTaggedBox::new(Dog { name: "Wk".into() })
    });
    assert_eq!(seen, Some(key));
    assert_eq!(map.get(key).unwrap().name, "Wk");

    let own = map.insert_with_key(|key| TypeTaggedBox::new(key) as TypeTaggedBox<dyn Any>);
    let typed: CastKey<Key, Key> = map.downcast_key(own.inner_key()).unwrap();
    assert_eq!(*map.get(typed).unwrap(), own.inner_key());

    let result: Result<CastKey<Dog, Key>, &str> =
        map.try_insert_sized_with_key(|_| Err::<TypeTaggedBox<Dog>, _>("nope"));
    assert_eq!(result.err(), Some("nope"));
    assert_eq!(map.len(), 2);
}

#[test]
fn iterators_retain_drain_and_into_iter() {
    let mut map = AnyMap::new();
    map.insert(TypeTaggedBox::new(Cat { lives: 1 }));
    map.insert(TypeTaggedBox::new(Cat { lives: 9 }));
    map.insert(TypeTaggedBox::new(Dog { name: "z".into() }));
    assert_eq!(map.iter().count(), 3);
    assert_eq!(map.keys().count(), 3);
    assert_eq!(map.values().count(), 3);

    for (_, value) in map.iter_mut() {
        if let Some(cat) = value.downcast_mut::<Cat>() {
            cat.lives += 1;
        }
    }
    map.retain(|_, value| value.downcast_ref::<Cat>().is_some_and(|cat| cat.lives > 5));
    assert_eq!(map.len(), 1);

    let drained: Vec<_> = map.drain().collect();
    assert_eq!(drained.len(), 1);
    assert!(map.is_empty());

    map.insert(TypeTaggedBox::new(Cat { lives: 4 }));
    let lives: Vec<u32> = map
        .into_iter()
        .map(|(_, value)| value.downcast_ref::<Cat>().unwrap().lives)
        .collect();
    assert_eq!(lives, [4]);
}

/// Checks that `DynKey` gets the same key back for a map with config `C`,
/// directly and after an unsizing coercion.
fn check_dyn_keys<C: MapConfigFor<TypeTaggedBox<dyn Any>>>() {
    type K<C> = Key<MapKeyConfig<C>>;
    let mut map: CastMapG<GenMap<TypeTaggedBox<dyn Any>, C>> = CastMapG::new();
    let dog: CastKey<Dog, K<C>> = map.insert_sized(TypeTaggedBox::new(Dog { name: "Rex".into() }));
    let cat: CastKey<Cat, K<C>> = map.insert_sized(TypeTaggedBox::new(Cat { lives: 9 }));
    map.remove(dog);
    let dog: CastKey<Dog, K<C>> = map.insert_sized(TypeTaggedBox::new(Dog { name: "Ax".into() }));

    assert_eq!(dog.as_dyn().key(), dog);
    let erased: DynKey<'_, dyn Any, K<C>> = cat.as_dyn();
    assert_eq!(erased.key(), upcast_key!(cat, dyn Any));
    assert_eq!(map.get(dog.as_dyn().key()).unwrap().name, "Ax");
    assert_eq!(map.get(cat.as_dyn().key()).unwrap().lives, 9);
}

#[test]
fn dyn_keys_work_for_every_key_size() {
    assert_eq!(std::mem::size_of::<Key<MapKeyConfig<Compact>>>(), 4);
    assert_eq!(std::mem::size_of::<Key>(), 8);
    assert_eq!(std::mem::size_of::<Key<MapKeyConfig<Wide>>>(), 16);
    check_dyn_keys::<Compact>();
    check_dyn_keys::<gen_map::DefaultMapConfig>();
    check_dyn_keys::<Wide>();
}

#[test]
fn dyn_key_dispatch() {
    let mut map = AnyMap::new();
    let dog: CastKey<Dog, Key> = map.insert_sized(TypeTaggedBox::new(Dog { name: "Rex".into() }));
    let cat: CastKey<Cat, Key> = map.insert_sized(TypeTaggedBox::new(Cat { lives: 9 }));

    let pets: [CastKey<dyn Pet, Key>; 2] = [upcast_key!(dog), upcast_key!(cat)];
    let spoken: Vec<String> = pets.iter().map(|k| k.as_dyn().speak(&map)).collect();
    assert_eq!(spoken, ["woof Rex", "meow x9"]);

    let dk: DynKey<'_, dyn Pet, Key> = pets[0].as_dyn();
    let back: DynKey<'_, Dog, Key> = dk.downcast::<Dog>().unwrap();
    assert_eq!(back.speak(&map), "woof Rex");
    assert!(dk.downcast::<Cat>().is_none());

    // `insert_as` keeps the source type in the key.
    let pet: CastKey<dyn Pet, Key> =
        map.insert_as(TypeTaggedBox::new(Cat { lives: 2 }) as TypeTaggedBox<dyn Pet>);
    assert_eq!(pet.as_dyn().speak(&map), "meow x2");
    let removed: TypeTaggedBox<dyn Pet> = map.remove(pet).unwrap();
    drop(removed);
}

#[test]
fn fixed_capacity_storage() {
    let mut map: CastMapG<GenMap<TypeTaggedBox<dyn Any>, Four>> = CastMapG::new();
    assert_eq!(map.capacity(), 4);
    let keys: Vec<CastKey<u32, Key<MapKeyConfig<Four>>>> = (0..4u32)
        .map(|i| map.insert_sized(TypeTaggedBox::new(i)))
        .collect();
    assert_eq!(*map.get(keys[3]).unwrap(), 3);

    // A removed value frees its slot for the next insert.
    map.remove(keys[0]);
    let again: CastKey<u32, Key<MapKeyConfig<Four>>> = map.insert_sized(TypeTaggedBox::new(9));
    assert_eq!(*map.get(again).unwrap(), 9);
    assert_eq!(map.capacity(), 4);
}

#[test]
#[should_panic(expected = "GenMap is full")]
fn full_map_panics_on_insert() {
    let mut map: CastMapG<GenMap<TypeTaggedBox<dyn Any>, Four>> = CastMapG::new();
    for i in 0..5u32 {
        map.insert(TypeTaggedBox::new(i));
    }
}

#[test]
#[should_panic(expected = "GenMap is full")]
fn full_map_panics_on_try_insert_with_key() {
    let mut map: CastMapG<GenMap<TypeTaggedBox<dyn Any>, Four>> = CastMapG::new();
    for i in 0..4u32 {
        map.insert(TypeTaggedBox::new(i));
    }
    let _ = map.try_insert_with_key(|_| -> Result<TypeTaggedBox<dyn Any>, ()> {
        unreachable!("the closure is not called for a full map")
    });
}

#[test]
fn unsafe_map_over_gen_map() {
    let mut map: UnsafeBoxGenCastMap<dyn Any> = UnsafeBoxGenCastMap::new();
    let key: CastKey<Dog, Key> = map.insert_sized(Box::new(Dog { name: "U".into() }));

    // SAFETY: the value under `key` is the `Dog` that `key` was made for.
    assert_eq!(unsafe { map.get(key) }.unwrap().name, "U");

    // SAFETY: same key, and the value is still there.
    let removed: Box<Dog> = unsafe { map.remove(key) }.unwrap();
    assert_eq!(removed.name, "U");
    // SAFETY: no value is under `key` anymore, so the lookup finds nothing
    // and never uses the metadata.
    assert!(unsafe { map.get(key) }.is_none());
}

#[test]
fn clone_keeps_keys_working() {
    let mut map: UnsafeGenCastMap<Box<u32>> = UnsafeGenCastMap::new();
    let key: CastKey<u32, Key> = map.insert(Box::new(7));
    let clone = map.clone();
    // SAFETY: the clone keeps the same `u32` under `key`.
    assert_eq!(unsafe { clone.get(key) }, Some(&7));
}
