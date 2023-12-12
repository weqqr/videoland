use glam::Vec3;
use hecs::{Entity, World};
use winit::keyboard::KeyCode;

use crate::camera::Camera;
use crate::input::InputState;
use crate::timing::Timings;

pub enum Action {
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Sprint,
}

pub fn move_player(world: &mut World, player: Entity, input_state: &InputState, timings: &Timings) {
    let mut movement_dir = Vec3::ZERO;

    let speed = 4.0; // in m/s
    let distance = speed * timings.dtime_s() as f32;

    let mut player_components = world.query_one::<&mut Camera>(player).unwrap();
    let camera = player_components.get().unwrap();

    let (forward, right) = camera.forward_right();
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
    camera.position += movement_dir * distance;
}
