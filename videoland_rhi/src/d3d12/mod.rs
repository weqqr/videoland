use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::Mutex;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tracing::info;
use windows::core::{Interface, PCSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::{Direct3D::*, Direct3D12::*, Dxgi::*};
use windows::Win32::System::Threading::*;

use crate::{
    BindGroupLayoutDesc, BindingResource, BufferLocation, Extent2D, Scissor, TextureFormat,
    TextureLayout, VertexFormat, Viewport,
};

const DEBUG_ENABLED: bool = true;
const FRAME_COUNT: u32 = 3;
const FEATURE_LEVEL: D3D_FEATURE_LEVEL = D3D_FEATURE_LEVEL_12_0;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Instance has no supported devices")]
    NoDevices,

    #[error("winapi error: {0}")]
    Windows(#[from] windows::core::Error),
}

pub struct Pipeline {
    pso: ID3D12PipelineState,
}

pub struct Buffer {
    buffer: ID3D12Resource,
    len: u64,
}

impl Buffer {
    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn write_data(&self, offset: u64, data: &[u8]) {
        assert!(offset + data.len() as u64 <= self.len);

        unsafe {
            let written_range = D3D12_RANGE {
                Begin: offset as usize,
                End: offset as usize + data.len(),
            };
            let mut mapped_ptr = std::ptr::null_mut();
            self.buffer.Map(0, None, Some(&mut mapped_ptr)).unwrap();
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                mapped_ptr.offset(offset as isize) as *mut u8,
                data.len(),
            );
            self.buffer.Unmap(0, Some(&written_range));
        }
    }
}

pub struct Texture {
    texture: ID3D12Resource,
}

pub struct TextureView {}

pub struct ShaderModule {
    data: Vec<u8>,
}

pub struct SwapchainFrame {
    texture: Rc<Texture>,
}

impl SwapchainFrame {
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}

pub struct CommandBuffer {
    rtv_dsv_allocator: Rc<RtvDsvAllocator>,
    device: ID3D12Device5,
    gpu_descriptor_allocator: Rc<Mutex<DescriptorAllocator>>,
    command_list: ID3D12GraphicsCommandList,
    referenced_objects: Rc<RefCell<Vec<ID3D12Resource>>>,
}

impl CommandBuffer {
    pub fn set_scissor(&self, scissor: Scissor) {
        unsafe {
            self.command_list.RSSetScissorRects(&[RECT {
                left: scissor.offset.x as i32,
                top: scissor.offset.y as i32,
                right: (scissor.offset.x + scissor.extent.width) as i32,
                bottom: (scissor.offset.y + scissor.extent.height) as i32,
            }])
        }
    }

    pub fn set_viewport(&self, viewport: Viewport) {
        unsafe {
            self.command_list.RSSetViewports(&[D3D12_VIEWPORT {
                TopLeftX: viewport.x,
                TopLeftY: viewport.y,
                Width: viewport.width,
                Height: viewport.height,
                MinDepth: viewport.min_depth,
                MaxDepth: viewport.max_depth,
            }]);
        }
    }

