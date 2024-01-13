use videoland::ecs::{Defer, EventQueue, Query, Res, Schedule, Stage};
use videoland::input::InputState;
use videoland::winit::event::KeyEvent;
use videoland::winit::keyboard::KeyCode;
use videoland::{App, AppInfo};

use crate::control::{Action, ActionMap, Player};

mod control;

fn test(input_state: Res<InputState>, q: Query<(&String, &mut i32)>) {
    if input_state.is_key_pressed(KeyCode::KeyW) {
        println!("W pressed");
    }

    for (string, int) in q.iter() {
        println!("{string} {int}");
        *int = 51;
    }
}

fn init_systems(mut defer: Defer) {
    defer.insert(ActionMap::new());
    defer.insert(Player::new());
    defer.insert(EventQueue::<Action>::new());

    defer.spawn(("test".to_owned(), 2222i32));
}

fn main() {
    let mut schedule = Schedule::new();
    schedule.add_system(Stage::Init, init_systems);
    schedule.add_system(Stage::Frame, test);
    schedule.add_system(Stage::Frame, control::handle_input);
    schedule.add_system(Stage::Frame, control::move_player);
    schedule.add_system(Stage::Frame, videoland::sys::show_test_window);
    schedule.add_system(Stage::Frame, videoland::sys::prepare_ui);
    schedule.add_system(Stage::Frame, videoland::sys::render);
    schedule.add_system(Stage::Frame, control::update_engine_state);
    schedule.add_system(Stage::Frame, videoland::sys::clear_events::<KeyEvent>);
    schedule.add_system(Stage::Frame, videoland::sys::clear_events::<Action>);

    let app_info = AppInfo {
        internal_name: "dsots".to_owned(),
        title: "dsots".to_owned(),
    };

    App::new(schedule, app_info).run();
}
