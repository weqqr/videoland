#![allow(dead_code)]

use anyhow::Result;
use tracing_subscriber::fmt::format::FmtSpan;
use videoland::Renderer;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

pub struct App {
    event_loop: EventLoop<()>,
    renderer: Renderer,
}

impl App {
    pub fn new() -> Result<Self> {
        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("hello triangle")
            .build(&event_loop)?;

        let renderer = Renderer::new(window)?;

        Ok(Self {
            event_loop,
            renderer,
        })
    }

    fn run(mut self) -> ! {
        self.event_loop.run(move |event, _, cf| {
            *cf = ControlFlow::Poll;

            match event {
                Event::MainEventsCleared => {
                    self.renderer.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    self.renderer.render();
                }
                _ => {}
            }

            let Event::WindowEvent { event, .. } = event else {
                return;
            };

            match event {
                WindowEvent::CloseRequested => {
                    *cf = ControlFlow::Exit;
                }
                WindowEvent::Resized(size) => self.renderer.resize(size),
                _ => {}
            }
        })
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_span_events(FmtSpan::ENTER)
        .init();

    App::new()?.run();
}