    pub fn texture_barrier(
        &self,
        old_layout: TextureLayout,
        new_layout: TextureLayout,
        texture: &Texture,
    ) {
        unsafe {
            let barrier = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: std::mem::transmute_copy(&texture.texture),
                        StateBefore: old_layout.into(),
                        StateAfter: new_layout.into(),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                    }),
                },
            };

            self.command_list.ResourceBarrier(&[barrier]);
        }
    }

    pub fn set_render_target(&self, target: &Texture) {
        unsafe {
            let descriptor = self
                .rtv_dsv_allocator
                .create_rtv_descriptor(&target.texture);

            self.command_list
                .OMSetRenderTargets(1, Some(&descriptor), FALSE, None);
        }
    }

    pub fn clear_texture(&self, texture: &Texture, color: [f32; 4]) {
        unsafe {
            let descriptor = self
                .rtv_dsv_allocator
                .create_rtv_descriptor(&texture.texture);
            self.command_list
                .ClearRenderTargetView(descriptor, &color, None);
        }
    }

    pub fn set_bind_group(&self, bind_group: &BindGroup) {
        unsafe {
            self.command_list
                .SetGraphicsRootSignature(&bind_group.root_signature);

            let cpu_range = &bind_group.descriptor_range;
            let gpu_range = self
                .gpu_descriptor_allocator
                .lock()
                .unwrap()
                .allocate_range(bind_group.descriptor_range.size);

            self.device.CopyDescriptorsSimple(
                cpu_range.size as u32,
                gpu_range.cpu,
                cpu_range.cpu,
                cpu_range.ty,
            );

            self.command_list
                .SetGraphicsRootDescriptorTable(0, gpu_range.gpu);
        }
    }

    pub fn bind_pipeline(&self, pipeline: &Pipeline) {
        unsafe {
            self.command_list.SetPipelineState(&pipeline.pso);
        }
    }

    pub fn bind_vertex_buffer(&self, vertex_buffer: &Buffer, stride: u32) {
        unsafe {
            self.command_list
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            let view = D3D12_VERTEX_BUFFER_VIEW {
                BufferLocation: vertex_buffer.buffer.GetGPUVirtualAddress(),
                StrideInBytes: stride,
                SizeInBytes: vertex_buffer.len() as u32,
            };

            self.command_list.IASetVertexBuffers(0, Some(&[view]));
        }
    }

    pub fn copy_buffer_to_buffer(&self, src: &Buffer, dst: &Buffer, len: u64) {
        self.referenced_objects.borrow_mut().push(src.buffer.clone());
        self.referenced_objects.borrow_mut().push(dst.buffer.clone());

        unsafe {
            self.command_list
                .CopyBufferRegion(&dst.buffer, 0, &src.buffer, 0, len);
        }
    }

    pub fn copy_buffer_to_texture(&self, buffer: &Buffer, texture: &Texture) {
        self.referenced_objects.borrow_mut().push(buffer.buffer.clone());
        self.referenced_objects.borrow_mut().push(texture.texture.clone());

        unsafe {
            let dst = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&texture.texture),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: 0,
                },
            };

            let texture_desc = texture.texture.GetDesc();

            let mut layouts = vec![Default::default(); 1];

            self.device.GetCopyableFootprints(
                &texture_desc,
                0,
                1,
                0,
                Some(layouts.as_mut_ptr()),
                None,
                None,
                None,
            );

            let src = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&buffer.buffer),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: layouts[0],
                },
            };

            self.command_list
                .CopyTextureRegion(&dst, 0, 0, 0, &src, None);
        }
    }

    pub fn bind_index_buffer(&self, vertex_buffer: &Buffer) {
        unsafe {
            let view = D3D12_INDEX_BUFFER_VIEW {
                BufferLocation: vertex_buffer.buffer.GetGPUVirtualAddress(),
                SizeInBytes: vertex_buffer.len() as u32,
                Format: DXGI_FORMAT_R32_UINT,
            };

            self.command_list.IASetIndexBuffer(Some(&view));
        }
    }

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.command_list.DrawIndexedInstanced(
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }
}

pub struct BindGroupLayout {
    root_signature: ID3D12RootSignature,
}

pub struct BindGroup {
    root_signature: ID3D12RootSignature,
    descriptor_range: DescriptorRange,
}

pub struct Context {
    dxgi_factory: IDXGIFactory6,
    device: ID3D12Device5,
    swapchain: IDXGISwapChain3,
    command_queue: ID3D12CommandQueue,
    render_targets: Vec<Rc<Texture>>,
    command_allocator: ID3D12CommandAllocator,
    rtv_dsv_allocator: Rc<RtvDsvAllocator>,
    descriptor_allocator: Rc<Mutex<DescriptorAllocator>>,
    gpu_descriptor_allocator: Rc<Mutex<DescriptorAllocator>>,
    frame_resources: Rc<RefCell<Vec<ID3D12Resource>>>,

    fence: ID3D12Fence,
    fence_value: u64,
    fence_event: HANDLE,

    frame_index: u32,
}

impl Context {
    pub fn new<W>(window: W, extent: Extent2D) -> Result<Self, Error>
    where
        W: HasWindowHandle,
    {
        unsafe {
            if DEBUG_ENABLED {
                let mut debug: Option<ID3D12Debug> = None;
                if let Some(debug) = D3D12GetDebugInterface(&mut debug).ok().and(debug) {
                    debug.EnableDebugLayer();
                }
            }

            let dxgi_factory_flags = DEBUG_ENABLED
                .then_some(DXGI_CREATE_FACTORY_DEBUG)
                .unwrap_or_default();

            let dxgi_factory: IDXGIFactory6 = CreateDXGIFactory2(dxgi_factory_flags)?;

            let adapter = select_adapter(&dxgi_factory)?;

            let mut device: Option<ID3D12Device5> = None;
            D3D12CreateDevice(&adapter, FEATURE_LEVEL, &mut device)?;
            let device = device.unwrap();

            let command_queue: ID3D12CommandQueue = device
                .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                    ..Default::default()
                })
                .unwrap();

