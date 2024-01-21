use ahash::AHashMap;
use glam::Mat4;
use uuid::Uuid;
use videoland_ap::model::{Mesh, Model};
use videoland_ap::shader::{Shader, ShaderStage};
use videoland_ap::Vfs;
use videoland_rhi as rhi;
use winit::window::Window;

use crate::egui::{EguiRenderer, PreparedUi};

pub mod egui;

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

    material: Uuid,

    depth_desc: rhi::TextureDesc,
    depth: rhi::Texture,
    depth_view: rhi::TextureView,
    depth_layout: rhi::TextureLayout,

    egui_renderer: EguiRenderer,
}

impl Renderer {
    pub fn new(window: &Window, vfs: &Vfs) -> Self {
        let device = rhi::Context::new(window).unwrap();

        let size = window.inner_size();
        let depth_desc = rhi::TextureDesc {
            extent: rhi::Extent3D {
                width: size.width,
                height: size.height,
                depth: 1,
            },
        };

        let depth = device.create_texture(&depth_desc);
        let depth_view = device.create_texture_view(
            &depth,
            &rhi::TextureViewDesc {
                extent: depth_desc.extent,
            },
        );

        let egui_vertex = vfs.load_shader_sync("shaders/egui.hlsl", ShaderStage::Vertex);
        let egui_fragment = vfs.load_shader_sync("shaders/egui.hlsl", ShaderStage::Fragment);
        let egui_renderer = EguiRenderer::new(device.clone(), egui_vertex, egui_fragment);

        let mesh = vfs.load_binary_sync("models/sponza.obj");
        let mesh = videoland_ap::model::Model::from_obj(&mesh);

        let mut renderer = Self {
            context: device,

            materials: AHashMap::new(),
            meshes: AHashMap::new(),
            material: Uuid::nil(),

            depth_desc,
            depth,
            depth_view,
            depth_layout: rhi::TextureLayout::Undefined,

            egui_renderer,
        };

        renderer.material = renderer.upload_material(&MaterialDesc {
            vertex_shader: &vfs.load_shader_sync("shaders/object.hlsl", ShaderStage::Vertex),
            fragment_shader: &vfs.load_shader_sync("shaders/object.hlsl", ShaderStage::Fragment),
        });
        renderer.upload_model(&mesh);

        renderer
    }

    pub fn upload_material(&mut self, desc: &MaterialDesc) -> Uuid {
        let vs = self
            .context
            .create_shader_module(desc.vertex_shader.spirv());
        let fs = self
            .context
            .create_shader_module(desc.fragment_shader.spirv());

        let pipeline = self.context.create_pipeline(&rhi::PipelineDesc {
            vertex: &vs,
            fragment: &fs,
            vertex_layout: rhi::VertexBufferLayout {
                stride: 8 * 4,
                attributes: &[
                    // position
                    rhi::VertexAttribute {
                        binding: 0,
                        format: rhi::VertexFormat::Float32x3,
                        offset: 0,
                        location: 0,
                    },
                    // normal
                    rhi::VertexAttribute {
                        binding: 0,
                        format: rhi::VertexFormat::Float32x3,
                        offset: 3 * 4,
                        location: 1,
                    },
                    // texcoord
                    rhi::VertexAttribute {
                        binding: 0,
                        format: rhi::VertexFormat::Float32x2,
                        offset: 6 * 4,
                        location: 2,
                    },
                ],
            },
        });

        let id = Uuid::new_v4();

        self.materials.insert(id, GpuMaterial { pipeline });

        id
    }

    fn upload_model(&mut self, model: &Model) {
        for mesh in model.meshes() {
            self.upload_mesh(mesh);
        }
    }

    fn upload_mesh(&mut self, mesh: &Mesh) {
        let renderable_mesh_id = Uuid::new_v4();

        let mesh_data_size = std::mem::size_of_val(mesh.data()) as u64;

        let mut staging = self.context.create_buffer(rhi::BufferAllocation {
            usage: rhi::BufferUsage::VERTEX,
            location: rhi::BufferLocation::Cpu,
            size: mesh_data_size,
        });

        staging.write_data(0, bytemuck::cast_slice(mesh.data()));

        // let gpu_buffer = self.device.create_buffer(ra::BufferAllocation {
        //     usage: ra::BufferUsage::VERTEX,
        //     location: ra::BufferLocation::Gpu,
        //     size: mesh_data_size,
        // });

        self.meshes.insert(
            renderable_mesh_id,
            GpuMesh {
                vertex_count: mesh.vertex_count(),
                buffer: staging,
            },
        );
    }

    pub fn resize(&mut self, size: Extent2D) {
        self.context.resize_swapchain(size.into());

        self.depth_desc.extent.width = size.width;
        self.depth_desc.extent.height = size.height;

        let mut depth = self.context.create_texture(&self.depth_desc);
        let mut depth_view = self.context.create_texture_view(
            &depth,
            &rhi::TextureViewDesc {
                extent: self.depth_desc.extent,
            },
        );

        std::mem::swap(&mut self.depth, &mut depth);
        std::mem::swap(&mut self.depth_view, &mut depth_view);

        self.depth_layout = rhi::TextureLayout::Undefined;
    }

    pub fn render(&mut self, camera_transform: Mat4, viewport_extent: Extent2D, ui: &PreparedUi) {
        let frame = self.context.acquire_next_frame();
        let frame_image = frame.image_view();

        let command_buffer = self.context.begin_command_buffer(&frame);

        command_buffer.texture_barrier(
            &self.depth,
            self.depth_layout,
            rhi::TextureLayout::DepthStencil,
        );
        self.depth_layout = rhi::TextureLayout::DepthStencil;

        command_buffer.begin_rendering(rhi::RenderPassDesc {
            color_attachment: frame_image,
            depth_attachment: &self.depth_view,
            render_area: viewport_extent.into(),
        });

        command_buffer.set_viewport(viewport_extent.into());

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

        self.egui_renderer.render(&command_buffer, ui);

        command_buffer.end_rendering();

        self.context.submit_frame(command_buffer, &frame);
    }
}
