use wasmtime::{Caller, Linker};
use wasmtime::Val;
use std::collections::{HashMap, HashSet, VecDeque};
use std::vec::Vec;


/// Set implementation
pub struct Set<T> {
    data: HashSet<T>,
}

impl<T: Eq + std::hash::Hash> Set<T> {
    pub fn new() -> Self {
        Set {
            data: HashSet::new(),
        }
    }

    pub fn add(&mut self, value: T) {
        self.data.insert(value);
    }

    pub fn remove(&mut self, value: T) {
        self.data.remove(&value);
    }

    pub fn contains(&self, value: &T) -> bool {
        self.data.contains(value)
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

/// Map implementation
pub struct Map<K, V> {
    data: HashMap<K, V>,
}

impl<K: Eq + std::hash::Hash, V> Map<K, V> {
    pub fn new() -> Self {
        Map {
            data: HashMap::new(),
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }

    pub fn remove(&mut self, key: &K) {
        self.data.remove(key);
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.data.iter()
    }
}

/// Queue implementation
pub struct Queue<T> {
    data: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            data: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, value: T) {
        self.data.push_back(value);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        self.data.pop_front()
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.front()
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

/// Stack implementation
pub struct Stack<T> {
    data: Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack {
            data: Vec::new(),
        }
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.last()
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

/// Register collection type functions
pub fn register_collection_functions(_linker: &mut Linker<()>) {
    // TODO: Fix Wasmtime API compatibility issues
    /*
    // Set functions
    linker.func_wrap("env", "set_new", |caller: Caller<'_, _>| {
        let set = Set::new();
        Ok(Val::I32(set as *const _ as i32))
    }).unwrap();

    linker.func_wrap("env", "set_add", |caller: Caller<'_, _>, set_ptr: i32, value: i32| {
        let set = unsafe { &mut *(set_ptr as *mut Set<i32>) };
        set.add(value);
        Ok(())
    }).unwrap();

    linker.func_wrap("env", "set_remove", |caller: Caller<'_, _>, set_ptr: i32, value: i32| {
        let set = unsafe { &mut *(set_ptr as *mut Set<i32>) };
        set.remove(value);
        Ok(())
    }).unwrap();

    linker.func_wrap("env", "set_contains", |caller: Caller<'_, _>, set_ptr: i32, value: i32| {
        let set = unsafe { &*(set_ptr as *const Set<i32>) };
        Ok(Val::I32(set.contains(&value) as i32))
    }).unwrap();

    linker.func_wrap("env", "set_size", |caller: Caller<'_, _>, set_ptr: i32| {
        let set = unsafe { &*(set_ptr as *const Set<i32>) };
        Ok(Val::I32(set.size() as i32))
    }).unwrap();

    // Map functions
    linker.func_wrap("env", "map_new", |caller: Caller<'_, _>| {
        let map = Map::new();
        Ok(Val::I32(map as *const _ as i32))
    }).unwrap();

    linker.func_wrap("env", "map_put", |caller: Caller<'_, _>, map_ptr: i32, key: i32, value: i32| {
        let map = unsafe { &mut *(map_ptr as *mut Map<i32, i32>) };
        map.put(key, value);
        Ok(())
    }).unwrap();

    linker.func_wrap("env", "map_get", |caller: Caller<'_, _>, map_ptr: i32, key: i32| {
        let map = unsafe { &*(map_ptr as *const Map<i32, i32>) };
        Ok(Val::I32(map.get(&key).map_or(0, |&v| v)))
    }).unwrap();

    linker.func_wrap("env", "map_remove", |caller: Caller<'_, _>, map_ptr: i32, key: i32| {
        let map = unsafe { &mut *(map_ptr as *mut Map<i32, i32>) };
        map.remove(&key);
        Ok(())
    }).unwrap();

    linker.func_wrap("env", "map_size", |caller: Caller<'_, _>, map_ptr: i32| {
        let map = unsafe { &*(map_ptr as *const Map<i32, i32>) };
        Ok(Val::I32(map.size() as i32))
    }).unwrap();

    // Queue functions
    linker.func_wrap("env", "queue_new", |caller: Caller<'_, _>| {
        let queue = Queue::new();
        Ok(Val::I32(queue as *const _ as i32))
    }).unwrap();

    linker.func_wrap("env", "queue_enqueue", |caller: Caller<'_, _>, queue_ptr: i32, value: i32| {
        let queue = unsafe { &mut *(queue_ptr as *mut Queue<i32>) };
        queue.enqueue(value);
        Ok(())
    }).unwrap();

    linker.func_wrap("env", "queue_dequeue", |caller: Caller<'_, _>, queue_ptr: i32| {
        let queue = unsafe { &mut *(queue_ptr as *mut Queue<i32>) };
        Ok(Val::I32(queue.dequeue().unwrap_or(0)))
    }).unwrap();

    linker.func_wrap("env", "queue_peek", |caller: Caller<'_, _>, queue_ptr: i32| {
        let queue = unsafe { &*(queue_ptr as *const Queue<i32>) };
        Ok(Val::I32(queue.peek().map_or(0, |&v| v)))
    }).unwrap();

    linker.func_wrap("env", "queue_size", |caller: Caller<'_, _>, queue_ptr: i32| {
        let queue = unsafe { &*(queue_ptr as *const Queue<i32>) };
        Ok(Val::I32(queue.size() as i32))
    }).unwrap();

    // Stack functions
    linker.func_wrap("env", "stack_new", |caller: Caller<'_, _>| {
        let stack = Stack::new();
        Ok(Val::I32(stack as *const _ as i32))
    }).unwrap();

    linker.func_wrap("env", "stack_push", |caller: Caller<'_, _>, stack_ptr: i32, value: i32| {
        let stack = unsafe { &mut *(stack_ptr as *mut Stack<i32>) };
        stack.push(value);
        Ok(())
    }).unwrap();

    linker.func_wrap("env", "stack_pop", |caller: Caller<'_, _>, stack_ptr: i32| {
        let stack = unsafe { &mut *(stack_ptr as *mut Stack<i32>) };
        Ok(Val::I32(stack.pop().unwrap_or(0)))
    }).unwrap();

    linker.func_wrap("env", "stack_peek", |caller: Caller<'_, _>, stack_ptr: i32| {
        let stack = unsafe { &*(stack_ptr as *const Stack<i32>) };
        Ok(Val::I32(stack.peek().map_or(0, |&v| v)))
    }).unwrap();

    linker.func_wrap("env", "stack_size", |caller: Caller<'_, _>, stack_ptr: i32| {
        let stack = unsafe { &*(stack_ptr as *const Stack<i32>) };
        Ok(Val::I32(stack.size() as i32))
    }).unwrap();
    */
} 