            let window_handle = match window.window_handle().unwrap().as_raw() {
                RawWindowHandle::Win32(window_handle) => window_handle,
                _ => unimplemented!(),
            };

            let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
                BufferCount: FRAME_COUNT,
                Width: extent.width,
                Height: extent.height,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    ..Default::default()
                },
                ..Default::default()
            };

            let swapchain: IDXGISwapChain3 = dxgi_factory
                .CreateSwapChainForHwnd(
                    &command_queue,
                    HWND(window_handle.hwnd.get()),
                    &swapchain_desc,
                    None,
                    None,
                )
                .unwrap()
                .cast()
                .unwrap();

            let mut render_targets = Vec::new();

            for i in 0..FRAME_COUNT {
                let render_target: ID3D12Resource = swapchain.GetBuffer(i).unwrap();

                render_targets.push(Rc::new(Texture {
                    texture: render_target,
                }));
            }

            let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE).unwrap();
            let fence_value = 1;
            let fence_event = CreateEventW(None, false, false, None).unwrap();

            let frame_index = swapchain.GetCurrentBackBufferIndex();

            let command_allocator: ID3D12CommandAllocator = device
                .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
                .unwrap();

            let rtv_dsv_allocator = Rc::new(RtvDsvAllocator::new(device.clone()));
            let descriptor_allocator = Rc::new(Mutex::new(DescriptorAllocator::new(
                device.clone(),
                DescriptorVisibility::Cpu,
            )));
            let gpu_descriptor_allocator = Rc::new(Mutex::new(DescriptorAllocator::new(
                device.clone(),
                DescriptorVisibility::CpuAndGpu,
            )));

            Ok(Self {
                dxgi_factory,
                device,
                swapchain,
                command_queue,
                render_targets,
                command_allocator,
                rtv_dsv_allocator,
                descriptor_allocator,
                gpu_descriptor_allocator,
                frame_resources: Rc::new(RefCell::new(Vec::new())),

                fence,
                fence_value,
                fence_event,

                frame_index,
            })
        }
    }

    unsafe fn wait_for_gpu(&mut self) {
        self.command_queue
            .Signal(&self.fence, self.fence_value)
            .unwrap();
        self.fence
            .SetEventOnCompletion(self.fence_value, self.fence_event)
            .unwrap();
        WaitForSingleObject(self.fence_event, INFINITE);
        self.fence_value += 1;
    }

    unsafe fn wait_for_previous_frame(&mut self) {
        let fence = self.fence_value;

        self.command_queue.Signal(&self.fence, fence).ok().unwrap();

        self.fence_value += 1;

        if self.fence.GetCompletedValue() < fence {
            self.fence
                .SetEventOnCompletion(fence, self.fence_event)
                .ok()
                .unwrap();

            WaitForSingleObject(self.fence_event, INFINITE);
        }
    }

    pub fn create_buffer(&self, allocation: crate::BufferAllocation) -> Buffer {
        unsafe {
            let mut buffer: Option<ID3D12Resource> = None;
            self.device
                .CreateCommittedResource(
                    &D3D12_HEAP_PROPERTIES {
                        Type: allocation.location.into(),
                        ..Default::default()
                    },
                    D3D12_HEAP_FLAG_NONE,
                    &D3D12_RESOURCE_DESC {
                        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                        Width: allocation.size,
                        Height: 1,
                        DepthOrArraySize: 1,
                        MipLevels: 1,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                        ..Default::default()
                    },
                    D3D12_RESOURCE_STATE_GENERIC_READ,
                    None,
                    &mut buffer,
                )
                .unwrap();

            Buffer {
                buffer: buffer.unwrap(),
                len: allocation.size,
            }
        }
    }

    pub fn create_shader_module(&self, data: Vec<u8>) -> ShaderModule {
        ShaderModule { data }
    }

    pub fn create_bind_group_layout(&self, desc: &BindGroupLayoutDesc) -> BindGroupLayout {
        unsafe {
            let mut ranges = Vec::new();

            for entry in desc.entries {
                let range_type = match entry.ty {
                    crate::BindingType::Uniform => D3D12_DESCRIPTOR_RANGE_TYPE_CBV,
                };

                ranges.push(D3D12_DESCRIPTOR_RANGE1 {
                    RangeType: range_type,
                    NumDescriptors: 1,
                    BaseShaderRegister: entry.binding,
                    RegisterSpace: 0,
                    OffsetInDescriptorsFromTableStart: D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND,
                    Flags: D3D12_DESCRIPTOR_RANGE_FLAG_DATA_VOLATILE,
                });
            }

            let mut parameters = Vec::new();

            if !ranges.is_empty() {
                parameters.push(D3D12_ROOT_PARAMETER1 {
                    ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                    Anonymous: D3D12_ROOT_PARAMETER1_0 {
                        DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE1 {
                            NumDescriptorRanges: ranges.len() as u32,
                            pDescriptorRanges: ranges.as_ptr(),
                        },
                    },
                    ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
                });
            }

            let root_signature_flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT
                | D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS
                | D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS
                | D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS
                | D3D12_ROOT_SIGNATURE_FLAG_DENY_PIXEL_SHADER_ROOT_ACCESS;

            let mut root_signature_desc1 = D3D12_ROOT_SIGNATURE_DESC1 {
                Flags: root_signature_flags,
                ..Default::default()
            };

            if !parameters.is_empty() {
                root_signature_desc1.NumParameters = parameters.len() as u32;
                root_signature_desc1.pParameters = parameters.as_ptr();
            }

            let root_signature_desc = D3D12_VERSIONED_ROOT_SIGNATURE_DESC {
                Version: D3D_ROOT_SIGNATURE_VERSION_1_1,
                Anonymous: D3D12_VERSIONED_ROOT_SIGNATURE_DESC_0 {
                    Desc_1_1: root_signature_desc1,
                },
            };

            let mut signature = None;

            D3D12SerializeVersionedRootSignature(&root_signature_desc, &mut signature, None)
                .unwrap();
            let signature = signature.unwrap();

            let root_signature: ID3D12RootSignature = self
                .device
                .CreateRootSignature(
                    0,
                    std::slice::from_raw_parts(
                        signature.GetBufferPointer() as _,
                        signature.GetBufferSize(),
                    ),
                )
                .unwrap();

            BindGroupLayout { root_signature }
        }
    }

    pub fn create_bind_group(&self, desc: &crate::BindGroupDesc) -> BindGroup {
        let descriptor_range = unsafe {
            self.descriptor_allocator
                .lock()
                .unwrap()
                .allocate_range(desc.entries.len())
        };

        for (i, entry) in desc.entries.iter().enumerate() {
            match entry.resource {
                BindingResource::Buffer(buffer) => unsafe {
                    let cbv_desc = D3D12_CONSTANT_BUFFER_VIEW_DESC {
                        BufferLocation: buffer.buffer.GetGPUVirtualAddress(),
                        SizeInBytes: buffer.len as u32,
                    };

                    self.device.CreateConstantBufferView(
                        Some(&cbv_desc),
                        descriptor_range.cpu_descriptor(i),
                    );
                },
            }
        }

        BindGroup {
            root_signature: desc.layout.root_signature.clone(),
            descriptor_range,
        }
    }

    pub fn create_pipeline(&self, desc: &crate::PipelineDesc) -> Pipeline {
        unsafe {
            let mut semantics = Vec::new();
            let mut input_element_descs = Vec::new();

            for (i, attribute) in desc.vertex_layout.attributes.iter().enumerate() {
                let semantic = CString::new(attribute.semantic).unwrap();
                semantics.push(semantic);

                input_element_descs.push(D3D12_INPUT_ELEMENT_DESC {
                    SemanticName: PCSTR(semantics[i].as_bytes().as_ptr()),
                    SemanticIndex: 0,
                    Format: attribute.format.into(),
                    InputSlot: 0,
                    AlignedByteOffset: attribute.offset,
                    InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                })
            }

            let mut desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                InputLayout: D3D12_INPUT_LAYOUT_DESC {
                    pInputElementDescs: input_element_descs.as_mut_ptr(),
                    NumElements: input_element_descs.len() as u32,
                },
                pRootSignature: std::mem::transmute_copy(&desc.bind_group_layout.root_signature),
                VS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: desc.vertex.data.as_ptr() as *const c_void,
                    BytecodeLength: desc.vertex.data.len(),
                },
                PS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: desc.fragment.data.as_ptr() as *const c_void,
                    BytecodeLength: desc.fragment.data.len(),
                },
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    ..Default::default()
                },
                BlendState: D3D12_BLEND_DESC {
                    AlphaToCoverageEnable: false.into(),
                    IndependentBlendEnable: false.into(),
                    RenderTarget: Default::default(),
                },
                DepthStencilState: D3D12_DEPTH_STENCIL_DESC::default(),
                SampleMask: u32::max_value(),
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                NumRenderTargets: 1,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    ..Default::default()
                },
                ..Default::default()
            };

            desc.BlendState.RenderTarget[0] = D3D12_RENDER_TARGET_BLEND_DESC {
                BlendEnable: true.into(),
                LogicOpEnable: false.into(),
                SrcBlend: D3D12_BLEND_SRC_ALPHA,
                DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D12_BLEND_OP_ADD,
                SrcBlendAlpha: D3D12_BLEND_ONE,
                DestBlendAlpha: D3D12_BLEND_ZERO,
                BlendOpAlpha: D3D12_BLEND_OP_ADD,
                LogicOp: D3D12_LOGIC_OP_NOOP,
                RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            desc.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;

            let pso: ID3D12PipelineState = self.device.CreateGraphicsPipelineState(&desc).unwrap();

            Pipeline { pso }
        }
    }

    pub fn immediate_submit(&self, callback: impl FnOnce(&CommandBuffer)) {
        unsafe {
            let command_list: ID3D12GraphicsCommandList = self
                .device
                .CreateCommandList(
                    0,
                    D3D12_COMMAND_LIST_TYPE_DIRECT,
                    &self.command_allocator,
                    None,
                )
                .unwrap();

            let cmd = CommandBuffer {
                rtv_dsv_allocator: Rc::clone(&self.rtv_dsv_allocator),
                device: self.device.clone(),
                gpu_descriptor_allocator: Rc::clone(&self.gpu_descriptor_allocator),
                command_list,
                referenced_objects: Rc::clone(&self.frame_resources),
            };

            callback(&cmd);

            cmd.command_list.Close().unwrap();
            let command_list = Some(cmd.command_list.cast().unwrap());
            self.command_queue.ExecuteCommandLists(&[command_list]);
        }
    }

    pub fn create_texture(&self, desc: &crate::TextureDesc) -> Texture {
        unsafe {
            let mut texture: Option<ID3D12Resource> = None;
            self.device
                .CreateCommittedResource(
                    &D3D12_HEAP_PROPERTIES {
                        Type: D3D12_HEAP_TYPE_DEFAULT,
                        ..Default::default()
                    },
                    D3D12_HEAP_FLAG_NONE,
                    &D3D12_RESOURCE_DESC {
                        Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                        Format: desc.format.into(),
                        Width: desc.extent.width as u64,
                        Height: desc.extent.height,
                        DepthOrArraySize: desc.extent.depth as u16,
                        MipLevels: 1,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        ..Default::default()
                    },
                    D3D12_RESOURCE_STATE_COPY_DEST,
                    None,
                    &mut texture,
                )
                .unwrap();

            Texture {
                texture: texture.unwrap(),
            }
        }
    }

    pub fn resize_swapchain(&mut self, extent: crate::Extent2D) {
        unsafe {
            self.wait_for_gpu();

            self.render_targets.drain(..);

            self.swapchain
                .ResizeBuffers(0, extent.width, extent.height, DXGI_FORMAT_UNKNOWN, 0)
                .unwrap();

            for i in 0..FRAME_COUNT {
                let texture: ID3D12Resource = self.swapchain.GetBuffer(i).unwrap();

                self.render_targets.push(Rc::new(Texture { texture }));
            }
        }
    }

    #[must_use]
    pub fn acquire_next_frame(&mut self) -> SwapchainFrame {
        unsafe {
            self.command_allocator.Reset().unwrap();
            self.frame_resources.borrow_mut().clear();
            self.frame_index = self.swapchain.GetCurrentBackBufferIndex();
        }

        SwapchainFrame {
            texture: Rc::clone(&self.render_targets[self.frame_index as usize]),
        }
    }

    pub fn submit_frame(&mut self) {
        unsafe {
            self.swapchain.Present(1, 0).unwrap();
            self.wait_for_previous_frame();
        }
    }

    pub fn begin_command_buffer(&self) -> CommandBuffer {
        unsafe {
            let command_list: ID3D12GraphicsCommandList = self
                .device
                .CreateCommandList(
                    0,
                    D3D12_COMMAND_LIST_TYPE_DIRECT,
                    &self.command_allocator,
                    None,
                )
                .unwrap();

            command_list.SetDescriptorHeaps(&[Some(
                self.gpu_descriptor_allocator
                    .lock()
                    .unwrap()
                    .cbv_srv_uav_heap
                    .clone(),
            )]);

            CommandBuffer {
                rtv_dsv_allocator: Rc::clone(&self.rtv_dsv_allocator),
                device: self.device.clone(),
                gpu_descriptor_allocator: Rc::clone(&self.gpu_descriptor_allocator),
                command_list,
                referenced_objects: Rc::clone(&self.frame_resources),
            }
        }
    }

    pub fn submit_command_buffer(&self, cmd: CommandBuffer) {
        unsafe {
            cmd.command_list.Close().unwrap();
            let command_list = Some(cmd.command_list.cast().unwrap());
            self.command_queue.ExecuteCommandLists(&[command_list]);
        }
    }
}

