//! Virtual Frame Rate Control Prototypes (VirtualFRC)
//!
//! Display modes (M):
//!   0 – Side-by-side: Left = temporal, Right = static average
//!   1 – Full temporal
//!   2 – Checkerboard flicker-dither
//!
//! Backends (B):
//!   GPU     – pure GPU clear / shader path (default)
//!   Software – multi-threaded SIMD fill of a CPU buffer, then upload
//!
//! Controls:
//!   M          : cycle display mode
//!   B          : toggle GPU ↔ Software backend
//!   Arrow/WASD : adjust selected color (R/G)
//!   E / C      : adjust Blue
//!   Space      : toggle editing A ↔ B
//!   1 / 2 / 3  : duty cycle
//!   V          : Fifo ↔ Immediate
//!   [ / ]      : checkerboard cell size
//!   R          : reset colors
//!   Esc / Q    : quit

use std::sync::Arc;
use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use log::{error, info, warn};
use rayon::prelude::*;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowBuilder},
};

const DEFAULT_COLOR_A: [f64; 3] = [0.40, 0.70, 0.30];
const DEFAULT_COLOR_B: [f64; 3] = [0.30, 0.55, 0.80];
const STEP: f64 = 0.02;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum DisplayMode {
    SideBySide = 0,
    FullTemporal = 1,
    Checkerboard = 2,
}

