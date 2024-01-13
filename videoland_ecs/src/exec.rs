use std::marker::PhantomData;

use ahash::AHashMap;

use crate::{expand_macro_staircase, Registry};

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

pub trait System {
    fn run(&mut self, reg: &Registry);
}

// user code will have to specify type parameters in add_system without this
pub trait IntoSystem<I, S> {
    fn into_system(self) -> S;
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Stage {
    Init,
    Frame,
}

pub struct Schedule {
    systems: AHashMap<Stage, Vec<Box<dyn System>>>,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: AHashMap::new(),
        }
    }

    pub fn add_system<I, S: System + 'static>(&mut self, stage: Stage, s: impl IntoSystem<I, S>) {
        let systems = self.systems.entry(stage).or_default();
        systems.push(Box::new(s.into_system()));
    }

    pub fn execute(&mut self, stage: Stage, reg: &mut Registry) {
        for system in self.systems.entry(stage).or_default() {
            system.run(reg);
            let mut defer_queue = reg.defer_queue.replace(Default::default());
            defer_queue.apply(reg);
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

expand_macro_staircase!(impl_system_for_systemfn A, B, C, D, E, F, G, H, I, J, K, L, M, N);

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

expand_macro_staircase!(impl_into_system_for_fn A, B, C, D, E, F, G, H, I, J, K, L, M, N);