unsafe fn select_adapter(factory: &IDXGIFactory6) -> Result<IDXGIAdapter1, Error> {
    for i in 0.. {
        let adapter: IDXGIAdapter1 = factory
            .EnumAdapterByGpuPreference(i, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE)
            .unwrap();

        let mut desc = Default::default();
        adapter.GetDesc1(&mut desc).unwrap();

        let name = String::from_utf16(&desc.Description).unwrap();
        let flags = DXGI_ADAPTER_FLAG(desc.Flags as i32);
        let is_software = (flags & DXGI_ADAPTER_FLAG_SOFTWARE) != DXGI_ADAPTER_FLAG_NONE;

        if is_software {
            continue;
        }

        let is_feature_level_supported = D3D12CreateDevice(
            &adapter,
            FEATURE_LEVEL,
            std::ptr::null_mut::<Option<ID3D12Device5>>(),
        )
        .is_ok();

        info!("Adapter #{i} (software={is_software}): {name}");

        if is_feature_level_supported {
            return Ok(adapter);
        }
    }

    Err(Error::NoDevices)
}

struct Descriptor {
    cpu: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu: D3D12_GPU_DESCRIPTOR_HANDLE,
}

struct DescriptorRange {
    cpu: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu: D3D12_GPU_DESCRIPTOR_HANDLE,
    ty: D3D12_DESCRIPTOR_HEAP_TYPE,
    descriptor_size: usize,
    size: usize,
}

