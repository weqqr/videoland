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
pub mod time;
pub mod ui;

pub use glam as math;
pub use tracing as log;
pub use uuid;
pub use winit;
use winit::application::ApplicationHandler;

use std::sync::Arc;

use rayon::ThreadPoolBuilder;
use winit::event::{DeviceEvent, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::Window;

use crate::asset::{ShaderStage, Vfs};
use crate::core::{Registry, Schedule, Stage};
use crate::input::InputState;
use crate::loader::{Loader, ShaderBytecode, ShaderCompiler};
use crate::render::PreparedUi;
use crate::render::{Extent2D, Renderer};
use crate::scene::SceneGraph;
use crate::settings::Settings;
use crate::time::Time;
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
    fn new(window: Window) -> Self {
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

        reg.register_event::<KeyEvent>();

        // window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        window.set_cursor_visible(false);

        reg.insert(InputState::new());
        reg.insert(Time::new());
        reg.insert(ui);
        reg.insert(window);
        reg.insert(Loader::new(vfs, thread_pool));
        reg.insert(settings);
        reg.insert(renderer);
        reg.insert(PreparedUi::default());
        reg.insert(EngineState::default());
        reg.insert(SceneGraph::new());

        // schedule(&reg).execute(Stage::Init, &mut reg);

        Self {
            reg,
            schedule: Box::new(|_| Schedule::new()),
        }
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
                self.reg.event_queue_mut::<KeyEvent>().emit(event);
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
    state: Option<AppState>,
}

impl App {
    pub fn new(schedule: impl Fn(&Registry) -> Schedule + 'static, info: AppInfo) -> Self {
        Self {
            schedule: Box::new(schedule),
            info,
            state: None,
        }
    }

    pub fn run(mut self) {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        let event_loop = EventLoop::new().unwrap();

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop.run_app(&mut self).unwrap();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title(&self.info.title))
            .unwrap();
        self.state = Some(AppState::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let it = self.state.as_mut().map(|s| s.handle_window_event(event));
        if let Some(EventLoopIterationDecision::Break) = it {
            event_loop.exit();
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        let it = self.state.as_mut().map(|s| s.handle_device_event(event));
        if let Some(EventLoopIterationDecision::Break) = it {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let it = self.state.as_mut().map(|s| s.update());
        if let Some(EventLoopIterationDecision::Break) = it {
            event_loop.exit();
        }
    }
}
