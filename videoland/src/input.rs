use ahash::AHashSet;
use glam::{vec2, Vec2};
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct InputState {
    held_keys: AHashSet<KeyCode>,
    held_mouse_buttons: AHashSet<MouseButton>,

    mouse_delta_since_last_frame: Vec2,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            held_keys: AHashSet::new(),
            held_mouse_buttons: AHashSet::new(),

            mouse_delta_since_last_frame: Vec2::ZERO,
        }
    }

    pub fn submit_window_input(&mut self, input: &WindowEvent) {
        match input {
            WindowEvent::KeyboardInput { event, .. } => {
                self.submit_key_input(event);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.submit_mouse_input(*state, *button);
            }
            _ => {}
        }
    }

    pub fn submit_device_input(&mut self, event: &DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.mouse_delta_since_last_frame += vec2(delta.0 as f32, delta.1 as f32);
            }
            _ => {}
        }
    }

    pub fn reset_mouse_movement(&mut self) {
        self.mouse_delta_since_last_frame = Vec2::ZERO;
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.held_keys.contains(&key)
    }

    pub fn is_mouse_button_pressed(&self, key: MouseButton) -> bool {
        self.held_mouse_buttons.contains(&key)
    }

    fn submit_key_input(&mut self, input: &KeyEvent) {
        let key_code = match input.physical_key {
            PhysicalKey::Code(code) => code,
            PhysicalKey::Unidentified(_) => return,
        };

        match input.state {
            ElementState::Pressed => {
                self.held_keys.insert(key_code);
            }
            ElementState::Released => {
                self.held_keys.remove(&key_code);
            }
        }
    }

    fn submit_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        match state {
            ElementState::Pressed => {
                self.held_mouse_buttons.insert(button);
            }
            ElementState::Released => {
                self.held_mouse_buttons.remove(&button);
            }
        }
    }
}
