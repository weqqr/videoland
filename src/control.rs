use std::collections::HashMap;

use videoland::camera::Camera;
use videoland::ecs::{Events, EventsMut, Res, ResMut};
use videoland::input::InputState;
use videoland::math::Vec3;
use videoland::timing::Timings;
use videoland::winit::event::KeyEvent;
use videoland::winit::keyboard::{KeyCode, PhysicalKey};
use videoland::EngineState;

#[derive(Clone)]
pub enum Action {
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Quit,
}

pub struct ActionMap {
    keys: HashMap<KeyCode, Action>,
}

impl ActionMap {
    pub fn new() -> Self {
        let mut keys = HashMap::new();
        keys.insert(KeyCode::KeyW, Action::MoveForward);
        keys.insert(KeyCode::KeyA, Action::MoveLeft);
        keys.insert(KeyCode::KeyS, Action::MoveBack);
        keys.insert(KeyCode::KeyD, Action::MoveRight);
        keys.insert(KeyCode::Space, Action::MoveUp);
        keys.insert(KeyCode::ShiftLeft, Action::MoveDown);
        keys.insert(KeyCode::KeyQ, Action::Quit);
        keys.insert(KeyCode::Escape, Action::Quit);

        Self { keys }
    }

    pub fn action_for_key(&self, key: KeyCode) -> Option<Action> {
        self.keys.get(&key).cloned()
    }
}

pub struct Player {
    camera: Camera,
}

impl Player {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
        }
    }
}

pub fn handle_input(
    action_map: Res<ActionMap>,
    input: Events<KeyEvent>,
    mut actions: EventsMut<Action>,
) {
    for key in input.iter() {
        let PhysicalKey::Code(key) = key.physical_key else {
            continue;
        };

        if let Some(action) = action_map.action_for_key(key) {
            actions.emit(action);
        }
    }
}

pub fn update_engine_state(mut engine_state: ResMut<EngineState>, actions: Events<Action>) {
    for action in actions.iter() {
        if let Action::Quit = action {
            engine_state.quit = true;
        }
    }
}

pub fn move_player(
    input_state: Res<InputState>,
    timings: Res<Timings>,
    mut player: ResMut<Player>,
) {
    let mut movement_dir = Vec3::ZERO;

    let speed = 1.5; // in m/s
    let distance = speed * timings.dtime_s() as f32;

    let (forward, right) = player.camera.forward_right();
    let forward = forward.normalize();
    let right = right.normalize();

    if input_state.is_key_pressed(KeyCode::KeyW) {
        movement_dir += forward;
    }

    if input_state.is_key_pressed(KeyCode::KeyS) {
        movement_dir -= forward;
    }

    if input_state.is_key_pressed(KeyCode::KeyD) {
        movement_dir += right;
    }

    if input_state.is_key_pressed(KeyCode::KeyA) {
        movement_dir -= right;
    }

    if input_state.is_key_pressed(KeyCode::Space) {
        movement_dir += Vec3::Y;
    }

    if input_state.is_key_pressed(KeyCode::ShiftLeft) {
        movement_dir -= Vec3::Y;
    }

    movement_dir = movement_dir.normalize_or_zero();
    player.camera.position += movement_dir * distance;
}
