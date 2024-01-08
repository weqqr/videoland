use videoland::ecs::{Res, Schedule, Query};
use videoland::input::InputState;
use videoland::winit::keyboard::KeyCode;
use videoland::{App, AppInfo};
use videoland::winit::event::KeyEvent;

mod control;

fn test(input_state: Res<InputState>, q: Query<(&String, &mut i32)>) {
    if input_state.is_key_pressed(KeyCode::KeyW) {
        println!("W pressed");
    }

    for (string, int) in q.iter() {
        *int = 51;
        println!("{string}");
    }
}

fn main() {
    let mut schedule = Schedule::new();
    schedule.add_system(test);
    schedule.add_system(videoland::sys::handle_input);
    schedule.add_system(videoland::sys::show_test_window);
    schedule.add_system(videoland::sys::prepare_ui);
    schedule.add_system(videoland::sys::render);
    schedule.add_system(videoland::sys::clear_events::<KeyEvent>);
    // schedule.add_system(control::move_player);

    let app_info = AppInfo {
        internal_name: "dsots".to_owned(),
        title: "dsots".to_owned(),
    };

    App::new(schedule, app_info).run();
}
