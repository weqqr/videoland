use videoland::input::InputState;
use videoland::winit::keyboard::KeyCode;
use videoland::{App, AppInfo};
use videoland::ecs::{Registry, Schedule, Res};
use videoland::loader::Loader;

mod control;

fn add_stuff_to_world(registry: &mut Registry, loader: &Loader) {
    // let sponza = world.spawn((Transform {
    //     position: Vec3::ZERO,
    //     rotation: Quat::IDENTITY,
    // },));
    // loader.load_and_attach_model_sync(sponza, "models/sponza.obj");
}

fn test(input_state: Res<InputState>) {
    if input_state.is_key_pressed(KeyCode::KeyW) {
        println!("W pressed");
    }
}

fn main() {
    let mut schedule = Schedule::new();
    schedule.add_system(test);
    schedule.add_system(control::move_player);

    let app_info = AppInfo {
        internal_name: "dsots".to_owned(),
        title: "dsots".to_owned(),
    };

    App::new(schedule, app_info).run();
}
