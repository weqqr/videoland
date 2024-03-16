#![allow(dead_code)]

#[cfg(feature = "d3d12")]
pub mod d3d12;

#[cfg(feature = "d3d12")]
pub use d3d12::*;

use bitflags::bitflags;

#[derive(Debug, Clone, Copy)]
pub struct ShaderStages(u32);

bitflags! {
    impl ShaderStages: u32 {
        const VERTEX = 1 << 0;
        const FRAGMENT = 1 << 1;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Offset2D {
    pub x: u32,
    pub y: u32,
}

impl Offset2D {
    pub const ZERO: Self = Offset2D { x: 0, y: 0 };
}

#[derive(Debug, Clone, Copy)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Scissor {
    pub offset: Offset2D,
    pub extent: Extent2D,
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct BufferUsage(u32);

bitflags! {
    impl BufferUsage: u32 {
        const VERTEX = 1 << 0;
        const INDEX = 1 << 1;
        const UNIFORM = 1 << 2;
        const TRANSFER_SRC = 1 << 3;
        const TRANSFER_DST = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BufferLocation {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy)]
pub struct BufferAllocation {
    pub usage: BufferUsage,
    pub location: BufferLocation,
    pub size: u64,
}

#[derive(Clone, Copy)]
pub enum DescriptorType {
    SampledTexture,
}

pub struct DescriptorSetLayoutEntry {
    pub binding: u32,
    pub visibility: ShaderStages,
    pub ty: DescriptorType,
}

pub struct DescriptorSetLayoutDesc<'a> {
    pub entries: &'a [DescriptorSetLayoutEntry],
}

pub struct RenderPassDesc<'a> {
    pub color_attachment: &'a TextureView,
    pub depth_attachment: &'a TextureView,
    pub render_area: Extent2D,
}

pub struct PipelineDesc<'a> {
    pub vertex: &'a ShaderModule,
    pub fragment: &'a ShaderModule,
    pub bind_group_layout: &'a BindGroupLayout,
    pub vertex_layout: VertexBufferLayout<'a>,
}

pub struct BindGroupLayoutDesc<'a> {
    pub entries: &'a [BindGroupLayoutEntry],
}

pub struct BindGroupLayoutEntry {
    pub binding: u32,
    pub visibility: ShaderStages,
    pub ty: BindingType,
}

pub enum BindingType {
    Uniform,
}

pub enum BindingResource<'a> {
    Buffer(&'a Buffer),
}

pub struct BindGroupDesc<'a> {
    pub layout: &'a BindGroupLayout,
    pub entries: &'a [BindGroupEntry<'a>],
}

pub struct BindGroupEntry<'a> {
    pub binding: u32,
    pub resource: BindingResource<'a>,
}

pub struct TextureDesc {
    pub extent: Extent3D,
}

pub struct TextureViewDesc {
    pub extent: Extent3D,
}

#[derive(Clone, Copy)]
pub enum TextureLayout {
    Undefined,
    General,
    Color,
    DepthStencil,
    Present,
    TransferSrc,
    TransferDst,
}

#[derive(Clone, Copy)]
pub enum VertexFormat {
    Uint32x1,
    Float32x1,
    Float32x2,
    Float32x3,
    Float32x4,
}

#[derive(Clone)]
pub struct VertexAttribute<'a> {
    pub semantic: &'a str,
    pub binding: u32,
    pub location: u32,
    pub offset: u32,
    pub format: VertexFormat,
}

#[derive(Clone)]
pub struct VertexBufferLayout<'a> {
    pub attributes: &'a [VertexAttribute<'a>],
    pub stride: u32,
}
