#![allow(dead_code)]
#![allow(unused_variables)]

pub mod camera;
pub mod domain;
pub mod input;
pub mod loader;
pub mod nodes;
pub mod settings;
pub mod sys;
pub mod timing;

pub use glam as math;
pub use videoland_ap as ap;
pub use videoland_ecs as ecs;
pub use videoland_render2 as render2;
pub use videoland_sg as sg;
pub use winit;

use std::sync::Arc;

use indexmap::IndexMap;
use rayon::{ThreadPool, ThreadPoolBuilder};
use videoland_egui::Ui;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Window, WindowBuilder};

use crate::ap::Vfs;
use crate::camera::MainCamera;
use crate::ecs::{EventQueue, Registry, Schedule, Stage};
use crate::input::InputState;
use crate::loader::Loader;
use crate::render2::egui::PreparedUi;
use crate::render2::{Extent2D, Renderer};
use crate::settings::Settings;
use crate::sg::SceneGraph;
use crate::timing::Timings;

#[derive(Default)]
pub struct EngineState {
    pub quit: bool,
}

struct AppState {
    reg: Registry,
    schedule: Schedule,
}

impl AppState {
    fn new(mut schedule: Schedule, window: Window) -> Self {
        let settings = Settings::load_global();

        let thread_pool = Arc::new(ThreadPoolBuilder::new().num_threads(4).build().unwrap());

        let vfs = Arc::new(Vfs::new());

        vfs.add_root("videoland".to_owned(), "../videoland/data");

        let renderer = Renderer::new(&window, &vfs);
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
        reg.insert(MainCamera::new());
        reg.insert(SceneGraph::new());
        schedule.execute(Stage::Init, &mut reg);

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

    fn prepare_stats(&self) -> IndexMap<String, String> {
        let timings = self.reg.res::<Timings>();

        let mut stats = IndexMap::new();
        stats.insert(
            "Frame rate".to_owned(),
            format!("{:>3}fps", timings.fps().round()),
        );
        stats.insert("ΔTime".to_owned(), format!("{:.2}ms", timings.dtime_ms()));

        stats
    }

    fn update(&mut self) -> EventLoopIterationDecision {
        {
            let mut timings = self.reg.res_mut::<Timings>();
            timings.advance_frame();
            let dt = timings.dtime_s() as f32;
        }

        self.schedule.execute(Stage::EachStep, &mut self.reg);

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
    schedule: Schedule,
    info: AppInfo,
}

impl App {
    pub fn new(schedule: Schedule, info: AppInfo) -> Self {
        Self { schedule, info }
    }

    pub fn run(self) {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();

        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(1600, 900))
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
