use videoland::EngineState;
use videoland::camera::Camera;
use videoland::ecs::{Res, ResMut, Events};
use videoland::input::InputState;
use videoland::math::Vec3;
use videoland::timing::Timings;
use videoland::winit::event::KeyEvent;
use videoland::winit::keyboard::{KeyCode, NamedKey, Key};

pub enum Action {
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Sprint,
}

pub struct Player {
    camera: Camera,
}

pub fn handle_input(mut engine_state: ResMut<EngineState>, input: Events<KeyEvent>) {
    for key in input.iter() {
        if let Key::Named(NamedKey::Escape) = key.logical_key {
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
