use super::*;
use wasmtime::{Engine, Linker, Module, Store};
use std::sync::Arc;

#[test]
fn test_set_operations() {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    collection_ops::register_collection_functions(&mut linker);

    let store = Store::new(&engine, ());
    let mut store = store;

    // Test set creation
    let set_new = linker.get(&mut store, "env", "set_new").unwrap().into_func().unwrap();
    let set_ptr = set_new.call(&mut store, &[]).unwrap()[0].i32().unwrap();

    // Test set add
    let set_add = linker.get(&mut store, "env", "set_add").unwrap().into_func().unwrap();
    set_add.call(&mut store, &[Val::I32(set_ptr), Val::I32(1)]).unwrap();
    set_add.call(&mut store, &[Val::I32(set_ptr), Val::I32(2)]).unwrap();
    set_add.call(&mut store, &[Val::I32(set_ptr), Val::I32(3)]).unwrap();

    // Test set contains
    let set_contains = linker.get(&mut store, "env", "set_contains").unwrap().into_func().unwrap();
    assert_eq!(set_contains.call(&mut store, &[Val::I32(set_ptr), Val::I32(1)]).unwrap()[0].i32().unwrap(), 1);
    assert_eq!(set_contains.call(&mut store, &[Val::I32(set_ptr), Val::I32(2)]).unwrap()[0].i32().unwrap(), 1);
    assert_eq!(set_contains.call(&mut store, &[Val::I32(set_ptr), Val::I32(3)]).unwrap()[0].i32().unwrap(), 1);
    assert_eq!(set_contains.call(&mut store, &[Val::I32(set_ptr), Val::I32(4)]).unwrap()[0].i32().unwrap(), 0);

    // Test set size
    let set_size = linker.get(&mut store, "env", "set_size").unwrap().into_func().unwrap();
    assert_eq!(set_size.call(&mut store, &[Val::I32(set_ptr)]).unwrap()[0].i32().unwrap(), 3);

    // Test set remove
    let set_remove = linker.get(&mut store, "env", "set_remove").unwrap().into_func().unwrap();
    set_remove.call(&mut store, &[Val::I32(set_ptr), Val::I32(2)]).unwrap();
    assert_eq!(set_contains.call(&mut store, &[Val::I32(set_ptr), Val::I32(2)]).unwrap()[0].i32().unwrap(), 0);
    assert_eq!(set_size.call(&mut store, &[Val::I32(set_ptr)]).unwrap()[0].i32().unwrap(), 2);
}

#[test]
fn test_map_operations() {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    collection_ops::register_collection_functions(&mut linker);

    let store = Store::new(&engine, ());
    let mut store = store;

    // Test map creation
    let map_new = linker.get(&mut store, "env", "map_new").unwrap().into_func().unwrap();
    let map_ptr = map_new.call(&mut store, &[]).unwrap()[0].i32().unwrap();

    // Test map put
    let map_put = linker.get(&mut store, "env", "map_put").unwrap().into_func().unwrap();
    map_put.call(&mut store, &[Val::I32(map_ptr), Val::I32(1), Val::I32(100)]).unwrap();
    map_put.call(&mut store, &[Val::I32(map_ptr), Val::I32(2), Val::I32(200)]).unwrap();
    map_put.call(&mut store, &[Val::I32(map_ptr), Val::I32(3), Val::I32(300)]).unwrap();

    // Test map get
    let map_get = linker.get(&mut store, "env", "map_get").unwrap().into_func().unwrap();
    assert_eq!(map_get.call(&mut store, &[Val::I32(map_ptr), Val::I32(1)]).unwrap()[0].i32().unwrap(), 100);
    assert_eq!(map_get.call(&mut store, &[Val::I32(map_ptr), Val::I32(2)]).unwrap()[0].i32().unwrap(), 200);
    assert_eq!(map_get.call(&mut store, &[Val::I32(map_ptr), Val::I32(3)]).unwrap()[0].i32().unwrap(), 300);
    assert_eq!(map_get.call(&mut store, &[Val::I32(map_ptr), Val::I32(4)]).unwrap()[0].i32().unwrap(), 0);

    // Test map size
    let map_size = linker.get(&mut store, "env", "map_size").unwrap().into_func().unwrap();
    assert_eq!(map_size.call(&mut store, &[Val::I32(map_ptr)]).unwrap()[0].i32().unwrap(), 3);

    // Test map remove
    let map_remove = linker.get(&mut store, "env", "map_remove").unwrap().into_func().unwrap();
    map_remove.call(&mut store, &[Val::I32(map_ptr), Val::I32(2)]).unwrap();
    assert_eq!(map_get.call(&mut store, &[Val::I32(map_ptr), Val::I32(2)]).unwrap()[0].i32().unwrap(), 0);
    assert_eq!(map_size.call(&mut store, &[Val::I32(map_ptr)]).unwrap()[0].i32().unwrap(), 2);
}

