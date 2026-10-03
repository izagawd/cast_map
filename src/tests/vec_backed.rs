//! Tests for a backing map that wraps a `Vec`, uses `usize` keys and only
//! implements [`Map`] and `IntoIterator`.

use std::any::Any;

use super::{Cat, Dog};
use crate::any_haver::AnyHaver;
use crate::cast_key::CastKey;
use crate::cast_map::CastMapG;
use crate::dyn_key::DynKey;
use crate::map::Map;
use crate::type_tagged_ptr::TypeTaggedBox;
use crate::unsafe_cast_map::UnsafeCastMapG;
use crate::upcast_key;

/// A map over a `Vec` whose keys are indices. Removing leaves `None` behind and
/// inserting always pushes, so a key never finds another value.
struct VecMap<V> {
    slots: Vec<Option<V>>,
    len: usize,
}

// SAFETY: a value stays at its index until a method takes it out, and every
// method reads the element at the key's index.
unsafe impl<V: 'static> Map for VecMap<V> {
    type Key = usize;
    type Value = V;
    type Iter<'a>
        = Box<dyn Iterator<Item = (usize, &'a V)> + 'a>
    where
        Self: 'a,
        V: 'a;
    type IterMut<'a>
        = Box<dyn Iterator<Item = (usize, &'a mut V)> + 'a>
    where
        Self: 'a,
        V: 'a;
    type Keys<'a>
        = Box<dyn Iterator<Item = usize> + 'a>
    where
        Self: 'a,
        V: 'a;
    type Values<'a>
        = Box<dyn Iterator<Item = &'a V> + 'a>
    where
        Self: 'a,
        V: 'a;
    type ValuesMut<'a>
        = Box<dyn Iterator<Item = &'a mut V> + 'a>
    where
        Self: 'a,
        V: 'a;
    type Drain<'a>
        = std::vec::IntoIter<(usize, V)>
    where
        Self: 'a;

    fn empty() -> Self {
        Self {
            slots: Vec::new(),
            len: 0,
        }
    }
    fn len(&self) -> usize {
        self.len
    }
    fn clear(&mut self) {
        self.slots.iter_mut().for_each(|slot| *slot = None);
        self.len = 0;
    }
    fn get(&self, key: usize) -> Option<&V> {
        self.slots.get(key)?.as_ref()
    }
    fn get_mut(&mut self, key: usize) -> Option<&mut V> {
        self.slots.get_mut(key)?.as_mut()
    }
    fn remove(&mut self, key: usize) -> Option<V> {
        let value = self.slots.get_mut(key)?.take()?;
        self.len -= 1;
        Some(value)
    }
    fn retain<F: FnMut(usize, &mut V) -> bool>(&mut self, mut f: F) {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if let Some(value) = slot {
                if !f(index, value) {
                    *slot = None;
                    self.len -= 1;
                }
            }
        }
    }
    fn insert(&mut self, value: V) -> usize {
        self.slots.push(Some(value));
        self.len += 1;
        self.slots.len() - 1
    }
    fn keys(&self) -> Self::Keys<'_> {
        Box::new(self.iter().map(|(key, _)| key))
    }
    fn values(&self) -> Self::Values<'_> {
        Box::new(self.slots.iter().flatten())
    }
    fn values_mut(&mut self) -> Self::ValuesMut<'_> {
        Box::new(self.slots.iter_mut().flatten())
    }
    fn iter(&self) -> Self::Iter<'_> {
        Box::new(
            self.slots
                .iter()
                .enumerate()
                .filter_map(|(index, slot)| Some((index, slot.as_ref()?))),
        )
    }
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        Box::new(
            self.slots
                .iter_mut()
                .enumerate()
                .filter_map(|(index, slot)| Some((index, slot.as_mut()?))),
        )
    }
    fn drain(&mut self) -> Self::Drain<'_> {
        // The values are taken out right away, so `len` stays right even if
        // the caller drops the iterator early.
        let drained: Vec<(usize, V)> = self
            .slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| Some((index, slot.take()?)))
            .collect();
        self.len = 0;
        drained.into_iter()
    }
}

impl<V: 'static> IntoIterator for VecMap<V> {
    type Item = (usize, V);
    type IntoIter = Box<dyn Iterator<Item = (usize, V)>>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            self.slots
                .into_iter()
                .enumerate()
                .filter_map(|(index, slot)| Some((index, slot?))),
        )
    }
}

type AnyVecMap = CastMapG<VecMap<TypeTaggedBox<dyn Any>>>;

