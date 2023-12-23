use videoland::ecs::Registry;
use videoland::loader::Loader;

mod control;

fn add_stuff_to_world(registry: &mut Registry, loader: &Loader) {
    // let sponza = world.spawn((Transform {
    //     position: Vec3::ZERO,
    //     rotation: Quat::IDENTITY,
    // },));
    // loader.load_and_attach_model_sync(sponza, "models/sponza.obj");
}

fn main() {
    // control::move_player(
    //     &mut self.world,
    //     self.player,
    //     &self.input_state,
    //     &self.timings,
    // );

    videoland::run();
}
