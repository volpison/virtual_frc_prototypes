//! The two WGSL shaders: one draws the patterns itself, one shows a texture filled by the CPU.

pub(super) const GPU_SHADER: &str = r#"
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

pub(super) const SOFT_SHADER: &str = r#"
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
