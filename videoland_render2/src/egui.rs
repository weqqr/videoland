use egui::epaint::Primitive;
use egui::{ClippedPrimitive, TexturesDelta};
use rhi::BufferAllocation;
use videoland_ap::shader::Shader;
use videoland_rhi as rhi;

#[derive(Default)]
pub struct PreparedUi {
    pub shapes: Vec<ClippedPrimitive>,
    pub textures_delta: TexturesDelta,
}

pub struct EguiRenderer {
    device: rhi::Device,
    vertex_buffer: rhi::Buffer,
    index_buffer: rhi::Buffer,
    pipeline: rhi::Pipeline,
}

impl EguiRenderer {
    pub fn new(device: rhi::Device, vertex_shader: Shader, fragment_shader: Shader) -> Self {
        let initial_buffer_size = 1024 * 1024 * 4;

        let vertex_buffer = device
            .create_buffer(BufferAllocation {
                usage: rhi::BufferUsage::VERTEX,
                location: rhi::BufferLocation::Cpu,
                size: initial_buffer_size,
            })
            .unwrap();

        let index_buffer = device
            .create_buffer(BufferAllocation {
                usage: rhi::BufferUsage::INDEX,
                location: rhi::BufferLocation::Cpu,
                size: initial_buffer_size,
            })
            .unwrap();

        let vertex_shader = device.create_shader_module(vertex_shader.spirv()).unwrap();
        let fragment_shader = device
            .create_shader_module(fragment_shader.spirv())
            .unwrap();

        let descriptor_set_layout = device.create_descriptor_set_layout(&rhi::DescriptorSetLayoutDesc {
            entries: &[rhi::DescriptorSetLayoutEntry {
                binding: 0,
                visibility: rhi::ShaderStages::FRAGMENT,
                ty: rhi::DescriptorType::SampledTexture,
            }],
        });

        let pipeline = device
            .create_pipeline(&rhi::PipelineDesc {
                vertex: &vertex_shader,
                fragment: &fragment_shader,
                vertex_layout: rhi::VertexBufferLayout {
                    attributes: &[
                        rhi::VertexAttribute {
                            binding: 0,
                            location: 0,
                            offset: 0,
                            format: rhi::VertexFormat::Float32x2,
                        },
                        rhi::VertexAttribute {
                            binding: 0,
                            location: 1,
                            offset: 2 * 4,
                            format: rhi::VertexFormat::Float32x2,
                        },
                        rhi::VertexAttribute {
                            binding: 0,
                            location: 2,
                            offset: 4 * 4,
                            format: rhi::VertexFormat::Uint32x1,
                        },
                    ],
                    stride: 5 * 4,
                },
            })
            .unwrap();

        Self {
            device,
            vertex_buffer,
            index_buffer,
            pipeline,
        }
    }

    pub fn render(&mut self, cbuf: &rhi::CommandBuffer, ui: &PreparedUi) {
        let mut vertex_buffer = vec![];
        let mut index_buffer = vec![];

        let mut vertex_count = 0;

        let mut vertex_offsets = vec![];
        let mut indices = vec![];

        for primitive in &ui.shapes {
            match &primitive.primitive {
                Primitive::Mesh(mesh) => {
                    let vertex_offset = vertex_count;
                    let index_offset = indices.len();

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
            println!("Egui Resize: {}", vertex_buffer.len());
            self.vertex_buffer = self
                .device
                .create_buffer(BufferAllocation {
                    usage: rhi::BufferUsage::VERTEX,
                    location: rhi::BufferLocation::Cpu,
                    size: vertex_buffer.len() as u64,
                })
                .unwrap();
        }

        if index_buffer.len() as u64 * 4 > self.index_buffer.len() {
            println!("Egui Resize: {}", index_buffer.len());
            self.index_buffer = self
                .device
                .create_buffer(BufferAllocation {
                    usage: rhi::BufferUsage::INDEX,
                    location: rhi::BufferLocation::Cpu,
                    size: index_buffer.len() as u64,
                })
                .unwrap();
        }

        self.vertex_buffer.write_data(0, &vertex_buffer);
        self.index_buffer
            .write_data(0, bytemuck::cast_slice(&index_buffer));

        cbuf.bind_pipeline(&self.pipeline);
        cbuf.bind_vertex_buffer(&self.vertex_buffer);
        cbuf.bind_index_buffer(&self.index_buffer);

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
