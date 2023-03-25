#![allow(dead_code)]

mod gapi;

use crate::gapi::*;
use anyhow::{Context, Result};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use tracing::instrument;
use winit::dpi::PhysicalSize;
use winit::window::Window;

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

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface.configure(SurfaceConfiguration {
            frames: FRAMES,
            width: size.width,
            height: size.height,
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

impl Renderer {
    pub fn window(&self) -> &Window {
        &self.window
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_for_sync();
        self.device.destroy_command_encoder(&mut self.encoder);
    }
}