impl DescriptorRange {
    fn cpu_descriptor(&self, i: usize) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        assert!(i < self.size);

        D3D12_CPU_DESCRIPTOR_HANDLE {
            ptr: self.cpu.ptr + i * self.descriptor_size,
        }
    }
}

enum DescriptorVisibility {
    Cpu,
    CpuAndGpu,
}

struct DescriptorAllocator {
    cbv_srv_uav_heap: ID3D12DescriptorHeap,
    cbv_srv_uav_descriptor_size: usize,

    start_cpu_descriptor: D3D12_CPU_DESCRIPTOR_HANDLE,
    start_gpu_descriptor: D3D12_GPU_DESCRIPTOR_HANDLE,

    next_descriptor_index: usize,
}

impl DescriptorAllocator {
    unsafe fn new(device: ID3D12Device5, visibility: DescriptorVisibility) -> Self {
        let cbv_srv_uav_heap: ID3D12DescriptorHeap = device
            .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                NumDescriptors: 100000,
                Flags: match visibility {
                    DescriptorVisibility::Cpu => D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                    DescriptorVisibility::CpuAndGpu => D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                },
                ..Default::default()
            })
            .unwrap();

        let cbv_srv_uav_descriptor_size = device
            .GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV)
            as usize;

        let start_cpu_descriptor = cbv_srv_uav_heap.GetCPUDescriptorHandleForHeapStart();
        let start_gpu_descriptor = cbv_srv_uav_heap.GetGPUDescriptorHandleForHeapStart();

        Self {
            cbv_srv_uav_heap,
            cbv_srv_uav_descriptor_size,

            start_cpu_descriptor,
            start_gpu_descriptor,

            next_descriptor_index: 0,
        }
    }

    unsafe fn allocate(&mut self) -> Descriptor {
        let offset = self.cbv_srv_uav_descriptor_size * self.next_descriptor_index;

        let descriptor = Descriptor {
            cpu: D3D12_CPU_DESCRIPTOR_HANDLE {
                ptr: self.start_cpu_descriptor.ptr + offset,
            },
            gpu: D3D12_GPU_DESCRIPTOR_HANDLE {
                ptr: self.start_gpu_descriptor.ptr + offset as u64,
            },
        };

        self.next_descriptor_index += 1;

        descriptor
    }

    unsafe fn allocate_range(&mut self, size: usize) -> DescriptorRange {
        let offset = self.cbv_srv_uav_descriptor_size * self.next_descriptor_index;

        let descriptor_range = DescriptorRange {
            cpu: D3D12_CPU_DESCRIPTOR_HANDLE {
                ptr: self.start_cpu_descriptor.ptr + offset,
            },
            gpu: D3D12_GPU_DESCRIPTOR_HANDLE {
                ptr: self.start_gpu_descriptor.ptr + offset as u64,
            },
            descriptor_size: self.cbv_srv_uav_descriptor_size,
            ty: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            size,
        };

        self.next_descriptor_index += size;

        descriptor_range
    }
}