#[test]
fn test_queue_operations() {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    collection_ops::register_collection_functions(&mut linker);

    let store = Store::new(&engine, ());
    let mut store = store;

    // Test queue creation
    let queue_new = linker.get(&mut store, "env", "queue_new").unwrap().into_func().unwrap();
    let queue_ptr = queue_new.call(&mut store, &[]).unwrap()[0].i32().unwrap();

    // Test queue enqueue
    let queue_enqueue = linker.get(&mut store, "env", "queue_enqueue").unwrap().into_func().unwrap();
    queue_enqueue.call(&mut store, &[Val::I32(queue_ptr), Val::I32(1)]).unwrap();
    queue_enqueue.call(&mut store, &[Val::I32(queue_ptr), Val::I32(2)]).unwrap();
    queue_enqueue.call(&mut store, &[Val::I32(queue_ptr), Val::I32(3)]).unwrap();

    // Test queue peek
    let queue_peek = linker.get(&mut store, "env", "queue_peek").unwrap().into_func().unwrap();
    assert_eq!(queue_peek.call(&mut store, &[Val::I32(queue_ptr)]).unwrap()[0].i32().unwrap(), 1);

    // Test queue size
    let queue_size = linker.get(&mut store, "env", "queue_size").unwrap().into_func().unwrap();
    assert_eq!(queue_size.call(&mut store, &[Val::I32(queue_ptr)]).unwrap()[0].i32().unwrap(), 3);

    // Test queue dequeue
    let queue_dequeue = linker.get(&mut store, "env", "queue_dequeue").unwrap().into_func().unwrap();
    assert_eq!(queue_dequeue.call(&mut store, &[Val::I32(queue_ptr)]).unwrap()[0].i32().unwrap(), 1);
    assert_eq!(queue_peek.call(&mut store, &[Val::I32(queue_ptr)]).unwrap()[0].i32().unwrap(), 2);
    assert_eq!(queue_size.call(&mut store, &[Val::I32(queue_ptr)]).unwrap()[0].i32().unwrap(), 2);
}

#[test]
fn test_stack_operations() {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    collection_ops::register_collection_functions(&mut linker);

    let store = Store::new(&engine, ());
    let mut store = store;

    // Test stack creation
    let stack_new = linker.get(&mut store, "env", "stack_new").unwrap().into_func().unwrap();
    let stack_ptr = stack_new.call(&mut store, &[]).unwrap()[0].i32().unwrap();

    // Test stack push
    let stack_push = linker.get(&mut store, "env", "stack_push").unwrap().into_func().unwrap();
    stack_push.call(&mut store, &[Val::I32(stack_ptr), Val::I32(1)]).unwrap();
    stack_push.call(&mut store, &[Val::I32(stack_ptr), Val::I32(2)]).unwrap();
    stack_push.call(&mut store, &[Val::I32(stack_ptr), Val::I32(3)]).unwrap();

    // Test stack peek
    let stack_peek = linker.get(&mut store, "env", "stack_peek").unwrap().into_func().unwrap();
    assert_eq!(stack_peek.call(&mut store, &[Val::I32(stack_ptr)]).unwrap()[0].i32().unwrap(), 3);

    // Test stack size
    let stack_size = linker.get(&mut store, "env", "stack_size").unwrap().into_func().unwrap();
    assert_eq!(stack_size.call(&mut store, &[Val::I32(stack_ptr)]).unwrap()[0].i32().unwrap(), 3);

    // Test stack pop
    let stack_pop = linker.get(&mut store, "env", "stack_pop").unwrap().into_func().unwrap();
    assert_eq!(stack_pop.call(&mut store, &[Val::I32(stack_ptr)]).unwrap()[0].i32().unwrap(), 3);
    assert_eq!(stack_peek.call(&mut store, &[Val::I32(stack_ptr)]).unwrap()[0].i32().unwrap(), 2);
    assert_eq!(stack_size.call(&mut store, &[Val::I32(stack_ptr)]).unwrap()[0].i32().unwrap(), 2);
} 