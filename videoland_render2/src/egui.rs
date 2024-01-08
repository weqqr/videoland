use egui::epaint::Primitive;
use egui::{ClippedPrimitive, TexturesDelta};
use rhi::BufferAllocation;
use videoland_rhi as rhi;

#[derive(Default)]
pub struct PreparedUi {
    pub shapes: Vec<ClippedPrimitive>,
    pub textures_delta: TexturesDelta,
}

pub struct EguiRenderer {
    vertex_buffer: rhi::Buffer,
}

impl EguiRenderer {
    pub fn new(device: &rhi::Device2, vertex_spirv: Vec<u32>, fragment_spirv: Vec<u32>) -> Self {
        let initial_buffer_size = 1024 * 1024 * 4;

        let buffer = device
            .create_buffer(BufferAllocation {
                usage: rhi::BufferUsage::VERTEX,
                location: rhi::BufferLocation::Cpu,
                size: initial_buffer_size,
            })
            .unwrap();

        Self {
            vertex_buffer: buffer,
        }
    }

    pub fn render(&self, ui: &PreparedUi) {
        let mut required_vertex_buffer_size = 0;
        let mut required_index_buffer_size = 0;

        for primitive in &ui.shapes {
            match &primitive.primitive {
                Primitive::Mesh(mesh) => {
                    let mesh_buffer: &[u8] = bytemuck::cast_slice(&mesh.vertices);
                    required_vertex_buffer_size += mesh_buffer.len();

                    let index_buffer: &[u8] = bytemuck::cast_slice(&mesh.indices);
                    required_index_buffer_size += index_buffer.len();
                }
                Primitive::Callback(_) => {}
            }
        }
    }
}
