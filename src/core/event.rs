use std::any::Any;
use std::cell::{Ref, RefMut};

use crate::core::{Defer, Registry, SystemParam};

pub trait AnyEventQueue {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear(&mut self);
}

pub struct EventQueue<E> {
    events: Vec<E>,
}

impl<E> EventQueue<E> {
    pub(super) fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn emit(&mut self, event: E) {
        self.events.push(event);
    }
}

impl<E: 'static> AnyEventQueue for EventQueue<E> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clear(&mut self) {
        self.events.clear();
    }
}

pub fn clear_events(mut defer: Defer) {
    defer.defer(|reg| {
        for queue in reg.event_queues.values() {
            queue.borrow_mut().clear();
        }
    });
}

pub struct Events<'a, E> {
    value: Ref<'a, EventQueue<E>>,
}

impl<'a, E: 'static> SystemParam for Events<'a, E> {
    type Item<'w> = Events<'w, E>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        Events {
            value: reg.event_queue::<E>(),
        }
    }
}

impl<E> Events<'_, E> {
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.value.events.iter()
    }
}

pub struct EventsMut<'a, E> {
    value: RefMut<'a, EventQueue<E>>,
}

impl<'a, E: 'static> SystemParam for EventsMut<'a, E> {
    type Item<'w> = EventsMut<'w, E>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        EventsMut {
            value: reg.event_queue_mut::<E>(),
        }
    }
}

impl<E> EventsMut<'_, E> {
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.value.events.iter()
    }

    pub fn emit(&mut self, event: E) {
        self.value.emit(event)
    }
}
