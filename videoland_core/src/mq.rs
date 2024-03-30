use std::cell::{Ref, RefMut};

use crate::{Registry, ResMut, SystemParam};

pub struct EventQueue<E> {
    events: Vec<E>,
}

impl<E> EventQueue<E> {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn emit(&mut self, event: E) {
        self.events.push(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

pub fn clear_events<E>(mut events: ResMut<EventQueue<E>>) {
    events.clear();
}

pub struct Events<'a, E> {
    value: Ref<'a, EventQueue<E>>,
}

impl<'a, E: 'static> SystemParam for Events<'a, E> {
    type Item<'w> = Events<'w, E>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        Events {
            value: reg.res::<EventQueue<E>>(),
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
            value: reg.res_mut::<EventQueue<E>>(),
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
