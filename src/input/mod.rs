use std::collections::HashMap;

use winit::event::{VirtualKeyCode, WindowEvent};

pub struct Input<E> {
    event_mapping: HashMap<VirtualKeyCode, E>,

    events: Vec<E>,
}

impl<E: Clone> Input<E> {
    pub fn new() -> Self {
        Self {
            event_mapping: HashMap::new(),

            events: Vec::new(),
        }
    }

    pub fn bind(&mut self, key: VirtualKeyCode, event: E) {
        self.event_mapping.insert(key, event);
    }

    pub fn submit_event(&mut self, event: &WindowEvent) {
        let WindowEvent::KeyboardInput { input, .. } = event else {
            return
        };

        let Some(key) = input.virtual_keycode else {
            return;
        };

        if let Some(event) = self.event_mapping.get(&key) {
            self.events.push(event.clone());
        }
    }

    pub fn events(&mut self) -> impl Iterator<Item = E> + '_ {
        self.events.drain(..)
    }
}