// https://asawicki.info/news_1772_secrets_of_direct3d_12_do_rtv_and_dsv_descriptors_make_any_sense
struct RtvDsvAllocator {
    device: ID3D12Device5,
    rtv_heap: ID3D12DescriptorHeap,
    rtv_descriptor_size: usize,
}

impl RtvDsvAllocator {
    unsafe fn new(device: ID3D12Device5) -> Self {
        let rtv_heap: ID3D12DescriptorHeap = device
            .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                NumDescriptors: FRAME_COUNT,
                ..Default::default()
            })
            .unwrap();

        let rtv_descriptor_size =
            device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) as usize;

        Self {
            device,
            rtv_heap,
            rtv_descriptor_size,
        }
    }

    unsafe fn create_rtv_descriptor(
        &self,
        texture: &ID3D12Resource,
    ) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        let descriptor = D3D12_CPU_DESCRIPTOR_HANDLE {
            ptr: self.rtv_heap.GetCPUDescriptorHandleForHeapStart().ptr,
        };

        self.device
            .CreateRenderTargetView(texture, None, descriptor);

        descriptor
    }
}

impl From<BufferLocation> for D3D12_HEAP_TYPE {
    fn from(value: BufferLocation) -> Self {
        match value {
            BufferLocation::Cpu => D3D12_HEAP_TYPE_UPLOAD,
            BufferLocation::Gpu => D3D12_HEAP_TYPE_DEFAULT,
        }
    }
}

