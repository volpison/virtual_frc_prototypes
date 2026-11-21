//! Everything that talks to the graphics card (wgpu): the surface, both pipelines and the two render paths.

mod present;
mod shaders;

use bytemuck::{Pod, Zeroable};
use log::info;
use winit::window::Window;

use crate::frc::color;
use crate::software::{fill_checkerboard, fill_gradient, fill_side_by_side, fill_solid};
use crate::state::{AppState, DisplayMode};
use present::choose_present_mode;
use shaders::{GPU_SHADER, SOFT_SHADER};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    mode: u32,
    phase: u32,
    cell_size: u32,
    _pad: u32,
    color_a: [f32; 4],
    color_b: [f32; 4],
    temporal: [f32; 4],
    static_avg: [f32; 4],
    resolution: [f32; 2],
    _pad2: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
}

pub(crate) struct GpuState<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    // Whether the surface encodes colors for us (sRGB) or stores the codes we write.
    surface_is_srgb: bool,
    // Format of the software texture: it must hold the same codes the surface stores.
    soft_format: wgpu::TextureFormat,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    available_present_modes: Vec<wgpu::PresentMode>,

    // Pure GPU path
    gpu_pipeline: wgpu::RenderPipeline,
    gpu_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,

    // Software path: CPU buffer → texture → textured fullscreen triangle
    soft_pipeline: wgpu::RenderPipeline,
    soft_bind_group: wgpu::BindGroup,
    soft_texture: wgpu::Texture,
    soft_texture_view: wgpu::TextureView,
    soft_sampler: wgpu::Sampler,
    cpu_pixels: Vec<u32>,
}

impl<'a> GpuState<'a> {
    pub(crate) async fn new(window: &'a Window, prefer_vsync: bool) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).expect("surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("adapter");

        info!(
            "Using GPU: {} ({:?})",
            adapter.get_info().name,
            adapter.get_info().backend
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("VirtualFRC"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("device");

        let surface_caps = surface.get_capabilities(&adapter);
        // Prefer a surface that stores plain codes: then the program decides the exact
        // 8-bit codes (see frc::color) and the graphics card does not round them again.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let surface_is_srgb = surface_format.is_srgb();
        let soft_format = if surface_is_srgb {
            wgpu::TextureFormat::Bgra8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8Unorm
        };
        info!(
            "Surface format: {:?} (encodes sRGB itself: {})",
            surface_format, surface_is_srgb
        );
        let present_mode = choose_present_mode(&surface_caps.present_modes, prefer_vsync);
        info!("Present mode: {:?}", present_mode);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        // ---- shared fullscreen triangle ----
        let vertices: [Vertex; 3] = [
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [3.0, -1.0],
            },
            Vertex {
                position: [-1.0, 3.0],
            },
        ];
        let vertex_buffer = {
            use wgpu::util::DeviceExt;
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("verts"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        };

        // ---- GPU (shader) path ----
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let gpu_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
        });
        let gpu_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &gpu_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let gpu_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu_shader"),
            source: wgpu::ShaderSource::Wgsl(GPU_SHADER.into()),
        });
        let gpu_pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&gpu_bgl],
            push_constant_ranges: &[],
        });
        let gpu_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gpu_pipe"),
            layout: Some(&gpu_pll),
            vertex: wgpu::VertexState {
                module: &gpu_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &gpu_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // ---- Software path (texture upload) ----
        let soft_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("soft_tex"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: soft_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let soft_texture_view = soft_texture.create_view(&Default::default());
        let soft_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let soft_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let soft_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &soft_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&soft_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&soft_sampler),
                },
            ],
        });

        let soft_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("soft_shader"),
            source: wgpu::ShaderSource::Wgsl(SOFT_SHADER.into()),
        });
        let soft_pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&soft_bgl],
            push_constant_ranges: &[],
        });
        let soft_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("soft_pipe"),
            layout: Some(&soft_pll),
            vertex: wgpu::VertexState {
                module: &soft_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &soft_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let cpu_pixels = vec![0u32; (width * height) as usize];

        Self {
            surface,
            device,
            queue,
            config,
            surface_is_srgb,
            soft_format,
            size,
            available_present_modes: surface_caps.present_modes,
            gpu_pipeline,
            gpu_bind_group,
            uniform_buffer,
            vertex_buffer,
            soft_pipeline,
            soft_bind_group,
            soft_texture,
            soft_texture_view,
            soft_sampler,
            cpu_pixels,
        }
    }

    pub(crate) fn set_present_mode(&mut self, prefer_vsync: bool) {
        let new_mode = choose_present_mode(&self.available_present_modes, prefer_vsync);
        if new_mode != self.config.present_mode {
            info!("Present mode → {:?}", new_mode);
            self.config.present_mode = new_mode;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);

        // Recreate software texture + CPU buffer
        self.soft_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("soft_tex"),
            size: wgpu::Extent3d {
                width: new_size.width,
                height: new_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.soft_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.soft_texture_view = self.soft_texture.create_view(&Default::default());

        // Rebuild soft bind group (texture view changed)
        let soft_bgl = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        self.soft_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &soft_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.soft_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.soft_sampler),
                },
            ],
        });

        self.cpu_pixels
            .resize((new_size.width * new_size.height) as usize, 0);
    }

    pub(crate) fn render_gpu(&mut self, state: &AppState) -> Result<(), wgpu::SurfaceError> {
        let temporal = state.temporal_color();
        let static_avg = state.static_average();
        let ca = state.color_a;
        let cb = state.color_b;

        // The colors are already 8-bit codes; the card only has to put them on screen.
        let srgb = self.surface_is_srgb;
        let on_surface = |c: [u8; 3]| {
            let v = |x: u8| color::code_value(x, srgb);
            [v(c[0]), v(c[1]), v(c[2]), 1.0]
        };

        let uniforms = Uniforms {
            mode: state.mode as u32,
            phase: state.frame_phase,
            cell_size: state.cell_size,
            _pad: 0,
            color_a: on_surface(ca),
            color_b: on_surface(cb),
            temporal: on_surface(temporal),
            static_avg: on_surface(static_avg),
            resolution: [self.size.width as f32, self.size.height as f32],
            _pad2: [0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.gpu_pipeline);
            pass.set_bind_group(0, &self.gpu_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }

    pub(crate) fn render_software(&mut self, state: &AppState) -> Result<(), wgpu::SurfaceError> {
        let w = self.size.width;
        let h = self.size.height;
        let temporal = color::pack_bgra(state.temporal_color());
        let static_avg = color::pack_bgra(state.static_average());
        let ca = color::pack_bgra(state.color_a);
        let cb = color::pack_bgra(state.color_b);

        match state.mode {
            DisplayMode::SideBySide => {
                fill_side_by_side(&mut self.cpu_pixels, w, h, temporal, static_avg);
            }
            DisplayMode::FullTemporal => {
                fill_solid(&mut self.cpu_pixels, temporal);
            }
            DisplayMode::Checkerboard => {
                fill_checkerboard(
                    &mut self.cpu_pixels,
                    w,
                    h,
                    ca,
                    cb,
                    state.cell_size,
                    state.frame_phase,
                );
            }
            DisplayMode::Gradient => {
                fill_gradient(
                    &mut self.cpu_pixels,
                    w,
                    h,
                    state.color_a,
                    state.color_b,
                    state.frame_phase,
                    state.gradient_dither,
                );
            }
        }

        // Upload CPU buffer → GPU texture
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.soft_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.cpu_pixels),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.soft_pipeline);
            pass.set_bind_group(0, &self.soft_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }
}
