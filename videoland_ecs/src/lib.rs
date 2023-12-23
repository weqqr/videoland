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

pub struct ResMut<'a, T: 'static> {
    value: RefMut<'a, T>,
}

pub struct SystemFn<F, FnParams> {
    func: F,

    // needed to constrain input types for System impl
    _pd: PhantomData<FnParams>,
}

impl<F: FnMut(T, U), T: SystemParam + 'static, U: SystemParam + 'static> System
    for SystemFn<F, (T, U)>
where
    for<'a, 'b> &'a mut F: FnMut(T, U) + FnMut(SystemParamItem<'b, T>, SystemParamItem<'b, U>),
{
    fn run(&mut self, reg: &Registry) {
        let a = T::get(reg);
        let b = U::get(reg);

        fn call_inner<T, U>(mut f: impl FnMut(T, U), t: T, u: U) {
            f(t, u)
        }

        call_inner(&mut self.func, a, b)
    }
}

impl<F, T, U> IntoSystem<(T, U), SystemFn<Self, (T, U)>> for F
where
    T: 'static,
    U: 'static,
    F: FnMut(T, U),
{
    fn into_system(self) -> SystemFn<Self, (T, U)> {
        SystemFn {
            func: self,
            _pd: PhantomData,
        }
    }
}

// whoever invented this is a genius
// https://promethia-27.github.io/dependency_injection_like_bevy_from_scratch/chapter2/passing_references.html
pub trait SystemParam {
    type Item<'w>;

    fn get(reg: &Registry) -> Self::Item<'_>;
}

type SystemParamItem<'w, T> = <T as SystemParam>::Item<'w>;

impl<'a, T> SystemParam for Res<'a, T> {
    type Item<'w> = Res<'w, T>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        Res {
            value: reg.res::<T>(),
        }
    }
}

impl<'a, T> SystemParam for ResMut<'a, T> {
    type Item<'w> = ResMut<'w, T>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        ResMut {
            value: reg.res_mut::<T>(),
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
