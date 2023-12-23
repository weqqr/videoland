#![allow(dead_code)]

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

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

    pub fn insert<R: 'static>(&mut self, r: R) {
        let id = TypeId::of::<R>();
        self.resources.insert(id, Box::new(RefCell::new(r)));
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

impl<'a, T> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Ref::deref(&self.value)
    }
}

pub struct ResMut<'a, T: 'static> {
    value: RefMut<'a, T>,
}

impl<'a, T> Deref for ResMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        RefMut::deref(&self.value)
    }
}

impl<'a, T> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        RefMut::deref_mut(&mut self.value)
    }
}

pub struct SystemFn<F, FnParams> {
    func: F,

    // needed to constrain input types for System impl
    _pd: PhantomData<FnParams>,
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

macro_rules! impl_system_for_systemfn {
    ($($ts:ident),*) => {
        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        #[allow(clippy::too_many_arguments)]
        impl<Func: FnMut($($ts),*), $($ts: SystemParam + 'static),*> System
            for SystemFn<Func, ($($ts,)*)>
        where
            for<'a, 'b> &'a mut Func:
                FnMut($($ts),*) + FnMut($(SystemParamItem<'b, $ts>),*),
        {
            fn run(&mut self, reg: &Registry) {
                $(
                    let $ts = $ts::get(reg);
                )*

                fn call_inner<$($ts),*>(mut f: impl FnMut($($ts),*), $($ts:$ts),*) {
                    f($($ts),*)
                }

                call_inner(&mut self.func, $($ts),*)
            }
        }
    };
}

macro_rules! impl_into_system_for_fn {
    ($($ts:ident),*) => {
        impl<Func, $($ts,)*> IntoSystem<($($ts,)*), SystemFn<Self, ($($ts,)*)>> for Func
        where
            Func: FnMut($($ts),*),
            $($ts: 'static,)*
        {
            fn into_system(self) -> SystemFn<Self, ($($ts,)*)> {
                SystemFn {
                    func: self,
                    _pd: PhantomData,
                }
            }
        }
    }
}

impl_system_for_systemfn!();
impl_system_for_systemfn!(A);
impl_system_for_systemfn!(A, B);
impl_system_for_systemfn!(A, B, C);
impl_system_for_systemfn!(A, B, C, D);
impl_system_for_systemfn!(A, B, C, D, E);
impl_system_for_systemfn!(A, B, C, D, E, F);
impl_system_for_systemfn!(A, B, C, D, E, F, G);
impl_system_for_systemfn!(A, B, C, D, E, F, G, H);
impl_system_for_systemfn!(A, B, C, D, E, F, G, H, I);
impl_system_for_systemfn!(A, B, C, D, E, F, G, H, I, J);
impl_system_for_systemfn!(A, B, C, D, E, F, G, H, I, J, K);
impl_system_for_systemfn!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_system_for_systemfn!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_system_for_systemfn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);

impl_into_system_for_fn!();
impl_into_system_for_fn!(A);
impl_into_system_for_fn!(A, B);
impl_into_system_for_fn!(A, B, C);
impl_into_system_for_fn!(A, B, C, D);
impl_into_system_for_fn!(A, B, C, D, E);
impl_into_system_for_fn!(A, B, C, D, E, F);
impl_into_system_for_fn!(A, B, C, D, E, F, G);
impl_into_system_for_fn!(A, B, C, D, E, F, G, H);
impl_into_system_for_fn!(A, B, C, D, E, F, G, H, I);
impl_into_system_for_fn!(A, B, C, D, E, F, G, H, I, J);
impl_into_system_for_fn!(A, B, C, D, E, F, G, H, I, J, K);
impl_into_system_for_fn!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_into_system_for_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_into_system_for_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
