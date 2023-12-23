use videoland::camera::Camera;
use videoland::ecs::{Entity, Registry};
use videoland::input::InputState;
use videoland::math::Vec3;
use videoland::timing::Timings;
use videoland::winit::keyboard::KeyCode;

pub enum Action {
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Sprint,
}

pub fn move_player(world: &mut Registry, player: Entity, input_state: &InputState, timings: &Timings) {
    let mut movement_dir = Vec3::ZERO;

    let speed = 4.0; // in m/s
    let distance = speed * timings.dtime_s() as f32;

    // let mut player_components = world.query_one::<&mut Camera>(player).unwrap();
    // let camera = player_components.get().unwrap();
    let mut camera = Camera::new();

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
