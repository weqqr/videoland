#![allow(dead_code)]
#![allow(unused_variables)]

pub mod camera;
pub mod domain;
pub mod input;
pub mod settings;
pub mod timing;
pub mod ui;

pub use glam as math;
pub use videoland_ap as ap;
pub use videoland_ecs as ecs;
pub use videoland_render2 as render2;
pub use winit;

use std::sync::Arc;

use indexmap::IndexMap;
use rayon::{ThreadPool, ThreadPoolBuilder};
use uuid::Uuid;
use videoland_ecs::{Registry, Schedule};
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{CursorGrabMode, Window, WindowBuilder};

use crate::ap::shader::ShaderStage;
use crate::ap::Vfs;
use crate::camera::Camera;
use crate::input::InputState;
use crate::render2::{Extent2D, MaterialDesc, Renderer};
use crate::settings::Settings;
use crate::timing::Timings;
use crate::ui::{RenderedUi, Ui};

struct AppState {
    renderer: Renderer,
    material: Uuid,
    reg: Registry,
    schedule: Schedule,
    thread_pool: Arc<ThreadPool>,
}

impl AppState {
    fn new(schedule: Schedule, window: Window) -> Self {
        let settings = Settings::load_global();

        let thread_pool = Arc::new(ThreadPoolBuilder::new().num_threads(4).build().unwrap());

        let vfs = Vfs::new("data");
        let mut renderer = Renderer::new(&window);
        let mut ui = Ui::new(&window);

        ui.begin_frame(&window);

        let mut reg = Registry::new();

        let vertex_shader = &vfs.load_shader_sync("shaders/object.hlsl", ShaderStage::Vertex);
        let fragment_shader = &vfs.load_shader_sync("shaders/object.hlsl", ShaderStage::Fragment);

        let material = Uuid::new_v4();
        renderer.upload_material(
            material,
            &MaterialDesc {
                vertex_shader,
                fragment_shader,
            },
        );

        reg.insert(InputState::new());
        reg.insert(Timings::new());

        window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        window.set_cursor_visible(false);

        reg.insert(ui);
        reg.insert(window);
        reg.insert(vfs);
        reg.insert(settings);

        Self {
            renderer,
            material,
            reg,
            schedule,
            thread_pool,
        }
    }

    fn handle_key(&mut self, input: KeyEvent) -> EventLoopIterationDecision {
        if let Key::Named(NamedKey::Escape) = input.logical_key {
            return EventLoopIterationDecision::Break;
        }

        EventLoopIterationDecision::Continue
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> EventLoopIterationDecision {
        self.reg.res_mut::<Ui>().on_event(&event);

        self.reg.res_mut::<InputState>().submit_window_input(&event);

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
        self.reg.res_mut::<InputState>().submit_device_input(&event);

        EventLoopIterationDecision::Continue
    }

    pub(crate) fn render(&mut self, _rendered_ui: RenderedUi) {
        let window_size = self.reg.res::<Window>().inner_size();

        let extent = Extent2D {
            width: window_size.width,
            height: window_size.height,
        };

        let camera = &Camera::new();

        self.renderer
            .render(&self.reg, extent, /*camera,*/ self.material);
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
        let rendered_ui = {
            let window = self.reg.res::<Window>();
            let mut ui = self.reg.res_mut::<Ui>();

            let rendered_ui = ui.finish_frame(&window);
            ui.begin_frame(&window);

            rendered_ui
        };

        {
            let mut timings = self.reg.res_mut::<Timings>();
            timings.advance_frame();
            let dt = timings.dtime_s() as f32;
        }

        self.schedule.execute(&self.reg);
        // self.loader.poll(&mut self.reg);
        // self.renderer.upload_meshes(&mut self.world);
        self.render(rendered_ui);
        self.reg.res_mut::<InputState>().reset_mouse_movement();

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
