use std::borrow::Cow;

use crate::asset::{Mesh, Model, Shader};
use crate::scene::Scene;
use ahash::AHashMap;
use glam::{Mat4, Vec2};
use pollster::FutureExt;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tracing::info;
use uuid::Uuid;
use wgpu;
use wgpu::util::DeviceExt;
use winit::window::Window;

pub mod fg;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

impl From<Extent2D> for Vec2 {
    fn from(value: Extent2D) -> Self {
        Self {
            x: value.width as f32,
            y: value.height as f32,
        }
    }
}

impl Extent2D {
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

#[derive(Default)]
pub struct PreparedUi {
    pub shapes: Vec<egui::ClippedPrimitive>,
    pub textures_delta: egui::TexturesDelta,
}

#[derive(Clone)]
pub struct MaterialDesc<'a> {
    pub vertex_shader: &'a Shader,
    pub fragment_shader: &'a Shader,
}

struct GpuMaterial {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
}

struct GpuMesh {
    vertex_count: u32,
    buffer: wgpu::Buffer,
}

#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct PushConstants {
    camera_transform: Mat4,
    transform: Mat4,
}

pub struct Renderer {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_format: wgpu::TextureFormat,

    materials: AHashMap<Uuid, GpuMaterial>,
    meshes: AHashMap<Uuid, GpuMesh>,

    egui_renderer: egui_wgpu::Renderer,
    egui_render_targets: AHashMap<egui::TextureId, (wgpu::TextureView, Extent2D)>,
}

impl Renderer {
    pub fn new(window: &Window, egui_vs: Shader, egui_fs: Shader) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::empty(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let raw_window_handle = window.window_handle().unwrap().as_raw();
        let raw_display_handle = window.display_handle().unwrap().as_raw();

        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle,
                raw_window_handle,
            })
        }
        .unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .block_on()
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .block_on()
            .unwrap();

        let surface_format = surface.get_capabilities(&adapter).formats[0];

        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1);

        Self {
            instance,
            device,
            surface,
            queue,
            surface_format,

            materials: AHashMap::new(),
            meshes: AHashMap::new(),
            egui_renderer,
            egui_render_targets: AHashMap::new(),
        }
    }

    pub fn upload_material(&mut self, desc: &MaterialDesc) -> Uuid {
        let (vs, fs) = unsafe {
            let vs = self
                .device
                .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                    label: None,
                    source: Cow::Borrowed(bytemuck::cast_slice(desc.vertex_shader.data())),
                });
            let fs = self
                .device
                .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                    label: None,
                    source: Cow::Borrowed(bytemuck::cast_slice(desc.fragment_shader.data())),
                });

            (vs, fs)
        };

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[],
                    label: None,
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: "vs_main",
                    buffers: &[crate::asset::Vertex::layout()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: "fs_main",
                    targets: &[Some(self.surface_format.into())],
                }),
                label: None,
                layout: Some(&pipeline_layout),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        let id = Uuid::new_v4();

        self.materials.insert(
            id,
            GpuMaterial {
                bind_group_layout,
                pipeline_layout,
                pipeline,
            },
        );

        id
    }

    fn upload_model(&mut self, model: &Model) {
        for mesh in model.meshes() {
            self.upload_mesh(mesh);
        }
    }

    fn upload_mesh(&mut self, mesh: &Mesh) {
        let renderable_mesh_id = Uuid::new_v4();
        info!(%renderable_mesh_id);

        let mesh_data_size = std::mem::size_of_val(mesh.data()) as u64;

        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(mesh.data()),
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.meshes.insert(
            renderable_mesh_id,
            GpuMesh {
                vertex_count: mesh.vertex_count(),
                buffer,
            },
        );
    }

    pub fn resize(&mut self, size: Extent2D) {
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.surface_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::AutoVsync,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: Vec::new(),
            },
        );
    }

    pub fn create_egui_render_target(&mut self, size: Extent2D) -> egui::TextureId {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&Default::default());
        let texture_id = self.egui_renderer.register_native_texture(
            &self.device,
            &texture_view,
            wgpu::FilterMode::Nearest,
        );

        self.egui_render_targets
            .insert(texture_id, (texture_view, size));

        texture_id
    }

    fn resize_egui_render_target(&mut self, texture_id: egui::TextureId, size: Extent2D) {
        self.egui_render_targets
            .entry(texture_id)
            .and_modify(|(view, rt_size)| {
                if size != *rt_size {
                    return;
                }

                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });

                let texture_view = texture.create_view(&Default::default());

                self.egui_renderer.update_egui_texture_from_wgpu_texture(
                    &self.device,
                    &texture_view,
                    wgpu::FilterMode::Nearest,
                    texture_id,
                );

                *view = texture_view;
            });
    }

    pub fn render_scene_to_egui_texture(
        &mut self,
        texture_id: egui::TextureId,
        size: Extent2D,
        scene: &Scene,
    ) {
        self.resize_egui_render_target(texture_id, size);

        let (view, _) = self.egui_render_targets.get(&texture_id).unwrap();

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: ((scene.bg_color >> 24) & 0xFF) as f64 / 255.0,
                            g: ((scene.bg_color >> 16) & 0xFF) as f64 / 255.0,
                            b: ((scene.bg_color >> 8) & 0xFF) as f64 / 255.0,
                            a: ((scene.bg_color) & 0xFF) as f64 / 255.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        self.queue.submit([encoder.finish()]);
    }

    pub fn render(
        &mut self,
        camera_transform: Mat4,
        scene: &Scene,
        prepared_ui: &PreparedUi,
        viewport_extent: Extent2D,
    ) {
        let frame = self.surface.get_current_texture().unwrap();
        let frame_view = frame.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        for (id, delta) in &prepared_ui.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &prepared_ui.shapes,
            &egui_wgpu::ScreenDescriptor {
                size_in_pixels: [viewport_extent.width, viewport_extent.height],
                pixels_per_point: 1.0,
            },
        );

        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: (scene.bg_color >> 24 & 0xFF) as f64 / 255.0,
                            g: (scene.bg_color >> 16 & 0xFF) as f64 / 255.0,
                            b: (scene.bg_color >> 8 & 0xFF) as f64 / 255.0,
                            a: (scene.bg_color & 0xFF) as f64 / 255.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer.render(
                &mut rp,
                &prepared_ui.shapes,
                &egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [viewport_extent.width, viewport_extent.height],
                    pixels_per_point: 1.0,
                },
            );
        }

        for id in &prepared_ui.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit([encoder.finish()]);

        frame.present();
    }
}