impl From<VertexFormat> for DXGI_FORMAT {
    fn from(value: VertexFormat) -> Self {
        match value {
            VertexFormat::Uint32x1 => DXGI_FORMAT_R32_UINT,
            VertexFormat::Float32x1 => DXGI_FORMAT_R32_FLOAT,
            VertexFormat::Float32x2 => DXGI_FORMAT_R32G32_FLOAT,
            VertexFormat::Float32x3 => DXGI_FORMAT_R32G32B32_FLOAT,
            VertexFormat::Float32x4 => DXGI_FORMAT_R32G32B32A32_FLOAT,
        }
    }
}

impl From<TextureFormat> for DXGI_FORMAT {
    fn from(value: TextureFormat) -> Self {
        match value {
            TextureFormat::R8G8B8A8Uint => DXGI_FORMAT_R8G8B8A8_UINT,
            TextureFormat::R32Float => DXGI_FORMAT_R32_FLOAT,
        }
    }
}

impl From<TextureLayout> for D3D12_RESOURCE_STATES {
    fn from(value: TextureLayout) -> Self {
        match value {
            TextureLayout::Undefined => todo!(),
            TextureLayout::General => todo!(),
            TextureLayout::Color => D3D12_RESOURCE_STATE_RENDER_TARGET,
            TextureLayout::DepthStencil => todo!(),
            TextureLayout::Present => D3D12_RESOURCE_STATE_PRESENT,
            TextureLayout::TransferSrc => todo!(),
            TextureLayout::TransferDst => todo!(),
        }
    }
}
