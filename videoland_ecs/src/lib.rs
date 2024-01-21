#![allow(dead_code)]

pub mod defer;
pub mod exec;
pub mod mq;
pub mod query;

pub use defer::*;
pub use exec::*;
pub use mq::*;
pub use query::*;

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};

use ahash::{AHashSet, HashMap};

pub struct Registry {
    resources: HashMap<TypeId, Box<RefCell<dyn Any>>>,
    defer_queue: RefCell<DeferQueue>,
    archetypes: Vec<Archetype>,
    step: Step,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::default(),
            defer_queue: RefCell::new(DeferQueue::new()),
            archetypes: Vec::new(),
            step: Step::new(0),
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

    pub fn spawn<B: Bundle>(&mut self, components: B) {
        let types = B::types();
        let archetype_index = self
            .archetypes
            .iter_mut()
            .position(|archetype| archetype.has_exactly(&types));

        let archetype_index = match archetype_index {
            Some(index) => index,
            None => {
                let index = self.archetypes.len();

                self.archetypes
                    .push(Archetype::new(AHashSet::from_iter(types)));

                index
            }
        };

        let archetype = &mut self.archetypes[archetype_index];

        let row_index = archetype.allocate_row();

        components.insert(row_index, archetype);
    }

    pub fn next_step(&mut self) {
        self.step.increment();
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Step(u64);

impl Step {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

pub trait Bundle {
    fn types() -> Vec<TypeId>;
    fn insert(self, index: usize, archetype: &mut Archetype);
}

impl<A: 'static, B: 'static> Bundle for (A, B) {
    fn types() -> Vec<TypeId> {
        vec![TypeId::of::<A>(), TypeId::of::<B>()]
    }

    fn insert(self, index: usize, archetype: &mut Archetype) {
        unsafe {
            (*archetype.column_mut::<A>().get()).insert(index, self.0);
            (*archetype.column_mut::<B>().get()).insert(index, self.1);
        }
    }
}

#[macro_export]
macro_rules! expand_macro_staircase {
    ($m:ident) => {
        $m!();
    };
    ($m:ident $ty:ident) => {
        $m!($ty);
    };
    ($m:ident $ty:ident, $($tys:ident),*) => {
        $m!($ty, $($tys),*);
        expand_macro_staircase!($m $($tys),*);
    };
}
