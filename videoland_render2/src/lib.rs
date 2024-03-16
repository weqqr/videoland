use ahash::AHashMap;
use glam::{Mat4, Vec2};
use tracing::info;
use uuid::Uuid;
use videoland_ap::model::{Mesh, Model};
use videoland_ap::shader::Shader;
use videoland_rhi as rhi;
use winit::window::Window;

use crate::egui::{EguiRenderer, PreparedUi};
use crate::fg::ResourceContainer;

pub mod egui;
pub mod fg;

#[derive(Clone, Copy)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

impl From<Extent2D> for rhi::Extent2D {
    fn from(extent: Extent2D) -> rhi::Extent2D {
        rhi::Extent2D {
            width: extent.width,
            height: extent.height,
        }
    }
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

#[derive(Clone)]
pub struct MaterialDesc<'a> {
    pub vertex_shader: &'a Shader,
    pub fragment_shader: &'a Shader,
}

struct GpuMaterial {
    bind_group_layout: rhi::BindGroupLayout,
    pipeline: rhi::Pipeline,
}

struct GpuMesh {
    vertex_count: u32,
    buffer: rhi::Buffer,
}

#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct PushConstants {
    camera_transform: Mat4,
    transform: Mat4,
}

pub struct Renderer {
    context: rhi::Context,

    materials: AHashMap<Uuid, GpuMaterial>,
    meshes: AHashMap<Uuid, GpuMesh>,

    rc: ResourceContainer,

    egui_renderer: EguiRenderer,
}

impl Renderer {
    pub fn new(window: &Window, egui_vs: Shader, egui_fs: Shader) -> Self {
        let size = window.inner_size();

        let context = rhi::Context::new(
            window,
            rhi::Extent2D {
                width: size.width,
                height: size.height,
            },
        )
        .unwrap();

        let egui_renderer = EguiRenderer::new(&context, egui_vs, egui_fs);

        Self {
            context,

            materials: AHashMap::new(),
            meshes: AHashMap::new(),

            rc: ResourceContainer::new(),

            egui_renderer,
        }
    }

    pub fn upload_material(&mut self, desc: &MaterialDesc) -> Uuid {
        let vs = self
            .context
            .create_shader_module(desc.vertex_shader.data().to_owned());
        let fs = self
            .context
            .create_shader_module(desc.fragment_shader.data().to_owned());

        let bind_group_layout = self
            .context
            .create_bind_group_layout(&rhi::BindGroupLayoutDesc { entries: &[] });

        let pipeline = self.context.create_pipeline(&rhi::PipelineDesc {
            vertex: &vs,
            fragment: &fs,
            bind_group_layout: &bind_group_layout,
            vertex_layout: rhi::VertexBufferLayout {
                stride: 8 * 4,
                attributes: &[
                    // position
                    rhi::VertexAttribute {
                        semantic: "POSITION",
                        binding: 0,
                        format: rhi::VertexFormat::Float32x3,
                        offset: 0,
                        location: 0,
                    },
                    // normal
                    rhi::VertexAttribute {
                        semantic: "NORMAL",
                        binding: 0,
                        format: rhi::VertexFormat::Float32x3,
                        offset: 3 * 4,
                        location: 1,
                    },
                    // texcoord
                    rhi::VertexAttribute {
                        semantic: "TEXCOORD",
                        binding: 0,
                        format: rhi::VertexFormat::Float32x2,
                        offset: 6 * 4,
                        location: 2,
                    },
                ],
            },
        });

        let id = Uuid::new_v4();

        self.materials.insert(
            id,
            GpuMaterial {
                bind_group_layout,
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

        let mut staging = self.context.create_buffer(rhi::BufferAllocation {
            usage: rhi::BufferUsage::VERTEX | rhi::BufferUsage::TRANSFER_SRC,
            location: rhi::BufferLocation::Cpu,
            size: mesh_data_size,
        });

        // staging.write_data(0, bytemuck::cast_slice(mesh.data()));

        let gpu_buffer = self.context.create_buffer(rhi::BufferAllocation {
            usage: rhi::BufferUsage::VERTEX | rhi::BufferUsage::TRANSFER_DST,
            location: rhi::BufferLocation::Gpu,
            size: mesh_data_size,
        });

        // self.context.immediate_submit(|cmd| {
        //     cmd.copy_buffer_to_buffer(&staging, &gpu_buffer, mesh_data_size);
        // });

        self.meshes.insert(
            renderable_mesh_id,
            GpuMesh {
                vertex_count: mesh.vertex_count(),
                buffer: gpu_buffer,
            },
        );
    }

    pub fn resize(&mut self, size: Extent2D) {
        self.context.resize_swapchain(size.into());
    }

    pub fn render(
        &mut self,
        camera_transform: Mat4,
        prepared_ui: &PreparedUi,
        viewport_extent: Extent2D,
    ) {
        let frame = self.context.acquire_next_frame();

        let command_buffer = self.context.begin_command_buffer();

        command_buffer.set_scissor(rhi::Scissor {
            offset: rhi::Offset2D::ZERO,
            extent: viewport_extent.into(),
        });

        command_buffer.set_viewport(rhi::Viewport {
            x: 0.0,
            y: 0.0,
            width: viewport_extent.width as f32,
            height: viewport_extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        });

        command_buffer.texture_barrier(
            rhi::TextureLayout::Present,
            rhi::TextureLayout::Color,
            frame.texture(),
        );

        command_buffer.set_render_target(frame.texture());
        command_buffer.clear_texture(frame.texture(), [0.9, 0.6, 0.3, 1.0]);

        self.egui_renderer.render(
            &self.context,
            &command_buffer,
            prepared_ui,
            viewport_extent.into(),
        );

        command_buffer.texture_barrier(
            rhi::TextureLayout::Color,
            rhi::TextureLayout::Present,
            frame.texture(),
        );

        /*
                let material = self.materials.get(&self.material).unwrap();

                command_buffer.bind_pipeline(&material.pipeline);

                for (_, gpu_mesh) in self.meshes.iter() {
                    let pc = PushConstants {
                        camera_transform,
                        transform: Mat4::IDENTITY, // transform.matrix(),
                    };

                    command_buffer.set_push_constants(&material.pipeline, 0, bytemuck::bytes_of(&pc));

                    command_buffer.bind_vertex_buffer(&gpu_mesh.buffer);
                    command_buffer.draw(gpu_mesh.vertex_count, 1, 0, 0);
                }

                self.egui_renderer
                    .render(&command_buffer, ui, viewport_extent.into());
        */

        self.context.submit_command_buffer(command_buffer);

        self.context.submit_frame();
    }
}
