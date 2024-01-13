use ahash::AHashMap;
use glam::Mat4;
use uuid::Uuid;
use videoland_ap::shader::Shader;
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
    fn aspect_ratio(&self) -> f32 {
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
    view_projection: Mat4,
    transform: Mat4,
}

pub struct Renderer {
    device: rhi::Device,

    materials: AHashMap<Uuid, GpuMaterial>,
    meshes: AHashMap<Uuid, GpuMesh>,

    depth_desc: rhi::TextureDesc,
    depth: rhi::Texture,
    depth_view: rhi::TextureView,
    depth_layout: rhi::TextureLayout,

    egui_renderer: EguiRenderer,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.destroy_texture_view(&mut self.depth_view);
        self.device.destroy_texture(&mut self.depth);
    }
}

impl Renderer {
    pub fn new(window: &Window) -> Self {
        let device = rhi::Device::new(window).unwrap();

        let size = window.inner_size();
        let depth_desc = rhi::TextureDesc {
            extent: rhi::Extent3D {
                width: size.width,
                height: size.height,
                depth: 1,
            },
        };

        let depth = device.create_texture(&depth_desc).unwrap();
        let depth_view = device
            .create_texture_view(
                &depth,
                &rhi::TextureViewDesc {
                    extent: depth_desc.extent,
                },
            )
            .unwrap();

        let egui_renderer = EguiRenderer::new(&device, vec![], vec![]);

        Self {
            device,

            materials: AHashMap::new(),
            meshes: AHashMap::new(),

            depth_desc,
            depth,
            depth_view,
            depth_layout: rhi::TextureLayout::Undefined,

            egui_renderer,
        }
    }

    pub fn upload_material(&mut self, id: Uuid, desc: &MaterialDesc) {
        let vs = self
            .device
            .create_shader_module(desc.vertex_shader.spirv())
            .unwrap();
        let fs = self
            .device
            .create_shader_module(desc.fragment_shader.spirv())
            .unwrap();

        let pipeline = self
            .device
            .create_pipeline(&rhi::PipelineDesc {
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
            })
            .unwrap();

        self.materials.insert(id, GpuMaterial { pipeline });
    }

    /*pub fn upload_meshes(&mut self, world: &mut World) {
        let mut entities_with_models = Vec::new();

        for (e, _) in world.query::<&Model>().iter() {
            entities_with_models.push(e);
        }

        for entity in entities_with_models {
            let model = world.remove_one::<Model>(entity).unwrap();
            for mesh in model.meshes() {
                let renderable_mesh = self.upload_mesh(mesh);

                world.spawn((
                    Parent {
                        entity,
                        relative_transform: Transform {
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                    },
                    Transform {
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    renderable_mesh,
                    Name(mesh.name.clone()),
                ));
            }
        }
    }

    fn upload_mesh(&mut self, mesh: &Mesh) -> RenderableMesh {
        let renderable_mesh_id = Uuid::new_v4();

        let mesh_data_size = std::mem::size_of_val(mesh.data()) as u64;

        let staging = self
            .device
            .create_buffer(ra::BufferAllocation {
                usage: ra::BufferUsage::VERTEX,
                location: ra::BufferLocation::Cpu,
                size: mesh_data_size,
            })
            .unwrap();

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

        RenderableMesh(renderable_mesh_id)
    }*/

    pub fn resize(&mut self, size: Extent2D) {
        self.device.resize_swapchain(size).unwrap();

        self.depth_desc.extent.width = size.width;
        self.depth_desc.extent.height = size.height;

        let mut depth = self.device.create_texture(&self.depth_desc).unwrap();
        let mut depth_view = self
            .device
            .create_texture_view(
                &depth,
                &rhi::TextureViewDesc {
                    extent: self.depth_desc.extent,
                },
            )
            .unwrap();

        std::mem::swap(&mut self.depth, &mut depth);
        std::mem::swap(&mut self.depth_view, &mut depth_view);
        self.depth_layout = rhi::TextureLayout::Undefined;

        self.device.destroy_texture_view(&mut depth_view);
        self.device.destroy_texture(&mut depth);
    }

    pub fn render(
        &mut self,
        viewport_extent: Extent2D,
        ui: &PreparedUi,
    ) {
        let frame = self.device.acquire_next_image();
        let frame_image = frame.image_view();

        let command_buffer = self.device.begin_command_buffer(&frame);

        command_buffer.texture_barrier(
            &self.depth,
            self.depth_layout,
            rhi::TextureLayout::DepthStencil,
        );
        self.depth_layout = rhi::TextureLayout::DepthStencil;

        command_buffer.begin_rendering(rhi::RenderPassDesc {
            color_attachment: &frame_image,
            depth_attachment: &self.depth_view,
            render_area: viewport_extent.into(),
        });

        command_buffer.set_viewport(viewport_extent);

        // let material = self.materials.get(&material).unwrap();

        // command_buffer.bind_pipeline(&material.pipeline);

        // for (e, (transform, mesh)) in world.query::<(&Transform, &RenderableMesh)>().iter() {
        //     let pc = PushConstants {
        //         view_projection: camera.view_projection(viewport_extent.aspect_ratio()),
        //         transform: transform.matrix(),
        //     };

        //     let gpu_mesh = self.meshes.get(&mesh.0).unwrap();

        //     command_buffer.set_push_constants(&material.pipeline, 0, bytemuck::bytes_of(&pc));

        //     command_buffer.bind_vertex_buffer(&gpu_mesh.buffer);
        //     command_buffer.draw(gpu_mesh.vertex_count, 1, 0, 0);
        // }

        self.egui_renderer.render(ui);

        command_buffer.end_rendering();

        self.device.submit_frame(command_buffer, &frame).unwrap();
    }
}
