#![allow(dead_code)]

pub mod exec;
pub mod query;

pub use exec::*;
pub use query::*;

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};

use ahash::HashMap;

pub struct Registry {
    resources: HashMap<TypeId, Box<RefCell<dyn Any>>>,
    archetypes: Vec<Archetype>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::default(),
            archetypes: Vec::new(),
        }
    }

    pub fn insert<R: 'static>(&mut self, r: R) {
        let id = TypeId::of::<R>();
        self.resources.insert(id, Box::new(RefCell::new(r)));
    }

    pub fn res<R: 'static>(&self) -> Ref<R> {
        let id = TypeId::of::<R>();
        let resource = self
            .resources
            .get(&id)
            .unwrap_or_else(|| {
                panic!("unknown resource: {}", std::any::type_name::<R>());
            })
            .borrow();
        Ref::map(resource, |x| x.downcast_ref().unwrap())
    }

    pub fn res_mut<R: 'static>(&self) -> RefMut<R> {
        let id = TypeId::of::<R>();
        let r = self
            .resources
            .get(&id)
            .unwrap_or_else(|| {
                panic!("unknown resource: {}", std::any::type_name::<R>());
            })
            .borrow_mut();
        RefMut::map(r, |x| x.downcast_mut().unwrap())
    }
}
