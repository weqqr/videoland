#![allow(dead_code)]

use std::any::{TypeId, Any};
use std::cell::{RefCell, Ref, RefMut};
use std::marker::PhantomData;

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
    fn query(&self);
}

pub struct Res<'a, T: 'static> {
    value: Ref<'a, T>,
}

pub struct SystemFn<F, FnParams> {
    func: F,

    // needed to constrain input types for System impl
    _pd: PhantomData<FnParams>,
}

impl<F: FnMut(Res<T>, Res<U>), T: 'static, U: 'static> System for SystemFn<F, (T, U)> {
    fn run(&mut self, reg: &Registry) {
        let a = reg.res::<T>();
        let b = reg.res::<U>();

        (self.func)(Res { value: a }, Res { value: b })
    }
}

impl<F: FnMut(Res<T>, Res<U>), T: 'static, U: 'static> From<F> for SystemFn<F, (T, U)> {
    fn from(func: F) -> Self {
        Self {
            func,
            _pd: PhantomData,
        }
    }
}

pub trait System {
    fn run(&mut self, reg: &Registry);
}

pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add_system<S: System + 'static>(&mut self, s: impl Into<S>) {
        self.systems.push(Box::new(s.into()));
    }

    pub fn execute(&mut self, reg: &Registry) {
        for system in &mut self.systems {
            system.run(reg);
        }
    }
}
