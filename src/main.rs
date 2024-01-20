use videoland::ecs::{Defer, EventQueue, Schedule, Stage};
use videoland::winit::event::KeyEvent;
use videoland::{App, AppInfo};

use crate::control::{Action, ActionMap, Player};

mod control;

fn init(mut defer: Defer) {
    defer.insert(ActionMap::new());
    defer.insert(Player::new());
    defer.insert(EventQueue::<Action>::new());
}

fn main() {
    let mut schedule = Schedule::new();

    schedule.add_system(Stage::Init, init);
    // schedule.add_system(Stage::Init, videoland_editor::init);
    schedule.add_system(Stage::EachStep, control::handle_input);
    schedule.add_system(Stage::EachStep, control::move_player);
    schedule.add_system(Stage::EachStep, videoland::sys::show_test_window);
    // schedule.add_system(Stage::EachStep, videoland_editor::show);
    schedule.add_system(Stage::EachStep, videoland::sys::prepare_ui);
    schedule.add_system(Stage::EachStep, videoland::sys::render);
    schedule.add_system(Stage::EachStep, control::update_engine_state);
    schedule.add_system(Stage::EachStep, videoland::sys::clear_events::<KeyEvent>);
    schedule.add_system(Stage::EachStep, videoland::sys::clear_events::<Action>);

    let app_info = AppInfo {
        internal_name: "dsots".to_owned(),
        title: "dsots".to_owned(),
    };

    App::new(schedule, app_info).run();
}
