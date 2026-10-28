//! The id table: declare, bind, release.

use meridian_memory::ResourcePool;

use super::{BindError, Binding, ResourceIdTable};
use crate::ResourceId;

struct Texture;

fn id(pool: &mut ResourcePool<u8>) -> ResourceId<Texture> {
    ResourceId::new(pool.insert(0))
}

#[test]
fn a_declared_key_is_unbound_until_a_resource_is_bound() {
    let mut pool = ResourcePool::new();
    let mut table = ResourceIdTable::<u64, Texture>::declaring([1, 2]);
    assert_eq!(table.len(), 2);
    assert_eq!(table.binding(1), Some(Binding::Unbound));
    assert_eq!(table.binding(9), None);

    let texture = id(&mut pool);
    table.bind(1, texture).unwrap();
    assert_eq!(table.resource(1), Some(texture));
    assert_eq!(table.key_of(texture), Some(1));
    assert_eq!(table.resource(2), None);
}

#[test]
fn binding_is_refused_for_unknown_bound_and_shared() {
    let mut pool = ResourcePool::new();
    let mut table = ResourceIdTable::<u64, Texture>::declaring([1, 2]);
    let (a, b) = (id(&mut pool), id(&mut pool));
    assert_eq!(table.bind(9, a), Err(BindError::UnknownKey));
    table.bind(1, a).unwrap();
    assert_eq!(table.bind(1, b), Err(BindError::AlreadyBound));
    assert_eq!(table.bind(2, a), Err(BindError::ResourceInUse));
    assert_eq!(table.resource(2), None);
}

#[test]
fn unbinding_keeps_the_key_and_frees_the_resource_for_another() {
    let mut pool = ResourcePool::new();
    let mut table = ResourceIdTable::<u64, Texture>::declaring([1, 2]);
    let a = id(&mut pool);
    table.bind(1, a).unwrap();
    assert_eq!(table.unbind(1), Some(a));
    assert_eq!(table.unbind(1), None);
    assert_eq!(table.binding(1), Some(Binding::Unbound));
    assert_eq!(table.key_of(a), None);
    table.bind(2, a).unwrap();
}

#[test]
fn a_reloaded_resource_has_a_new_generation_and_the_old_one_is_stale() {
    let mut pool = ResourcePool::new();
    let mut table = ResourceIdTable::<u64, Texture>::declaring([1]);
    let first = id(&mut pool);
    table.bind(1, first).unwrap();
    table.unbind(1);
    pool.remove(first.handle);
    let second = id(&mut pool);
    assert_ne!(first, second);
    table.bind(1, second).unwrap();
    assert_eq!(table.key_of(first), None);
    assert_eq!(table.key_of(second), Some(1));
}

#[test]
fn declaring_twice_keeps_the_binding() {
    let mut pool = ResourcePool::new();
    let mut table = ResourceIdTable::<u64, Texture>::new();
    table.declare(1);
    let a = id(&mut pool);
    table.bind(1, a).unwrap();
    table.declare(1);
    assert_eq!(table.resource(1), Some(a));
}