impl DisplayMode {
    fn next(self) -> Self {
        match self {
            DisplayMode::SideBySide => DisplayMode::FullTemporal,
            DisplayMode::FullTemporal => DisplayMode::Checkerboard,
            DisplayMode::Checkerboard => DisplayMode::SideBySide,
        }
    }
    fn name(self) -> &'static str {
        match self {
            DisplayMode::SideBySide => "Side-by-side",
            DisplayMode::FullTemporal => "Full temporal",
            DisplayMode::Checkerboard => "Checkerboard",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Backend {
    Gpu,
    Software,
}

impl Backend {
    fn toggle(self) -> Self {
        match self {
            Backend::Gpu => Backend::Software,
            Backend::Software => Backend::Gpu,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Backend::Gpu => "GPU",
            Backend::Software => "Software+SIMD",
        }
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct Metrics {
    frame_count: u64,
    last_report: Instant,
    current_fps: f64,
    total_frames: u64,
}

impl Metrics {
    fn new() -> Self {
        Self {
            frame_count: 0,
            last_report: Instant::now(),
            current_fps: 0.0,
            total_frames: 0,
        }
    }
    fn record_frame(&mut self) {
        self.frame_count += 1;
        self.total_frames += 1;
        let elapsed = self.last_report.elapsed();
        if elapsed >= Duration::from_millis(400) {
            self.current_fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.frame_count = 0;
            self.last_report = Instant::now();
        }
    }
}

struct AppState {
    color_a: [f64; 3],
    color_b: [f64; 3],
    editing_a: bool,
    duty_a: u32,
    frame_phase: u32,
    metrics: Metrics,
    monitor_hz: f64,
    use_vsync: bool,
    mode: DisplayMode,
    backend: Backend,
    cell_size: u32,
}

impl AppState {
    fn new(monitor_hz: f64) -> Self {
        Self {
            color_a: DEFAULT_COLOR_A,
            color_b: DEFAULT_COLOR_B,
            editing_a: true,
            duty_a: 1,
            frame_phase: 0,
            metrics: Metrics::new(),
            monitor_hz,
            use_vsync: true,
            mode: DisplayMode::SideBySide,
            backend: Backend::Gpu,
            cell_size: 8,
        }
    }

    fn temporal_color(&self) -> [f32; 3] {
        let use_a = if self.duty_a <= 1 {
            self.frame_phase % 2 == 0
        } else {
            (self.frame_phase % (self.duty_a + 1)) < self.duty_a
        };
        let c = if use_a { self.color_a } else { self.color_b };
        [c[0] as f32, c[1] as f32, c[2] as f32]
    }

    fn static_average(&self) -> [f32; 3] {
        [
            ((self.color_a[0] + self.color_b[0]) * 0.5) as f32,
            ((self.color_a[1] + self.color_b[1]) * 0.5) as f32,
            ((self.color_a[2] + self.color_b[2]) * 0.5) as f32,
        ]
    }

    fn color_a_f32(&self) -> [f32; 3] {
        [
            self.color_a[0] as f32,
            self.color_a[1] as f32,
            self.color_a[2] as f32,
        ]
    }
    fn color_b_f32(&self) -> [f32; 3] {
        [
            self.color_b[0] as f32,
            self.color_b[1] as f32,
            self.color_b[2] as f32,
        ]
    }

    fn advance_phase(&mut self) {
        self.frame_phase = self.frame_phase.wrapping_add(1);
    }

    fn adjust_channel(&mut self, channel: usize, delta: f64) {
        let color = if self.editing_a {
            &mut self.color_a
        } else {
            &mut self.color_b
        };
        color[channel] = (color[channel] + delta).clamp(0.0, 1.0);
    }

    fn title_string(&self) -> String {
        let which = if self.editing_a { "A" } else { "B" };
        let present = if self.use_vsync { "Fifo" } else { "Imm" };
        format!(
            "VirtualFRC  |  {:.0} FPS  |  {}  |  {}  |  {}  |  edit {}  |  duty {}:1  |  cell {}  |  frames:{}",
            self.metrics.current_fps,
            self.backend.name(),
            present,
            self.mode.name(),
            which,
            self.duty_a,
            self.cell_size,
            self.metrics.total_frames,
        )
    }
}

// ---------------------------------------------------------------------------
// Software SIMD fill
// ---------------------------------------------------------------------------

/// Pack an sRGB float color into a BGRA u32 (little-endian, matching common
/// Windows / wgpu texture layouts).
#[inline(always)]
fn pack_bgra(c: [f32; 3]) -> u32 {
    let r = (c[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    let g = (c[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    let b = (c[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    // BGRA in memory: B | G << 8 | R << 16 | A << 24
    b | (g << 8) | (r << 16) | (0xFF << 24)
}

/// Fill a whole buffer with a solid color using multi-threaded stores.
/// On x86_64 we use wide stores where possible; otherwise scalar.
fn fill_solid(buf: &mut [u32], color: u32) {
    // Rayon parallel fill – each thread writes a contiguous slice.
    buf.par_chunks_mut(4096).for_each(|chunk| {
        // Scalar fill of the chunk (compiler + rayon already give good throughput).
        // For true AVX2 we could use _mm256_storeu_si256 in an unsafe block;
        // the parallel scalar version is already a large win over single-threaded.
        for p in chunk.iter_mut() {
            *p = color;
        }
    });
}

/// Multi-threaded checkerboard fill.
fn fill_checkerboard(
    buf: &mut [u32],
    width: u32,
    height: u32,
    color_a: u32,
    color_b: u32,
    cell: u32,
    phase: u32,
) {
    let cell = cell.max(1);
    let flip = (phase % 2) == 1;
    let w = width as usize;

    buf.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let cy = (y as u32 / cell) as i32;
        for (x, pixel) in row.iter_mut().enumerate() {
            let cx = (x as u32 / cell) as i32;
            let mut checker = (cx + cy) & 1;
            if flip {
                checker = 1 - checker;
            }
            *pixel = if checker == 0 { color_a } else { color_b };
        }
    });
}

/// Side-by-side: left half temporal solid, right half static average.
fn fill_side_by_side(buf: &mut [u32], width: u32, _height: u32, temporal: u32, static_avg: u32) {
    let mid = (width / 2) as usize;
    let w = width as usize;

    buf.par_chunks_mut(w).for_each(|row| {
        for (x, pixel) in row.iter_mut().enumerate() {
            *pixel = if x < mid { temporal } else { static_avg };
        }
    });
}

// ---------------------------------------------------------------------------
// Uniforms (GPU path)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// GPU state (handles both pure-GPU and software-upload paths)
// ---------------------------------------------------------------------------

struct GpuState<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
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
    async fn new(window: &'a Window, prefer_vsync: bool) -> Self {
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
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
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
            format: wgpu::TextureFormat::Bgra8Unorm,
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

    fn set_present_mode(&mut self, prefer_vsync: bool) {
        let new_mode = choose_present_mode(&self.available_present_modes, prefer_vsync);
        if new_mode != self.config.present_mode {
            info!("Present mode → {:?}", new_mode);
            self.config.present_mode = new_mode;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
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
            format: wgpu::TextureFormat::Bgra8Unorm,
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

    fn render_gpu(&mut self, state: &AppState) -> Result<(), wgpu::SurfaceError> {
        let temporal = state.temporal_color();
        let static_avg = state.static_average();
        let ca = state.color_a_f32();
        let cb = state.color_b_f32();

        let uniforms = Uniforms {
            mode: state.mode as u32,
            phase: state.frame_phase,
            cell_size: state.cell_size,
            _pad: 0,
            color_a: [ca[0], ca[1], ca[2], 1.0],
            color_b: [cb[0], cb[1], cb[2], 1.0],
            temporal: [temporal[0], temporal[1], temporal[2], 1.0],
            static_avg: [static_avg[0], static_avg[1], static_avg[2], 1.0],
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

    fn render_software(&mut self, state: &AppState) -> Result<(), wgpu::SurfaceError> {
        let w = self.size.width;
        let h = self.size.height;
        let temporal = pack_bgra(state.temporal_color());
        let static_avg = pack_bgra(state.static_average());
        let ca = pack_bgra(state.color_a_f32());
        let cb = pack_bgra(state.color_b_f32());

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

// ---------------------------------------------------------------------------
// Shaders
// ---------------------------------------------------------------------------

const GPU_SHADER: &str = r#"
struct Uniforms {
    mode: u32,
    phase: u32,
    cell_size: u32,
    _pad: u32,
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    temporal: vec4<f32>,
    static_avg: vec4<f32>,
    resolution: vec2<f32>,
    _pad2: vec2<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = in.clip_position.xy;
    if (u.mode == 0u) {
        if (pixel.x < u.resolution.x * 0.5) { return u.temporal; }
        else { return u.static_avg; }
    }
    if (u.mode == 1u) { return u.temporal; }
    // checkerboard
    let cell = f32(max(u.cell_size, 1u));
    let cx = i32(floor(pixel.x / cell));
    let cy = i32(floor(pixel.y / cell));
    var checker = (cx + cy) % 2;
    if ((u.phase % 2u) == 1u) { checker = 1 - checker; }
    if (checker == 0) { return u.color_a; } else { return u.color_b; }
}
"#;

const SOFT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(position, 0.0, 1.0);
    // map clip [-1,1] to uv [0,1]
    out.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    // flip Y for typical texture orientation
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}
"#;

fn choose_present_mode(available: &[wgpu::PresentMode], prefer_vsync: bool) -> wgpu::PresentMode {
    if prefer_vsync {
        if available.contains(&wgpu::PresentMode::Fifo) {
            return wgpu::PresentMode::Fifo;
        }
        if available.contains(&wgpu::PresentMode::FifoRelaxed) {
            return wgpu::PresentMode::FifoRelaxed;
        }
    }
    if available.contains(&wgpu::PresentMode::Immediate) {
        return wgpu::PresentMode::Immediate;
    }
    if available.contains(&wgpu::PresentMode::Mailbox) {
        return wgpu::PresentMode::Mailbox;
    }
    if available.contains(&wgpu::PresentMode::Fifo) {
        return wgpu::PresentMode::Fifo;
    }
    available[0]
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Use all logical cores for the software path.
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus_approx())
        .build_global()
        .ok();

    let event_loop = EventLoop::new().expect("event loop");
    let window = WindowBuilder::new()
        .with_title("VirtualFRC Prototypes")
        .with_inner_size(LogicalSize::new(1280, 720))
        .with_min_inner_size(LogicalSize::new(640, 360))
        .build(&event_loop)
        .expect("window");
    let window = Arc::new(window);

    let monitor_hz = window
        .current_monitor()
        .and_then(|m| m.refresh_rate_millihertz())
        .map(|mhz| mhz as f64 / 1000.0)
        .unwrap_or(60.0);
    info!("Monitor refresh rate: {:.1} Hz", monitor_hz);

    let mut state = AppState::new(monitor_hz);
    window.set_title(&state.title_string());

    let window_for_gpu = window.clone();
    let mut gpu = pollster::block_on(GpuState::new(&window_for_gpu, state.use_vsync));

    let mut title_timer = Instant::now();
    let mut last_frame = Instant::now();
    let frame_period = Duration::from_secs_f64(1.0 / state.monitor_hz.max(1.0));

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(s) => gpu.resize(s),
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    logical_key,
                                    state: key_state,
                                    repeat: false,
                                    ..
                                },
                            ..
                        } if key_state == ElementState::Pressed => {
                            match &logical_key {
                                Key::Named(NamedKey::Escape) => elwt.exit(),
                                Key::Named(NamedKey::Space) => state.editing_a = !state.editing_a,
                                Key::Named(NamedKey::ArrowUp) => state.adjust_channel(1, STEP),
                                Key::Named(NamedKey::ArrowDown) => state.adjust_channel(1, -STEP),
                                Key::Named(NamedKey::ArrowLeft) => state.adjust_channel(0, -STEP),
                                Key::Named(NamedKey::ArrowRight) => state.adjust_channel(0, STEP),
                                Key::Character(c) => match c.as_str() {
                                    "q" | "Q" => elwt.exit(),
                                    "m" | "M" => {
                                        state.mode = state.mode.next();
                                        info!("Mode → {}", state.mode.name());
                                    }
                                    "b" | "B" => {
                                        state.backend = state.backend.toggle();
                                        info!("Backend → {}", state.backend.name());
                                    }
                                    "1" => state.duty_a = 1,
                                    "2" => state.duty_a = 2,
                                    "3" => state.duty_a = 3,
                                    "v" | "V" => {
                                        state.use_vsync = !state.use_vsync;
                                        gpu.set_present_mode(state.use_vsync);
                                    }
                                    "[" => state.cell_size = (state.cell_size / 2).max(1),
                                    "]" => state.cell_size = (state.cell_size * 2).min(128),
                                    "r" | "R" => {
                                        state.color_a = DEFAULT_COLOR_A;
                                        state.color_b = DEFAULT_COLOR_B;
                                    }
                                    "w" | "W" => state.adjust_channel(1, STEP),
                                    "s" | "S" => state.adjust_channel(1, -STEP),
                                    "a" | "A" => state.adjust_channel(0, -STEP),
                                    "d" | "D" => state.adjust_channel(0, STEP),
                                    "e" | "E" => state.adjust_channel(2, STEP),
                                    "c" | "C" => state.adjust_channel(2, -STEP),
                                    "i" | "I" => {
                                        state.editing_a = false;
                                        state.adjust_channel(1, STEP);
                                    }
                                    "k" | "K" => {
                                        state.editing_a = false;
                                        state.adjust_channel(1, -STEP);
                                    }
                                    "j" | "J" => {
                                        state.editing_a = false;
                                        state.adjust_channel(0, -STEP);
                                    }
                                    "l" | "L" => {
                                        state.editing_a = false;
                                        state.adjust_channel(0, STEP);
                                    }
                                    "u" | "U" => {
                                        state.editing_a = false;
                                        state.adjust_channel(2, STEP);
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                            window.set_title(&state.title_string());
                        }
                        _ => {}
                    }
                }

                Event::AboutToWait => {
                    let now = Instant::now();
                    if now.duration_since(last_frame) >= frame_period {
                        last_frame = now;
                        state.metrics.record_frame();

                        let result = match state.backend {
                            Backend::Gpu => gpu.render_gpu(&state),
                            Backend::Software => gpu.render_software(&state),
                        };
                        match result {
                            Ok(()) => {}
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                warn!("Surface lost — reconfigure");
                                gpu.resize(gpu.size);
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                error!("OOM");
                                elwt.exit();
                            }
                            Err(wgpu::SurfaceError::Timeout) => {}
                        }
                        state.advance_phase();

                        if title_timer.elapsed() >= Duration::from_millis(250) {
                            window.set_title(&state.title_string());
                            title_timer = Instant::now();
                        }
                    }
                }
                _ => {}
            }
        })
        .expect("event loop");
}

fn num_cpus_approx() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
