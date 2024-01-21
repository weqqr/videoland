#![allow(dead_code)]

#[cfg(feature = "vk")]
pub mod vk2;

#[cfg(feature = "vk")]
pub use vk2::*;

use bitflags::bitflags;

#[derive(Debug, Clone, Copy)]
pub struct ShaderStages(u32);

bitflags! {
    impl ShaderStages: u32 {
        const VERTEX = 1 << 0;
        const FRAGMENT = 1 << 0;
    }
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
pub struct BufferUsage(u32);

bitflags! {
    impl BufferUsage: u32 {
        const VERTEX = 1 << 0;
        const INDEX = 1 << 0;
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
    pub vertex_layout: VertexBufferLayout<'a>,
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
pub struct VertexAttribute {
    pub binding: u32,
    pub location: u32,
    pub offset: u32,
    pub format: VertexFormat,
}

#[derive(Clone)]
pub struct VertexBufferLayout<'a> {
    pub attributes: &'a [VertexAttribute],
    pub stride: u32,
}
