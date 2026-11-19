// Transition pass: one fullscreen triangle, two scene textures, a progress value.
// The engine owns everything above the marker; effect shaders only replace fs_*.

struct Params {
    progress: f32, // 0.0 = all A (old frame), 1.0 = all B (new frame)
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var tex_a: texture_2d<f32>; // old frame
@group(0) @binding(1) var tex_b: texture_2d<f32>; // new frame
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var<uniform> params: Params;

@vertex
fn vs_fullscreen(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let p = corners[idx];
    var out: VertexOutput;
    out.clip = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}

// ===== Built-in effects =====

// fade: through black
@fragment
fn fs_fade(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = clamp(params.progress, 0.0, 1.0);
    let a = textureSampleLevel(tex_a, samp, in.uv, 0.0);
    let b = textureSampleLevel(tex_b, samp, in.uv, 0.0);
    let dark = 1.0 - abs(t * 2.0 - 1.0); // 0 -> 1 -> 0
    let mixed = select(a, b, t >= 0.5);
    return vec4<f32>(mixed.rgb * (1.0 - dark), 1.0);
}

// slide left: old frame moves out to the left, new frame pushes in from the right
@fragment
fn fs_slide_left(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = clamp(params.progress, 0.0, 1.0);
    let ua = in.uv.x + t;              // sample point moves right -> image moves left
    let ub = in.uv.x - (1.0 - t);      // new frame enters from the right
    let a = textureSampleLevel(tex_a, samp, vec2<f32>(ua, in.uv.y), 0.0);
    let b = textureSampleLevel(tex_b, samp, vec2<f32>(ub, in.uv.y), 0.0);
    let pick_a = t < 1.0 && ua < 1.0;
    return select(b, a, pick_a);
}

// slide right: old frame moves out to the right, new frame pushes in from the left
@fragment
fn fs_slide_right(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = clamp(params.progress, 0.0, 1.0);
    let ua = in.uv.x - t;              // sample point moves left -> image moves right
    let ub = in.uv.x + (1.0 - t);      // new frame enters from the left
    let a = textureSampleLevel(tex_a, samp, vec2<f32>(ua, in.uv.y), 0.0);
    let b = textureSampleLevel(tex_b, samp, vec2<f32>(ub, in.uv.y), 0.0);
    let pick_a = t < 1.0 && ua >= 0.0;
    return select(b, a, pick_a);
}
