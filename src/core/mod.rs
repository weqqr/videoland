#![allow(dead_code)]

pub mod arena;
pub mod defer;
pub mod event;
pub mod exec;
pub mod query;

pub use arena::*;
pub use defer::*;
pub use event::*;
pub use exec::*;
pub use query::*;

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};

use ahash::HashMap;

pub struct Registry {
    resources: HashMap<TypeId, Box<RefCell<dyn Any>>>,
    event_queues: HashMap<TypeId, Box<RefCell<dyn AnyEventQueue>>>,
    defer_queue: RefCell<DeferQueue>,
    step: Step,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::default(),
            event_queues: HashMap::default(),
            defer_queue: RefCell::new(DeferQueue::new()),
            step: Step::new(0),
        }
    }

    pub fn insert<R: 'static>(&mut self, r: R) {
        let id = TypeId::of::<R>();
        self.resources.insert(id, Box::new(RefCell::new(r)));
    }

    pub fn register_event<E: 'static>(&mut self) {
        let id = TypeId::of::<E>();
        self.event_queues
            .insert(id, Box::new(RefCell::new(EventQueue::<E>::new())));
    }

    #[track_caller]
    pub fn event_queue<E: 'static>(&self) -> Ref<EventQueue<E>> {
        let id = TypeId::of::<E>();
        let event_queue = self
            .event_queues
            .get(&id)
            .unwrap_or_else(|| {
                panic!("unknown event: {}", std::any::type_name::<E>());
            })
            .borrow();

        Ref::map(event_queue, |x| x.as_any().downcast_ref().unwrap())
    }

    #[track_caller]
    pub fn event_queue_mut<E: 'static>(&self) -> RefMut<EventQueue<E>> {
        let id = TypeId::of::<E>();
        let event_queue = self
            .event_queues
            .get(&id)
            .unwrap_or_else(|| {
                panic!("unknown event: {}", std::any::type_name::<E>());
            })
            .borrow_mut();

        RefMut::map(event_queue, |x| x.as_any_mut().downcast_mut().unwrap())
    }

    #[track_caller]
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

    #[track_caller]
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

pub(crate) use expand_macro_staircase;
