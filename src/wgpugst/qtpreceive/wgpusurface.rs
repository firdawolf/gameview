use iced_wgpu::wgpu::{self, Origin3d};
use iced_winit::Color;
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1.
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2.
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        // 3.
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Resolution {
    inputwidth: f32,
    inputheight: f32,
    outputwidth: f32,
    outputheight: f32,
}
impl Resolution {
    fn new(inputwidth: f32, inputheight: f32, outputwidth: f32, outputheight: f32) -> Resolution {
        Resolution {
            inputwidth: inputwidth,
            inputheight: inputheight,
            outputwidth: outputwidth,
            outputheight: outputheight,
        }
    }
}

const INDICES: &[u16] = &[0, 1, 3, 1, 2, 3];

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Wgpusurface {
    y_texture: wgpu::Texture,
    u_texture: wgpu::Texture,
    rgba_texture: wgpu::Texture,
    easu_texture: wgpu::Texture,
    yuv_bind_group: wgpu::BindGroup,
    compute_yuv_bind_group: wgpu::BindGroup,
    easu_texture_view: wgpu::TextureView,
    easu_bind_group: wgpu::BindGroup,
    compute_easu_bind_group: wgpu::BindGroup,
    lcas_texture_view: wgpu::TextureView,
    lcas_bind_group: wgpu::BindGroup,
    rcas_texture_view: wgpu::TextureView,
    rcas_bind_group: wgpu::BindGroup,
    resolution_bind_group: wgpu::BindGroup,
    compute_resolution_bind_group: wgpu::BindGroup,
    camera_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    render_pipeline_yuv: wgpu::RenderPipeline,
    compute_pipeline_yuv: wgpu::ComputePipeline,
    render_pipeline_easu: wgpu::RenderPipeline,
    compute_pipeline_easu: wgpu::ComputePipeline,
    render_pipeline_lcas: wgpu::RenderPipeline,
    render_pipeline_rcas: wgpu::RenderPipeline,
}

