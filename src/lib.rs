#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::new_without_default)]

pub mod asset;
pub mod core;
pub mod editor;
pub mod input;
pub mod loader;
pub mod render;
pub mod scene;
pub mod settings;
pub mod sys;
pub mod timing;
pub mod ui;

pub use glam as math;
pub use winit;

use std::sync::Arc;

use rayon::ThreadPoolBuilder;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use crate::asset::{ShaderStage, Vfs};
use crate::core::{EventQueue, Registry, Schedule, Stage};
use crate::input::InputState;
use crate::loader::{Loader, ShaderBytecode, ShaderCompiler};
use crate::render::PreparedUi;
use crate::render::{Extent2D, Renderer};
use crate::scene::SceneGraph;
use crate::settings::Settings;
use crate::timing::Timings;
use crate::ui::Ui;

#[derive(Default)]
pub struct EngineState {
    pub quit: bool,
}

struct AppState {
    reg: Registry,
    schedule: Box<dyn Fn(&Registry) -> Schedule>,
}

impl AppState {
    fn new(schedule: Box<dyn Fn(&Registry) -> Schedule>, window: Window) -> Self {
        let settings = Settings::load_global();

        let thread_pool = Arc::new(ThreadPoolBuilder::new().num_threads(4).build().unwrap());

        let vfs = Arc::new(Vfs::new());

        vfs.add_root("videoland".to_owned(), "../videoland/data");

        let shader_compiler = ShaderCompiler::new();

        let egui_vs = shader_compiler
            .compile_hlsl(
                "videoland/data/shaders/egui.hlsl",
                ShaderStage::Vertex,
                ShaderBytecode::SpirV,
            )
            .unwrap();
        let egui_fs = shader_compiler
            .compile_hlsl(
                "videoland/data/shaders/egui.hlsl",
                ShaderStage::Fragment,
                ShaderBytecode::SpirV,
            )
            .unwrap();

        let renderer = Renderer::new(&window, egui_vs, egui_fs);
        let mut ui = Ui::new(&window);

        ui.begin_frame(&window);

        let mut reg = Registry::new();

        reg.insert(EventQueue::<KeyEvent>::new());

        // window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        // window.set_cursor_visible(false);

        reg.insert(InputState::new());
        reg.insert(Timings::new());
        reg.insert(ui);
        reg.insert(window);
        reg.insert(Loader::new(vfs, thread_pool));
        reg.insert(settings);
        reg.insert(renderer);
        reg.insert(PreparedUi::default());
        reg.insert(EngineState::default());
        reg.insert(SceneGraph::new());

        schedule(&reg).execute(Stage::Init, &mut reg);

        Self { reg, schedule }
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> EventLoopIterationDecision {
        {
            let window = self.reg.res::<Window>();
            self.reg.res_mut::<Ui>().on_event(&window, &event);
        }

        self.reg.res_mut::<InputState>().submit_window_input(&event);

        match event {
            WindowEvent::CloseRequested => return EventLoopIterationDecision::Break,
            WindowEvent::KeyboardInput { event, .. } => {
                self.reg.res_mut::<EventQueue<KeyEvent>>().emit(event);
            }
            WindowEvent::Resized(size) => self.reg.res_mut::<Renderer>().resize(Extent2D {
                width: size.width,
                height: size.height,
            }),
            _ => {}
        }

        EventLoopIterationDecision::Continue
    }

    fn handle_device_event(&mut self, event: DeviceEvent) -> EventLoopIterationDecision {
        self.reg.res_mut::<InputState>().submit_device_input(&event);

        EventLoopIterationDecision::Continue
    }

    fn update(&mut self) -> EventLoopIterationDecision {
        {
            let mut timings = self.reg.res_mut::<Timings>();
            timings.advance_frame();
            let dt = timings.dtime_s() as f32;
        }

        (self.schedule)(&self.reg).execute(Stage::EachStep, &mut self.reg);

        self.reg.res_mut::<InputState>().reset_mouse_movement();

        if self.reg.res::<EngineState>().quit {
            return EventLoopIterationDecision::Break;
        }

        self.reg.next_step();

        EventLoopIterationDecision::Continue
    }
}

pub enum EventLoopIterationDecision {
    Continue,
    Break,
}

pub struct AppInfo {
    pub internal_name: String,
    pub title: String,
}

pub struct App {
    schedule: Box<dyn Fn(&Registry) -> Schedule>,
    info: AppInfo,
}

impl App {
    pub fn new(schedule: impl Fn(&Registry) -> Schedule + 'static, info: AppInfo) -> Self {
        Self {
            schedule: Box::new(schedule),
            info,
        }
    }

    pub fn run(self) {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(1600, 900))
            .with_title(&self.info.title)
            .build(&event_loop)
            .unwrap();

        let mut state = AppState::new(self.schedule, window);

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop
            .run(move |event, elwt| {
                let cf = match event {
                    Event::WindowEvent { event, .. } => state.handle_window_event(event),
                    Event::DeviceEvent { event, .. } => state.handle_device_event(event),
                    Event::AboutToWait => state.update(),
                    Event::LoopExiting => {
                        state.reg.res::<Settings>().save();
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
}
