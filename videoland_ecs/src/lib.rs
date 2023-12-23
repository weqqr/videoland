#![allow(dead_code)]

use std::any::{TypeId, Any};
use std::cell::{RefCell, Ref, RefMut};

use ahash::HashMap;

pub struct Registry {
    resources: HashMap<TypeId, Box<RefCell<dyn Any>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::default(),
        }
    }

    pub fn res<R: 'static>(&self) -> Ref<R> {
        let id = TypeId::of::<R>();
        let r = self.resources.get(&id).unwrap().borrow();
        Ref::map(r, |x| x.downcast_ref().unwrap())
    }

    pub fn res_mut<R: 'static>(&self) -> RefMut<R> {
        let id = TypeId::of::<R>();
        let r = self.resources.get(&id).unwrap().borrow_mut();
        RefMut::map(r, |x| x.downcast_mut().unwrap())
    }
}

pub struct Entity {
    id: u32,
    generation: u32,
}

pub struct Archetype {
    // column types
    types: Vec<TypeId>,

    // len(entities) == len(component_table.column())
    entities: Vec<Entity>,
    component_table: Table,
}

impl Archetype {
    fn has(&self, ty: TypeId) -> bool {
        self.types.contains(&ty)
    }
}

pub struct Table {
    columns: Vec<Box<dyn Any>>,
}

pub trait Query {

}