impl Wgpusurface {
    pub fn new(
        device: &wgpu::Device,
        swapchain_format: wgpu::TextureFormat,
        size1: PhysicalSize<u32>,
        size2: PhysicalSize<u32>,
    ) -> Wgpusurface {
        build_pipeline(device, swapchain_format, size1, size2)
    }
    pub fn yuv_renderpass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.easu_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        })
    }
    pub fn yuv_computepass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::ComputePass<'a> {
        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None })
    }
    pub fn easu_computepass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::ComputePass<'a> {
        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None })
    }
    pub fn easu_renderpass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.lcas_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        })
    }
    pub fn lcas_renderpass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.rcas_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        })
    }
    pub fn rcas_renderpass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        rcas_view: &'a wgpu::TextureView,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &rcas_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        })
    }
    pub fn transfertexture<'a>(
        &'a self,
        compute_encoder: &'a mut wgpu::CommandEncoder,
        size1: PhysicalSize<u32>,
    ) {
        compute_encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.rgba_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &self.easu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: size1.width,
                height: size1.height,
                depth_or_array_layers: 1,
            },
        )
    }
    pub fn write_texture<'a>(
        &'a self,
        queue: &wgpu::Queue,
        size1: PhysicalSize<u32>,
        data_y: &[u8],
        data_uv: &[u8],
    ) {
        let texture_size = wgpu::Extent3d {
            width: size1.width,
            height: size1.height,
            depth_or_array_layers: 1,
        };
        let texture_size2 = wgpu::Extent3d {
            width: size1.width / 2,
            height: size1.height / 2,
            depth_or_array_layers: 1,
        };
        queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &self.y_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data_y,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(size1.width as u32),
                rows_per_image: std::num::NonZeroU32::new(size1.height as u32),
            },
            texture_size,
        );
        queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &self.u_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data_uv,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(size1.width as u32),
                rows_per_image: std::num::NonZeroU32::new(size1.height as u32),
            },
            texture_size2,
        );
    }

    pub fn yuv_draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&self.render_pipeline_yuv);
        rpass.set_bind_group(0, &self.yuv_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
        rpass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
    }
    pub fn yuv_compute<'a>(&'a self, size1: PhysicalSize<u32>, cpass: &mut wgpu::ComputePass<'a>) {
        cpass.set_pipeline(&self.compute_pipeline_yuv);
        cpass.set_bind_group(0, &self.compute_yuv_bind_group, &[]);
        cpass.dispatch_workgroups(size1.width / 4, size1.height / 4, 1);
    }
    pub fn easu_draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&self.render_pipeline_easu);
        rpass.set_bind_group(0, &self.easu_bind_group, &[]);
        rpass.set_bind_group(1, &self.resolution_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
        rpass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
    }
    pub fn easu_compute<'a>(&'a self, size1: PhysicalSize<u32>, cpass: &mut wgpu::ComputePass<'a>) {
        cpass.set_pipeline(&self.compute_pipeline_easu);
        cpass.set_bind_group(0, &self.compute_easu_bind_group, &[]);
        cpass.set_bind_group(1, &self.compute_resolution_bind_group, &[]);
        cpass.dispatch_workgroups(size1.width / 8, size1.height / 8, 1);
    }
    pub fn lcas_draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&self.render_pipeline_lcas);
        rpass.set_bind_group(0, &self.lcas_bind_group, &[]);
        rpass.set_bind_group(1, &self.resolution_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
        rpass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
    }
    pub fn rcas_draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&self.render_pipeline_rcas);
        rpass.set_bind_group(0, &self.rcas_bind_group, &[]);
        rpass.set_bind_group(1, &self.resolution_bind_group, &[]);
        rpass.set_bind_group(2, &self.camera_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
        rpass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
    }
}
fn build_pipeline(
    device: &wgpu::Device,
    swapchain_format: wgpu::TextureFormat,
    size1: PhysicalSize<u32>,
    size2: PhysicalSize<u32>,
) -> Wgpusurface {
    let mut camera = Camera {
        // position the camera one unit up and 2 units back
        // +z is out of the screen
        eye: (0.0, 0.0, 2.42).into(),
        // have it look at the origin
        target: (0.0, 0.0, 0.0).into(),
        // which way is "up"
        up: cgmath::Vector3::unit_y(),
        //aspect for ultrawide
        //aspect: 4 as f32 / 3 as f32,
        //aspect for normal aspect
        aspect: 1 as f32 / 1 as f32,
        fovy: 45.0,
        znear: 0.1,
        zfar: 100.0,
    };
    let mut camera_uniform = CameraUniform::new();
    camera_uniform.update_view_proj(&camera);

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let vertices: &[Vertex] = &[
        Vertex {
            position: [-1.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [-1.0, -1.0, 0.0],
            tex_coords: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [1.0, -1.0, 0.0],
            tex_coords: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 0.0],
            tex_coords: [1.0, 0.0, 1.0],
        },
    ];
    let shader_yuv = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("convert.wgsl"))),
    });

    let shader_easu = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("fsr_easu.wgsl"))),
    });

    let shader_lcas = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("lcas.wgsl"))),
    });
    let shader_rcas = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("fsr_rcas.wgsl"))),
    });

    let texture_size = wgpu::Extent3d {
        width: size1.width,
        height: size1.height,
        depth_or_array_layers: 1,
    };
    let u_size = wgpu::Extent3d {
        width: size1.width / 2,
        height: size1.height / 2,
        depth_or_array_layers: 1,
    };
    let y_texture = device.create_texture(&wgpu::TextureDescriptor {
        // All textures are stored as 3D, we represent our 2D texture
        // by setting depth to 1.
        size: texture_size,
        mip_level_count: 1, // We'll talk about this a little later
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Most images are stored using sRGB so we need to reflect that here.
        format: wgpu::TextureFormat::R8Unorm,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("y_texture"),
    });
    let u_texture = device.create_texture(&wgpu::TextureDescriptor {
        // All textures are stored as 3D, we represent our 2D texture
        // by setting depth to 1.
        size: u_size,
        mip_level_count: 1, // We'll talk about this a little later
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Most images are stored using sRGB so we need to reflect that here.
        format: wgpu::TextureFormat::Rg8Unorm,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("uv_texture"),
    });
    let rgba_texture = device.create_texture(&wgpu::TextureDescriptor {
        // All textures are stored as 3D, we represent our 2D texture
        // by setting depth to 1.
        size: texture_size,
        mip_level_count: 1, // We'll talk about this a little later
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Most images are stored using sRGB so we need to reflect that here.
        format: wgpu::TextureFormat::Rgba8Unorm,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING,
        label: Some("diffuse_texture"),
    });
    let easu_texture = device.create_texture(&wgpu::TextureDescriptor {
        // All textures are stored as 3D, we represent our 2D texture
        // by setting depth to 1.
        size: texture_size,
        mip_level_count: 1, // We'll talk about this a little later
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Most images are stored using sRGB so we need to reflect that here.
        // format: wgpu::TextureFormat::Rgba8Unorm,
        format: wgpu::TextureFormat::Rgba8Unorm,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST,
        label: Some("diffuse_texture"),
    });
    let lcas_texture = device.create_texture(&wgpu::TextureDescriptor {
        // All textures are stored as 3D, we represent our 2D texture
        // by setting depth to 1.
        size: wgpu::Extent3d {
            width: size2.width,
            height: size2.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1, // We'll talk about this a little later
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Most images are stored using sRGB so we need to reflect that here.
        format: wgpu::TextureFormat::Rgba8Unorm,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        label: Some("easu_texture"),
    });
    let rcas_texture = device.create_texture(&wgpu::TextureDescriptor {
        // All textures are stored as 3D, we represent our 2D texture
        // by setting depth to 1.
        size: wgpu::Extent3d {
            width: size2.width,
            height: size2.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1, // We'll talk about this a little later
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Most images are stored using sRGB so we need to reflect that here.
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: Some("easu_texture"),
    });
    let y_texture_view = y_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let u_texture_view = u_texture.create_view(&wgpu::TextureViewDescriptor::default());
    //let v_texture_view = v_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let yuv_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToBorder,
        address_mode_v: wgpu::AddressMode::ClampToBorder,
        address_mode_w: wgpu::AddressMode::ClampToBorder,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let uv_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToBorder,
        address_mode_v: wgpu::AddressMode::ClampToBorder,
        address_mode_w: wgpu::AddressMode::ClampToBorder,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let rgba_texture_view = rgba_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let easu_texture_view = rgba_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let easu_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToBorder,
        address_mode_v: wgpu::AddressMode::ClampToBorder,
        address_mode_w: wgpu::AddressMode::ClampToBorder,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let lcas_texture_view = lcas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let lcas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToBorder,
        address_mode_v: wgpu::AddressMode::ClampToBorder,
        address_mode_w: wgpu::AddressMode::ClampToBorder,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let rcas_texture_view = rcas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let rcas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToBorder,
        address_mode_v: wgpu::AddressMode::ClampToBorder,
        address_mode_w: wgpu::AddressMode::ClampToBorder,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let resolution_uniform = Resolution::new(
        size1.width as f32,
        size1.height as f32,
        size2.width as f32,
        size2.height as f32,
    );

    let resolution_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Resolution Image Buffer"),
        contents: bytemuck::cast_slice(&[resolution_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });
    let texture_yuv_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                // wgpu::BindGroupLayoutEntry {
                //     binding: 2,
                //     visibility: wgpu::ShaderStages::FRAGMENT,
                //     ty: wgpu::BindingType::Texture {
                //         multisampled: false,
                //         view_dimension: wgpu::TextureViewDimension::D2,
                //         sample_type: wgpu::TextureSampleType::Float { filterable: false },
                //     },
                //     count: None,
                // },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(
                        // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                        // SamplerBindingType::Filtering if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        wgpu::SamplerBindingType::NonFiltering,
                    ),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
    let compute_texture_yuv_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(
                        // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                        // SamplerBindingType::Filtering if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
    let texture_easu_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(
                        // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                        // SamplerBindingType::Filtering if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
    let compute_texture_easu_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(
                        // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                        // SamplerBindingType::Filtering if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
    let texture_lcas_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(
                        // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                        // SamplerBindingType::Filtering if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        wgpu::SamplerBindingType::NonFiltering,
                    ),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

    let texture_rcas_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(
                        // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                        // SamplerBindingType::Filtering if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        wgpu::SamplerBindingType::NonFiltering,
                    ),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

    let resolution_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Resolution_bind_group_layout"),
        });
    let compute_resolution_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Resolution_bind_group_layout"),
        });
    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
    });
    let yuv_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_yuv_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&y_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&u_texture_view),
            },
            // wgpu::BindGroupEntry {
            //     binding: 2,
            //     resource: wgpu::BindingResource::TextureView(&v_texture_view),
            // },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&yuv_sampler),
            },
        ],
        label: Some("yuv_bind_group"),
    });
    let compute_yuv_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &compute_texture_yuv_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&y_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&u_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&uv_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&rgba_texture_view),
            },
        ],
        label: Some("yuv_bind_group2"),
    });
    let resolution_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &resolution_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: resolution_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
    });
    let compute_resolution_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &compute_resolution_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: resolution_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
    });
    let yuv_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&texture_yuv_bind_group_layout],
        push_constant_ranges: &[],
    });
    let compute_yuv_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&compute_texture_yuv_bind_group_layout],
            push_constant_ranges: &[],
        });

    let easu_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[
            &texture_easu_bind_group_layout,
            &resolution_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });
    let compute_easu_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &compute_texture_easu_bind_group_layout,
                &compute_resolution_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
    let lcas_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[
            &texture_lcas_bind_group_layout,
            &resolution_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });

    let rcas_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[
            &texture_rcas_bind_group_layout,
            &resolution_bind_group_layout,
            &camera_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });
    let compute_pipeline_yuv = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("compute_pipeline_yuv"),
        layout: Some(&compute_yuv_pipeline_layout),
        module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader_module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("yuvtorgb.wgsl"))),
        }),
        entry_point: "main",
    });
    let render_pipeline_yuv = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&yuv_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_yuv,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_yuv,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: swapchain_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 1.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw, // 2.
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });
    let compute_pipeline_easu = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("compute_pipeline_easu"),
        layout: Some(&compute_easu_pipeline_layout),
        module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader_module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("fsr_easu_compute.wgsl"))),
        }),
        entry_point: "main",
    });
    let render_pipeline_easu = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&easu_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_easu,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_easu,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: swapchain_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 1.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw, // 2.
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });
    let render_pipeline_lcas = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&lcas_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_lcas,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_lcas,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: swapchain_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 1.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw, // 2.
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let render_pipeline_rcas = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&rcas_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_rcas,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_rcas,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: swapchain_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 1.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw, // 2.
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let easu_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_easu_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&easu_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&easu_sampler),
            },
        ],
        label: Some("diffuse_bind_group"),
    });
    let compute_easu_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &compute_texture_easu_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&easu_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&easu_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&lcas_texture_view),
            },
        ],
        label: Some("diffuse_bind_group"),
    });
    let lcas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_rcas_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&lcas_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&lcas_sampler),
            },
        ],
        label: Some("diffuse_bind_group"),
    });

    let rcas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_rcas_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&rcas_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&rcas_sampler),
            },
        ],
        label: Some("diffuse_bind_group"),
    });
    Wgpusurface {
        y_texture: y_texture,
        u_texture: u_texture,
        rgba_texture: rgba_texture,
        easu_texture: easu_texture,
        yuv_bind_group: yuv_bind_group,
        compute_yuv_bind_group: compute_yuv_bind_group,
        easu_texture_view: easu_texture_view,
        compute_easu_bind_group: compute_easu_bind_group,
        easu_bind_group: easu_bind_group,
        lcas_texture_view: lcas_texture_view,
        lcas_bind_group: lcas_bind_group,
        rcas_texture_view: rcas_texture_view,
        rcas_bind_group: rcas_bind_group,
        resolution_bind_group: resolution_bind_group,
        compute_resolution_bind_group: compute_resolution_bind_group,
        camera_bind_group: camera_bind_group,
        vertex_buffer: vertex_buffer,
        index_buffer: index_buffer,
        render_pipeline_yuv: render_pipeline_yuv,
        compute_pipeline_yuv: compute_pipeline_yuv,
        render_pipeline_easu: render_pipeline_easu,
        compute_pipeline_easu: compute_pipeline_easu,
        render_pipeline_lcas: render_pipeline_lcas,
        render_pipeline_rcas: render_pipeline_rcas,
    }
}
