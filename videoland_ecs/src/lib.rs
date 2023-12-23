#![allow(dead_code)]

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
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

impl<F: , T, U> IntoSystem<(T, U), SystemFn<Self, (T, U)>> for F
where
    T: 'static,
    U: 'static,
    F: FnMut(Res<T>, Res<U>)
{
    fn into_system(self) -> SystemFn<Self, (T, U)> {
        SystemFn {
            func: self,
            _pd: PhantomData,
        }
    }
}

pub trait System {
    fn run(&mut self, reg: &Registry);
}

// user code will have to specify type parameters in add_system without this
pub trait IntoSystem<I, S> {
    fn into_system(self) -> S;
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

    pub fn add_system<I, S: System + 'static>(&mut self, s: impl IntoSystem<I, S>) {
        self.systems.push(Box::new(s.into_system()));
    }

    pub fn execute(&mut self, reg: &Registry) {
        for system in &mut self.systems {
            system.run(reg);
        }
    }
}
