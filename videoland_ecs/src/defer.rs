use std::cell::RefMut;

use crate::{Registry, SystemParam};

pub type DeferredFn = Box<dyn FnOnce(&mut Registry)>;

#[derive(Default)]
pub struct DeferQueue {
    callbacks: Vec<DeferredFn>,
}

impl DeferQueue {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    pub fn apply(&mut self, reg: &mut Registry) {
        for callback in self.callbacks.drain(..) {
            callback(reg);
        }
    }
}

pub struct Defer<'a> {
    queue: RefMut<'a, DeferQueue>,
}

impl<'a> SystemParam for Defer<'a> {
    type Item<'w> = Defer<'w>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        Defer {
            queue: reg.defer_queue.borrow_mut(),
        }
    }
}

impl<'a> Defer<'a> {
    pub fn defer(&mut self, f: impl FnOnce(&mut Registry) + 'static) {
        self.queue.callbacks.push(Box::new(f));
    }

    pub fn insert<R: 'static>(&mut self, r: R) {
        self.defer(|reg| {
            reg.insert(r);
        })
    }
}
