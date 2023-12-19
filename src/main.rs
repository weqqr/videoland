use videoland::App;
use videoland::camera::Camera;
use videoland::domain::{Transform, Player};
use videoland::ecs::{World, Entity};
use videoland::loader::Loader;
use videoland::math::{Quat, Vec3};

mod control;

struct Game {

}

impl App for Game {
    fn run(&mut self) {

    }
}

fn add_stuff_to_world(world: &mut World, loader: &Loader) -> Entity {
    // let sponza = world.spawn((Transform {
    //     position: Vec3::ZERO,
    //     rotation: Quat::IDENTITY,
    // },));
    // loader.load_and_attach_model_sync(sponza, "models/sponza.obj");

    let monkey = world.spawn((
        Transform {
            position: Vec3::Y * 100.0,
            rotation: Quat::IDENTITY,
        },
    ));
    loader.load_and_attach_model_sync(monkey, "models/monkey.obj");

    let flatplane = world.spawn((Transform {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
    },));
    loader.load_and_attach_model_sync(flatplane, "models/flatplane.obj");

    world.spawn((Player, Camera::new()))
}

fn main() {
    // control::move_player(
    //     &mut self.world,
    //     self.player,
    //     &self.input_state,
    //     &self.timings,
    // );

    let game = Game{};

    videoland::run();
}
