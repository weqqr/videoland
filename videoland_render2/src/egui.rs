use egui::epaint::Primitive;
use egui::{ClippedPrimitive, TexturesDelta};
use glam::Vec2;
use videoland_ap::shader::Shader;
use videoland_rhi as rhi;

#[derive(Default)]
pub struct PreparedUi {
    pub shapes: Vec<ClippedPrimitive>,
    pub textures_delta: TexturesDelta,
}

pub struct EguiRenderer {
    vertex_buffer: rhi::Buffer,
    index_buffer: rhi::Buffer,
    uniform_buffer: rhi::Buffer,

    bind_group: rhi::BindGroup,
    bind_group_layout: rhi::BindGroupLayout,
    pipeline: rhi::Pipeline,
}

#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Uniforms {
    viewport_size: Vec2,
    _padding1: [f32; 32],
    _padding2: [f32; 30],
}

impl EguiRenderer {
    pub fn new(context: &rhi::Context, vertex_shader: Shader, fragment_shader: Shader) -> Self {
        let initial_buffer_size = 1024 * 1024 * 4;

        let vertex_buffer = context.create_buffer(rhi::BufferAllocation {
            usage: rhi::BufferUsage::VERTEX,
            location: rhi::BufferLocation::Cpu,
            size: initial_buffer_size,
        });

        let index_buffer = context.create_buffer(rhi::BufferAllocation {
            usage: rhi::BufferUsage::INDEX,
            location: rhi::BufferLocation::Cpu,
            size: initial_buffer_size,
        });

        let vertex_shader = context.create_shader_module(vertex_shader.data().to_owned());
        let fragment_shader = context.create_shader_module(fragment_shader.data().to_owned());

        let bind_group_layout = context.create_bind_group_layout(&rhi::BindGroupLayoutDesc {
            entries: &[rhi::BindGroupLayoutEntry {
                binding: 0,
                visibility: rhi::ShaderStages::all(),
                ty: rhi::BindingType::Uniform,
            }],
        });

        let pipeline = context.create_pipeline(&rhi::PipelineDesc {
            vertex: &vertex_shader,
            fragment: &fragment_shader,
            bind_group_layout: &bind_group_layout,
            vertex_layout: rhi::VertexBufferLayout {
                attributes: &[
                    rhi::VertexAttribute {
                        semantic: "POSITION",
                        binding: 0,
                        location: 0,
                        offset: 0,
                        format: rhi::VertexFormat::Float32x2,
                    },
                    rhi::VertexAttribute {
                        semantic: "TEXCOORD",
                        binding: 0,
                        location: 1,
                        offset: 2 * 4,
                        format: rhi::VertexFormat::Float32x2,
                    },
                    rhi::VertexAttribute {
                        semantic: "COLOR",
                        binding: 0,
                        location: 2,
                        offset: 4 * 4,
                        format: rhi::VertexFormat::Uint32x1,
                    },
                ],
                stride: 5 * 4,
            },
        });

        let uniform_buffer = context.create_buffer(rhi::BufferAllocation {
            usage: rhi::BufferUsage::UNIFORM,
            location: rhi::BufferLocation::Cpu,
            size: std::mem::size_of::<Uniforms>() as u64,
        });

        let bind_group = context.create_bind_group(&rhi::BindGroupDesc {
            layout: &bind_group_layout,
            entries: &[rhi::BindGroupEntry {
                binding: 0,
                resource: rhi::BindingResource::Buffer(&uniform_buffer),
            }],
        });

        Self {
            vertex_buffer,
            index_buffer,
            uniform_buffer,

            bind_group,
            bind_group_layout,
            pipeline,
        }
    }

    pub fn render(&mut self, context: &rhi::Context, cbuf: &rhi::CommandBuffer, ui: &PreparedUi, viewport_size: Vec2) {
        let mut vertex_buffer = vec![];
        let mut index_buffer = vec![];

        let mut vertex_count = 0;

        let mut vertex_offsets = vec![];
        let mut indices = vec![];

        for primitive in &ui.shapes {
            match &primitive.primitive {
                Primitive::Mesh(mesh) => {
                    let vertex_offset = vertex_count;
                    let index_offset = index_buffer.len();

                    let mesh_vertex_buffer: &[u8] = bytemuck::cast_slice(&mesh.vertices);
                    vertex_buffer.extend_from_slice(mesh_vertex_buffer);

                    vertex_count += mesh.vertices.len() as u32;

                    index_buffer.extend_from_slice(&mesh.indices);

                    vertex_offsets.push(vertex_offset);
                    indices.push((index_offset, mesh.indices.len()));
                }
                Primitive::Callback(_) => {}
            }
        }

        if vertex_buffer.len() as u64 > self.vertex_buffer.len() {
            self.vertex_buffer = context.create_buffer(rhi::BufferAllocation {
                usage: rhi::BufferUsage::VERTEX,
                location: rhi::BufferLocation::Cpu,
                size: vertex_buffer.len() as u64,
            });
        }

        if index_buffer.len() as u64 * 4 > self.index_buffer.len() {
            self.index_buffer = context.create_buffer(rhi::BufferAllocation {
                usage: rhi::BufferUsage::INDEX,
                location: rhi::BufferLocation::Cpu,
                size: index_buffer.len() as u64 * 4,
            });
        }

        self.vertex_buffer.write_data(0, &vertex_buffer);
        self.index_buffer
            .write_data(0, bytemuck::cast_slice(&index_buffer));

        let uniform_buffer_data = Uniforms {
            viewport_size,
            ..Default::default()
        };

        self.uniform_buffer.write_data(0, bytemuck::bytes_of(&uniform_buffer_data));
        cbuf.set_bind_group(&self.bind_group);

        cbuf.bind_pipeline(&self.pipeline);

        cbuf.bind_vertex_buffer(&self.vertex_buffer, 5 * 4);
        cbuf.bind_index_buffer(&self.index_buffer);

        // cbuf.set_push_constants(&self.pipeline, 0, bytemuck::bytes_of(&PushConstants {
        //     viewport_size,
        // }));

        for ((index_offset, index_count), vertex_offset) in
            indices.into_iter().zip(vertex_offsets.into_iter())
        {
            cbuf.draw_indexed(
                index_count as u32,
                1,
                index_offset as u32,
                vertex_offset as i32,
                0,
            );
        }

    }
}
