#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::single_match)]
#![allow(clippy::new_without_default)]

mod automata;
mod camera;
mod control;
mod domain;
mod editor;
mod geometry;
mod input;
mod loader;
mod render2;
mod settings;
mod shader_compiler;
mod timing;
mod ui;

// User-facing project name
const PROJECT_NAME: &str = "Walkhack";

use std::path::PathBuf;
use std::sync::Arc;

use glam::{Quat, Vec3};
use hecs::{Entity, World};
use indexmap::IndexMap;
use rayon::{ThreadPool, ThreadPoolBuilder};
use uuid::Uuid;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{CursorGrabMode, Window, WindowBuilder};

use crate::camera::Camera;
use crate::domain::{Player, RigidBody, Transform};
use crate::editor::{Editor, EditorData};
use crate::input::InputState;
use crate::loader::Loader;
use crate::render2::{Extent2D, MaterialDesc, Renderer};
use crate::settings::Settings;
use crate::shader_compiler::ShaderStage;
use crate::timing::Timings;
use crate::ui::{RenderedUi, Ui};

struct AppState {
    settings: Settings,
    loader: Loader,
    window: Window,
    renderer: Renderer,
    material: Uuid,
    ui: Ui,
    input_state: InputState,
    timings: Timings,
    editor: Editor,
    world: World,
    thread_pool: Arc<ThreadPool>,
    player: Entity,
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

impl AppState {
    fn new(window: Window) -> Self {
        let settings = Settings::load_global();

        let thread_pool = Arc::new(ThreadPoolBuilder::new().num_threads(4).build().unwrap());

        let loader = Loader::new(PathBuf::from("data"), Arc::clone(&thread_pool));
        let mut renderer = Renderer::new(&window);
        let mut ui = Ui::new(&window);

        ui.begin_frame(&window);

        let mut world = World::new();

        let vertex_shader = &loader.load_shader("shaders/object.hlsl", ShaderStage::Vertex);
        let fragment_shader = &loader.load_shader("shaders/object.hlsl", ShaderStage::Fragment);

        let material = Uuid::new_v4();
        renderer.upload_material(
            material,
            &MaterialDesc {
                vertex_shader,
                fragment_shader,
            },
        );

        let input_state = InputState::new();
        let timings = Timings::new();
        let editor = Editor::new();

        window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        window.set_cursor_visible(false);

        let player = add_stuff_to_world(&mut world, &loader);

        Self {
            settings,
            loader,
            window,
            renderer,
            material,
            ui,
            input_state,
            timings,
            editor,
            world,
            thread_pool,
            player,
        }
    }

    fn handle_key(&mut self, input: KeyEvent) -> EventLoopIterationDecision {
        if let Key::Named(NamedKey::Escape) = input.logical_key {
            return EventLoopIterationDecision::Break;
        }

        EventLoopIterationDecision::Continue
    }

    fn handle_mouse_move(&mut self, delta_x: f64, delta_y: f64) {
        const SENSITIVITY: f64 = 0.1;

        let delta_pitch = (delta_y * SENSITIVITY) as f32;
        let delta_yaw = (delta_x * SENSITIVITY) as f32;

        let (_, camera) = self
            .world
            .query_one_mut::<(&Player, &mut Camera)>(self.player)
            .unwrap();

        camera.rotate(delta_pitch, delta_yaw);
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> EventLoopIterationDecision {
        self.ui.on_event(&event);

        self.input_state.submit_window_input(&event);

        match event {
            WindowEvent::CloseRequested => return EventLoopIterationDecision::Break,
            WindowEvent::KeyboardInput { event, .. } => return self.handle_key(event),
            WindowEvent::Resized(size) => self.renderer.resize(Extent2D {
                width: size.width,
                height: size.height,
            }),
            _ => {}
        }

        EventLoopIterationDecision::Continue
    }

    fn handle_device_event(&mut self, event: DeviceEvent) -> EventLoopIterationDecision {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.handle_mouse_move(delta.0, delta.1);
            }
            _ => {}
        }

        EventLoopIterationDecision::Continue
    }

    pub(crate) fn render(&mut self, _rendered_ui: RenderedUi) {
        let window_size = self.window.inner_size();

        let extent = Extent2D {
            width: window_size.width,
            height: window_size.height,
        };

        let mut player_components = self.world.query_one::<&Camera>(self.player).unwrap();
        let camera = player_components.get().unwrap();

        self.renderer
            .render(&self.world, extent, camera, self.material);
    }

    fn prepare_stats(&self) -> IndexMap<String, String> {
        let mut stats = IndexMap::new();
        stats.insert(
            "Frame rate".to_owned(),
            format!("{:>3}fps", self.timings.fps().round()),
        );
        stats.insert(
            "ΔTime".to_owned(),
            format!("{:.2}ms", self.timings.dtime_ms()),
        );

        stats
    }

    fn show_editor(&mut self) {
        let stats = self.prepare_stats();

        self.editor.show(
            &self.ui,
            EditorData {
                renderer: &mut self.renderer,
                stats: &stats,
            },
        );
    }

    fn update(&mut self) -> EventLoopIterationDecision {
        let rendered_ui = self.ui.finish_frame(&self.window);
        self.ui.begin_frame(&self.window);

        self.timings.advance_frame();

        let dt = self.timings.dtime_s() as f32;

        control::move_player(
            &mut self.world,
            self.player,
            &self.input_state,
            &self.timings,
        );
        self.show_editor();
        self.loader.poll(&mut self.world);
        self.renderer.upload_meshes(&mut self.world);
        self.render(rendered_ui);
        self.input_state.reset_mouse_movement();

        EventLoopIterationDecision::Continue
    }
}

pub enum EventLoopIterationDecision {
    Continue,
    Break,
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(1600, 900))
        .build(&event_loop)
        .unwrap();

    let mut state = AppState::new(window);

    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop
        .run(move |event, elwt| {
            let cf = match event {
                Event::WindowEvent { event, .. } => state.handle_window_event(event),
                Event::DeviceEvent { event, .. } => state.handle_device_event(event),
                Event::AboutToWait => state.update(),
                Event::LoopExiting => {
                    state.editor.save_layout();
                    state.settings.save();
                    EventLoopIterationDecision::Break
                }
                _ => EventLoopIterationDecision::Continue,
            };

            if let EventLoopIterationDecision::Break = cf {
                elwt.exit();
            }
        })
        .unwrap();
}