/// Inserts `value` as a `dyn Any` and returns a typed key for it.
fn insert_typed<T: Any>(map: &mut AnyVecMap, value: T) -> CastKey<T, usize> {
    let erased = map.insert(TypeTaggedBox::from_any(Box::new(value)));
    map.downcast_key(erased.inner_key()).unwrap()
}

trait Speak: AnyHaver + Any {
    fn speak(self: DynKey<'_, Self, usize>, map: &AnyVecMap) -> String;
}

impl Speak for Dog {
    fn speak(self: DynKey<'_, Self, usize>, map: &AnyVecMap) -> String {
        format!("woof {}", map.get(self.key()).unwrap().name)
    }
}

impl Speak for Cat {
    fn speak(self: DynKey<'_, Self, usize>, map: &AnyVecMap) -> String {
        format!("meow x{}", map.get(self.key()).unwrap().lives)
    }
}

#[test]
fn backing_keys_match_iter() {
    fn check<M: Map<Value = u32>>()
    where
        M::Key: PartialEq + std::fmt::Debug,
    {
        let mut map = M::empty();
        let a = map.insert(1);
        let b = map.insert(2);
        let c = map.insert(3);
        map.remove(b);
        let keys: Vec<M::Key> = map.keys().collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&a) && keys.contains(&c));
        assert_eq!(keys, map.iter().map(|(k, _)| k).collect::<Vec<_>>());
    }
    #[cfg(feature = "slotmap")]
    check::<slotmap::SlotMap<crate::DefaultKey, u32>>();
    #[cfg(feature = "slotmap")]
    check::<slotmap::DenseSlotMap<crate::DefaultKey, u32>>();
    #[cfg(feature = "gen_map")]
    check::<gen_map::GenMap<u32>>();
    check::<VecMap<u32>>();
}

#[test]
fn cast_map_works_on_core_trait_alone() {
    let mut map = AnyVecMap::new();
    let dog = insert_typed(&mut map, Dog { name: "Rex".into() });
    let cat_erased = map.insert(TypeTaggedBox::from_any(Box::new(Cat { lives: 9 })));
    let pet: CastKey<dyn Speak, usize> =
        upcast_key!(insert_typed(&mut map, Dog { name: "Ax".into() }));
    assert_eq!(map.len(), 3);
    assert_eq!(
        (dog.inner_key(), cat_erased.inner_key(), pet.inner_key()),
        (0, 1, 2)
    );

    assert_eq!(map.get(dog).unwrap().name, "Rex");
    assert!(map.get(pet).is_some());
    let cat: CastKey<Cat, usize> = map.downcast_key(cat_erased.inner_key()).unwrap();
    map.get_mut(cat).unwrap().lives -= 1;
    assert_eq!(map[cat].lives, 8);

    // A key whose type does not match its value is rejected.
    let wrong: CastKey<Cat, usize> = CastKey::from_raw_parts(dog.inner_key(), ());
    assert!(map.get(wrong).is_none());
    assert!(map.remove(wrong).is_none());

    assert_eq!(map.iter().count(), 3);
    let removed: TypeTaggedBox<Dog> = map.remove(dog).unwrap();
    assert_eq!(removed.name, "Rex");
    assert!(map.get(dog).is_none());
    assert_eq!(map.len(), 2);

    // The removed value's index is not reused, so its key stays dead.
    let next = insert_typed(&mut map, Dog { name: "Max".into() });
    assert_eq!(next.inner_key(), 3);
    assert!(map.get(dog).is_none());

    map.retain(|_, v| !v.is::<Cat>());
    assert_eq!(map.len(), 2);
    assert_eq!(map.drain().count(), 2);
    assert!(map.is_empty());
}

#[cfg(feature = "coerce_unsized")]
#[test]
fn coercing_inserts_work_on_core_trait_alone() {
    use crate::type_tagged_ptr::ConcreteTypeId;

    let mut map = AnyVecMap::new();
    let dog: CastKey<Dog, usize> = map.insert_sized(TypeTaggedBox::new(Dog { name: "Rex".into() }));
    let pet: CastKey<dyn Speak, usize> =
        map.insert_as(TypeTaggedBox::new(Cat { lives: 9 }) as TypeTaggedBox<dyn Speak>);
    let erased = map.insert(TypeTaggedBox::new(Dog { name: "Ax".into() }));
    assert_eq!(
        (dog.inner_key(), pet.inner_key(), erased.inner_key()),
        (0, 1, 2)
    );

    assert_eq!(map.get(dog).unwrap().name, "Rex");
    assert_eq!(pet.as_dyn().speak(&map), "meow x9");
    let removed: TypeTaggedBox<dyn Speak> = map.remove(pet).unwrap();
    assert_eq!(removed.concrete_type_id(), std::any::TypeId::of::<Cat>());
    assert_eq!(map.len(), 2);
}

