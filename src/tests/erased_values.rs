//! Tests for `TypeTaggedPtr::from_any` and `from_any_haver`, which work without
//! the `coerce_unsized` feature.

use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::Arc;

use super::{Cat, Dog};
use crate::any_haver::AnyHaver;
use crate::type_tagged_ptr::{ConcreteTypeId, TypeTaggedBox, TypeTaggedPtr};

trait Pet: AnyHaver {}

impl Pet for Dog {}

impl Pet for Cat {}

#[test]
fn from_any_records_the_concrete_type() {
    let boxed: TypeTaggedBox<dyn Any> =
        TypeTaggedBox::from_any(Box::new(Dog { name: "Rex".into() }));
    assert_eq!(boxed.concrete_type_id(), TypeId::of::<Dog>());
    assert!(boxed.is::<Dog>());

    let shared: TypeTaggedPtr<Rc<dyn Any + Send>> = TypeTaggedPtr::from_any(Rc::new(5u32));
    assert_eq!(shared.concrete_type_id(), TypeId::of::<u32>());

    let synced: TypeTaggedPtr<Arc<dyn Any + Send + Sync>> =
        TypeTaggedPtr::from_any(Arc::new(Cat { lives: 9 }));
    assert_eq!(synced.concrete_type_id(), TypeId::of::<Cat>());

    let value = 1.5f64;
    let borrowed: TypeTaggedPtr<&dyn Any> = TypeTaggedPtr::from_any(&value);
    assert_eq!(borrowed.concrete_type_id(), TypeId::of::<f64>());

    // The recorded type is the type of the value, and not `dyn Any` or the
    // type of the pointer.
    assert_ne!(boxed.concrete_type_id(), TypeId::of::<dyn Any>());
    assert_ne!(boxed.concrete_type_id(), TypeId::of::<Box<dyn Any>>());
}

#[test]
fn from_any_haver_records_the_concrete_type() {
    let pet: TypeTaggedBox<dyn Pet> = TypeTaggedBox::from_any_haver(Box::new(Cat { lives: 9 }));
    assert_eq!(pet.concrete_type_id(), TypeId::of::<Cat>());

    let sized: TypeTaggedBox<Dog> =
        TypeTaggedBox::from_any_haver(Box::new(Dog { name: "Rex".into() }));
    assert_eq!(sized.concrete_type_id(), TypeId::of::<Dog>());
}

#[cfg(feature = "coerce_unsized")]
#[test]
fn constructors_match_the_coercion() {
    let coerced: TypeTaggedBox<dyn Any> = TypeTaggedBox::new(Dog { name: "Rex".into() });
    let wrapped: TypeTaggedBox<dyn Any> =
        TypeTaggedBox::from_any(Box::new(Dog { name: "Rex".into() }));
    assert_eq!(coerced.concrete_type_id(), wrapped.concrete_type_id());

    let coerced: TypeTaggedBox<dyn Pet> = TypeTaggedBox::new(Cat { lives: 1 });
    let wrapped: TypeTaggedBox<dyn Pet> = TypeTaggedBox::from_any_haver(Box::new(Cat { lives: 1 }));
    assert_eq!(coerced.concrete_type_id(), wrapped.concrete_type_id());
}

#[cfg(feature = "slotmap")]
#[test]
fn slotmap_without_coercion() {
    use crate::cast_key::CastKey;
    use crate::cast_map::BoxCastMap;
    use crate::DefaultKey;

    let mut map: BoxCastMap<DefaultKey, dyn Any> = BoxCastMap::new();
    let erased = map.insert(TypeTaggedBox::from_any(Box::new(Dog {
        name: "Rex".into(),
    })));
    let dog: CastKey<Dog, DefaultKey> = map.downcast_key(erased.inner_key()).unwrap();
    assert_eq!(map.get(dog).unwrap().name, "Rex");
    assert!(map.downcast_key::<Cat>(erased.inner_key()).is_none());

    // A `dyn Pet` key has `AnyHaver`, so the checked lookup takes it as it is,
    // and it downcasts without the map.
    let mut pets: BoxCastMap<DefaultKey, dyn Pet> = BoxCastMap::new();
    let cat = pets.insert(TypeTaggedBox::from_any_haver(Box::new(Cat { lives: 9 })));
    assert!(pets.get(cat).is_some());
    let typed: CastKey<Cat, DefaultKey> = cat.downcast().unwrap();
    assert_eq!(pets.get(typed).unwrap().lives, 9);
    assert!(cat.downcast::<Dog>().is_none());
}

#[cfg(feature = "gen_map")]
#[test]
fn gen_map_without_coercion() {
    use gen_map::Key;

    use crate::cast_key::CastKey;
    use crate::cast_map::BoxGenCastMap;

    let mut map: BoxGenCastMap<dyn Any> = BoxGenCastMap::new();
    let erased = map.insert(TypeTaggedBox::from_any(Box::new(Cat { lives: 9 })));
    let cat: CastKey<Cat, Key> = map.downcast_key(erased.inner_key()).unwrap();
    map.get_mut(cat).unwrap().lives -= 1;
    assert_eq!(map[cat].lives, 8);

    let mut pets: BoxGenCastMap<dyn Pet> = BoxGenCastMap::new();
    let dog = pets.insert(TypeTaggedBox::from_any_haver(Box::new(Dog {
        name: "Rex".into(),
    })));
    let typed: CastKey<Dog, Key> = dog.downcast().unwrap();
    assert_eq!(pets.remove(typed).unwrap().name, "Rex");
    assert!(pets.is_empty());
}
