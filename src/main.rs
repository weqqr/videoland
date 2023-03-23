#![allow(dead_code)]

mod gapi;
mod res;

use anyhow::{Context, Result};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use tracing::instrument;
use tracing_subscriber::fmt::format::FmtSpan;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use crate::gapi::*;

// FIXME: this value should be determined automatically by gapi
const FRAMES: u32 = 2;

pub struct Renderer {
    window: Window,

    encoder: CommandEncoder,
    surface: Surface,
    device: Device,
    instance: Instance,
}

impl Renderer {
    pub fn new(window: Window) -> Result<Self> {
        let instance = Instance::new(window.raw_display_handle()).context("create instance")?;
        let device = instance.create_device()?;
        let mut surface = instance.create_surface(
            &device,
            window.raw_display_handle(),
            window.raw_window_handle(),
        )?;

        let size = window.inner_size();
        surface.configure(SurfaceConfiguration {
            frames: FRAMES,
            width: size.width,
            height: size.height,
        });

        let encoder = device.create_command_encoder(FRAMES)?;

        Ok(Self {
            window,
            instance,
            device,
            surface,
            encoder,
        })
    }

    #[instrument(skip(self))]
    pub fn render(&mut self) {
        let frame = self.surface.acquire_next_image();
        self.encoder.begin(frame);

        self.device.finish_frame(&self.encoder, frame);
        frame.present();
        self.device.wait_for_sync();
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_for_sync();
        self.device.destroy_command_encoder(&mut self.encoder);
    }
}

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
                    self.renderer.window.request_redraw();
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
                WindowEvent::Resized(size) => {
                    self.renderer.surface.configure(SurfaceConfiguration {
                        frames: FRAMES,
                        width: size.width,
                        height: size.height,
                    })
                }
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