#[test]
fn owned_into_iter_uses_backing_into_iterator() {
    let mut map = AnyVecMap::new();
    map.insert(TypeTaggedBox::from_any(Box::new(Cat { lives: 1 })));
    let removed = map.insert(TypeTaggedBox::from_any(Box::new(Cat { lives: 5 })));
    map.insert(TypeTaggedBox::from_any(Box::new(Cat { lives: 2 })));
    map.remove_by_inner_key(removed.inner_key());

    let pairs: Vec<(usize, u32)> = map
        .into_iter()
        .map(|(k, v)| (k.inner_key(), v.downcast_ref::<Cat>().unwrap().lives))
        .collect();
    assert_eq!(pairs, [(0, 1), (2, 2)]);
}

#[test]
fn unsafe_map_with_usize_keys() {
    let mut map: UnsafeCastMapG<VecMap<Box<dyn Any>>> = UnsafeCastMapG::new();
    let erased = map.insert(Box::new(Dog { name: "U".into() }));
    // The value under the key is a `Dog`, and a `Dog` key needs no metadata.
    let key: CastKey<Dog, usize> = CastKey::from_raw_parts(erased.inner_key(), ());

    // SAFETY: the value under `key` is the `Dog` that `key` was made for.
    assert_eq!(unsafe { map.get(key) }.unwrap().name, "U");

    // SAFETY: same key, and the value is still there.
    let removed: Box<Dog> = unsafe { map.remove(key) }.unwrap();
    assert_eq!(removed.name, "U");
    assert!(map.is_empty());
}

#[test]
fn dyn_key_with_usize_keys() {
    let mut map = AnyVecMap::new();
    map.insert(TypeTaggedBox::from_any(Box::new(Cat { lives: 1 })));
    let dog = insert_typed(&mut map, Dog { name: "Rex".into() });
    let cat = insert_typed(&mut map, Cat { lives: 9 });

    // `key()` reads the `usize` back, before and after an unsizing coercion.
    assert_eq!(dog.as_dyn().key(), dog);
    let erased: DynKey<'_, dyn Speak, usize> = dog.as_dyn();
    assert_eq!(erased.key(), upcast_key!(dog, dyn Speak));
    assert_eq!(erased.key().inner_key(), 1);

    // Dispatch goes through the vtable that each key carries.
    let pets: [CastKey<dyn Speak, usize>; 2] = [upcast_key!(dog), upcast_key!(cat)];
    let spoken: Vec<String> = pets.iter().map(|k| k.as_dyn().speak(&map)).collect();
    assert_eq!(spoken, ["woof Rex", "meow x9"]);

    // A downcast keeps the backing key.
    let back: DynKey<'_, Dog, usize> = erased.downcast::<Dog>().unwrap();
    assert_eq!(back.key(), dog);
    assert!(erased.downcast::<Cat>().is_none());
}

#[test]
fn upcast_key_predicts_the_target() {
    struct Holder {
        pet: CastKey<dyn Speak, usize>,
    }

    fn index_of(pet: CastKey<dyn Speak, usize>) -> usize {
        pet.inner_key()
    }

    fn erase(dog: CastKey<Dog, usize>) -> CastKey<dyn Speak, usize> {
        upcast_key!(dog)
    }

    let mut map = AnyVecMap::new();
    let dog = insert_typed(&mut map, Dog { name: "Rex".into() });

    // Each of these expects a `CastKey<dyn Speak, usize>`, so the macro
    // upcasts to `dyn Speak` without being told.
    let from_let: CastKey<dyn Speak, usize> = upcast_key!(dog);
    let holder = Holder {
        pet: upcast_key!(dog),
    };
    let mut pushed: Vec<CastKey<dyn Speak, usize>> = Vec::new();
    pushed.push(upcast_key!(dog, _));
    let closure = || -> CastKey<dyn Speak, usize> { upcast_key!(&dog) };
    let pets = [from_let, holder.pet, pushed[0], closure(), erase(dog)];
    for pet in pets {
        assert_eq!(pet.as_dyn().speak(&map), "woof Rex");
    }
    assert_eq!(index_of(upcast_key!(dog)), dog.inner_key());

    // Nothing expects a particular type here, so the key keeps its own type,
    // and `get` returns a `&Dog`.
    let same = upcast_key!(dog);
    assert_eq!(map.get(same).unwrap().name, "Rex");
}
