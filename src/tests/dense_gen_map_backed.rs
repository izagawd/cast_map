//! Tests for the cast maps over `gen_map::DenseGenMap`. They build each
//! `TypeTaggedBox<dyn Any>` with `TypeTaggedBox::from_any` instead of a
//! coercion, so they run without the `coerce_unsized` feature.

use std::any::Any;

use gen_map::{DenseGenMap, Key};

use super::{Cat, Dog};
use crate::cast_key::CastKey;
use crate::cast_map::BoxDenseGenCastMap;
use crate::type_tagged_ptr::TypeTaggedBox;
use crate::unsafe_cast_map::UnsafeBoxDenseGenCastMap;

type AnyMap = BoxDenseGenCastMap<dyn Any>;

/// Inserts `value` as a `dyn Any` and returns a typed key for it.
fn insert_typed<T: Any>(map: &mut AnyMap, value: T) -> CastKey<T, Key> {
    let erased = map.insert(TypeTaggedBox::from_any(Box::new(value)));
    map.downcast_key(erased.inner_key()).unwrap()
}

#[test]
fn keys_follow_the_values_that_remove_moves() {
    let mut map = AnyMap::new();
    let rex = insert_typed(&mut map, Dog { name: "Rex".into() });
    let tom = insert_typed(&mut map, Cat { lives: 9 });
    let max = insert_typed(&mut map, Dog { name: "Max".into() });

    // Removing `rex` moves `max`, the last value, into its place.
    let removed: TypeTaggedBox<Dog> = map.remove(rex).unwrap();
    assert_eq!(removed.name, "Rex");
    assert_eq!(
        map.inner_keys().collect::<Vec<_>>(),
        [max.inner_key(), tom.inner_key()]
    );
    assert!(map.get(rex).is_none());
    assert_eq!(map.get(max).unwrap().name, "Max");
    assert_eq!(map.get(tom).unwrap().lives, 9);

    // A key whose type does not match its value is rejected.
    let wrong: CastKey<Cat, Key> = CastKey::from_raw_parts(max.inner_key(), ());
    assert!(map.get(wrong).is_none());
    assert!(map.remove(wrong).is_none());
    assert_eq!(map.len(), 2);

    map.clear();
    assert!(map.is_empty());
    assert!(map.get(tom).is_none());
}

#[test]
fn detach_and_attach() {
    let mut map = AnyMap::new();
    let key = insert_typed(&mut map, Dog { name: "Rex".into() });
    let other = insert_typed(&mut map, Cat { lives: 3 });

    let dog: TypeTaggedBox<Dog> = map.detach(key).unwrap();
    assert_eq!(dog.name, "Rex");
    assert!(map.get(key).is_none());
    assert_eq!(map.len(), 1);

    let max: TypeTaggedBox<dyn Any> = TypeTaggedBox::from_any(Box::new(Dog { name: "Max".into() }));
    assert!(map.attach_by_inner_key(key.inner_key(), max).is_ok());
    assert_eq!(map.get(key).unwrap().name, "Max");

    // `DenseGenMap` hands the value back for a key that is not detached.
    let cat: TypeTaggedBox<dyn Any> = TypeTaggedBox::from_any(Box::new(Cat { lives: 7 }));
    let refused = map.attach_by_inner_key(other.inner_key(), cat);
    assert_eq!(
        refused.err().unwrap().downcast_ref::<Cat>(),
        Some(&Cat { lives: 7 })
    );
}

#[test]
fn disjoint_access() {
    let mut map = BoxDenseGenCastMap::<u32>::new();
    let a: CastKey<u32, Key> = map.insert(TypeTaggedBox::new(1));
    let b: CastKey<u32, Key> = map.insert(TypeTaggedBox::new(2));

    let [x, y] = map.get_disjoint_mut([a, b]).unwrap();
    std::mem::swap(x, y);
    assert_eq!((map[a], map[b]), (2, 1));
    assert!(map.get_disjoint_mut([a, a]).is_none());

    let [z] = map.get_disjoint_mut_by_inner_key([b.inner_key()]).unwrap();
    *z += 10;
    assert_eq!(map[b], 11);
}

#[test]
fn capacity_and_reserve() {
    let mut map = AnyMap::with_capacity(8);
    assert!(map.capacity() >= 8);
    map.try_reserve(32).unwrap();
    assert!(map.capacity() >= 32);
    assert!(map.try_reserve(usize::MAX).is_err());
}

#[test]
fn insert_with_key_sees_its_key() {
    let mut map = AnyMap::new();
    let own = map.insert_with_key(|key| TypeTaggedBox::from_any(Box::new(key)));
    let typed: CastKey<Key, Key> = map.downcast_key(own.inner_key()).unwrap();
    assert_eq!(*map.get(typed).unwrap(), own.inner_key());

    let result = map.try_insert_with_key(|_| Err::<TypeTaggedBox<dyn Any>, _>("nope"));
    assert_eq!(result.err(), Some("nope"));
    assert_eq!(map.len(), 1);
}

#[test]
fn iterators_retain_drain_and_into_iter() {
    let mut map = AnyMap::new();
    insert_typed(&mut map, Cat { lives: 1 });
    insert_typed(&mut map, Cat { lives: 9 });
    insert_typed(&mut map, Dog { name: "z".into() });
    assert_eq!(map.iter().count(), 3);
    assert_eq!(map.keys().count(), 3);
    assert_eq!(map.values().count(), 3);

    for value in map.values_mut() {
        if let Some(cat) = value.downcast_mut::<Cat>() {
            cat.lives += 1;
        }
    }
    map.retain(|_, value| value.downcast_ref::<Cat>().is_some_and(|cat| cat.lives > 5));
    assert_eq!(map.len(), 1);

    let drained: Vec<_> = map.drain().collect();
    assert_eq!(drained.len(), 1);
    assert!(map.is_empty());

    insert_typed(&mut map, Cat { lives: 4 });
    let lives: Vec<u32> = map
        .into_iter()
        .map(|(_, value)| value.downcast_ref::<Cat>().unwrap().lives)
        .collect();
    assert_eq!(lives, [4]);
}

#[test]
fn unsafe_map_round_trip() {
    let mut map: UnsafeBoxDenseGenCastMap<dyn Any> = UnsafeBoxDenseGenCastMap::new();
    let erased = map.insert(Box::new(Dog { name: "U".into() }));
    // The value under the key is a `Dog`, and a `Dog` key needs no metadata.
    let key: CastKey<Dog, Key> = CastKey::from_raw_parts(erased.inner_key(), ());

    // SAFETY: the value under `key` is the `Dog` that `key` was made for.
    assert_eq!(unsafe { map.get(key) }.unwrap().name, "U");

    // SAFETY: same key, and the value is still there.
    let removed: Box<Dog> = unsafe { map.remove(key) }.unwrap();
    assert_eq!(removed.name, "U");
    assert!(map.is_empty());
}

#[test]
fn converts_to_and_from_the_backing_map() {
    let mut map = AnyMap::new();
    let dog = insert_typed(&mut map, Dog { name: "Rex".into() });

    // The values stay in place, so the keys of the cast map still work after
    // the round trip.
    let backing: DenseGenMap<TypeTaggedBox<dyn Any>> = map.into();
    assert_eq!(backing.len(), 1);
    let map = AnyMap::from(backing);
    assert_eq!(map.get(dog).unwrap().name, "Rex");

    let unsafe_map = UnsafeBoxDenseGenCastMap::<dyn Any>::from(DenseGenMap::new());
    let backing: DenseGenMap<Box<dyn Any>> = unsafe_map.into();
    assert!(backing.is_empty());
}
