// The native runtime is the only consumer of the encode/upload halves; wasm
// compiles the module without them.
#![cfg_attr(target_family = "wasm", allow(dead_code))]

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::features::pages::shaders) enum DemoId {
    Gradient,
    Raymarch,
    Warp,
    Ripple,
}

impl DemoId {
    pub(in crate::features::pages::shaders) const fn ix(self) -> usize {
        match self {
            Self::Gradient => 0,
            Self::Raymarch => 1,
            Self::Warp => 2,
            Self::Ripple => 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::features::pages::shaders) struct SliderSpec {
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::features::pages::shaders) struct DemoSpec {
    pub id: DemoId,
    pub wgsl: &'static str,
    pub width: u32,
    pub height: u32,
    pub slider: SliderSpec,
    pub uses_texture: bool,
}

pub(in crate::features::pages::shaders) const DEMOS: [DemoSpec; 4] = [
    DemoSpec {
        id: DemoId::Gradient,
        wgsl: GRADIENT_WGSL,
        width: 512,
        height: 512,
        slider: SliderSpec {
            min: 0.1,
            max: 2.0,
            default: 0.5,
        },
        uses_texture: false,
    },
    DemoSpec {
        id: DemoId::Raymarch,
        wgsl: RAYMARCH_WGSL,
        width: 384,
        height: 384,
        slider: SliderSpec {
            min: 0.05,
            max: 1.0,
            default: 0.25,
        },
        uses_texture: false,
    },
    DemoSpec {
        id: DemoId::Warp,
        wgsl: WARP_WGSL,
        width: 512,
        height: 512,
        slider: SliderSpec {
            min: 0.0,
            max: 1.0,
            default: 0.35,
        },
        uses_texture: true,
    },
    DemoSpec {
        id: DemoId::Ripple,
        wgsl: RIPPLE_WGSL,
        width: 512,
        height: 512,
        slider: SliderSpec {
            min: 0.0,
            max: 1.0,
            default: 0.5,
        },
        uses_texture: false,
    },
];

pub(in crate::features::pages::shaders) fn demo(id: DemoId) -> &'static DemoSpec {
    &DEMOS[id.ix()]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::features::pages::shaders) struct ShaderParams {
    pub time: f32,
    pub delta: f32,
    pub slider: f32,
    pub resolution: (f32, f32),
    pub mouse: (f32, f32),
}

impl ShaderParams {
    /// Little-endian 32-byte uniform matching the WGSL `Params` struct: the
    /// vec2 members sit at 8-byte-aligned offsets 16 and 24.
    pub(in crate::features::pages::shaders) fn to_bytes(self) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[0..4].copy_from_slice(&self.time.to_le_bytes());
        out[4..8].copy_from_slice(&self.delta.to_le_bytes());
        out[8..12].copy_from_slice(&self.slider.to_le_bytes());
        out[12..16].copy_from_slice(&0f32.to_le_bytes());
        out[16..20].copy_from_slice(&self.resolution.0.to_le_bytes());
        out[20..24].copy_from_slice(&self.resolution.1.to_le_bytes());
        out[24..28].copy_from_slice(&self.mouse.0.to_le_bytes());
        out[28..32].copy_from_slice(&self.mouse.1.to_le_bytes());
        out
    }
}

pub(in crate::features::pages::shaders) const WARP_SOURCE_SIZE: u32 = 512;

/// 512x512 RGBA source for the warp demo: a 32-px checker whose even tiles
/// carry a radial gradient and odd tiles a linear one.
pub(in crate::features::pages::shaders) fn warp_source_bytes() -> Vec<u8> {
    const TILE: u32 = 32;
    let mut out = Vec::with_capacity((WARP_SOURCE_SIZE * WARP_SOURCE_SIZE * 4) as usize);
    for y in 0..WARP_SOURCE_SIZE {
        for x in 0..WARP_SOURCE_SIZE {
            let (r, g, b) = if ((x / TILE) + (y / TILE)).is_multiple_of(2) {
                let cx = (x % TILE) as f32 / TILE as f32 - 0.5;
                let cy = (y % TILE) as f32 / TILE as f32 - 0.5;
                let radial = (cx * cx + cy * cy).sqrt() / std::f32::consts::SQRT_2;
                let c = (radial * 255.0) as u8;
                (255 - c, c, 96)
            } else {
                let fx = (x % TILE) as f32 / TILE as f32;
                let fy = (y % TILE) as f32 / TILE as f32;
                (24, (fx * 255.0) as u8, (fy * 255.0) as u8)
            };
            out.extend_from_slice(&[r, g, b, 255]);
        }
    }
    out
}

pub(in crate::features::pages::shaders) const GRADIENT_WGSL: &str = r#"
struct Params {
    time: f32,
    delta: f32,
    slider: f32,
    _pad0: f32,
    resolution: vec2<f32>,
    mouse: vec2<f32>,
};
@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0));
    return vec4<f32>(pos[vi], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag.xy / params.resolution;
    let t = params.time * params.slider;
    let v = 0.5 + 0.5 * vec2<f32>(
        sin(uv.x * 6.28318 + t),
        cos(uv.y * 6.28318 + t * 0.7));
    let w = 0.5 + 0.5 * sin((uv.x + uv.y) * 12.56636 + t * 1.3);
    var col = vec3<f32>(v.x * w, v.y * (1.0 - w), 0.5 + 0.5 * sin(t + uv.x * 3.14159));
    return vec4<f32>(col.b, col.g, col.r, 1.0);
}
"#;

pub(in crate::features::pages::shaders) const RAYMARCH_WGSL: &str = r#"
struct Params {
    time: f32,
    delta: f32,
    slider: f32,
    _pad0: f32,
    resolution: vec2<f32>,
    mouse: vec2<f32>,
};
@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0));
    return vec4<f32>(pos[vi], 0.0, 1.0);
}

fn sdf(p: vec3<f32>) -> f32 {
    let sphere = length(p - vec3<f32>(-0.6, 0.4, 0.0)) - 0.5;
    var q = abs(p - vec3<f32>(0.6, 0.35, 0.0)) - vec3<f32>(0.35, 0.35, 0.35);
    let box_d = length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
    let plane = p.y + 0.4;
    return min(min(sphere, box_d), plane);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (frag.xy / params.resolution) * 2.0 - vec2<f32>(1.0);
    let angle = params.time * params.slider;
    let cam = vec3<f32>(sin(angle) * 3.0, 1.2, cos(angle) * 3.0);
    let cam_target = vec3<f32>(0.0, 0.2, 0.0);
    let f = normalize(cam_target - cam);
    let r = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), f));
    let u = cross(f, r);
    let dir = normalize(f * 1.6 + r * uv.x + u * uv.y);
    var p = cam;
    var dist = 0.0;
    var hit = false;
    for (var i = 0; i < 64; i++) {
        let d = sdf(p);
        if (d < 0.001) {
            hit = true;
            break;
        }
        p = p + dir * d;
        dist = dist + d;
    }
    if (!hit) {
        return vec4<f32>(vec3<f32>(0.10, 0.12, 0.16), 1.0);
    }
    let n = normalize(vec3<f32>(
        sdf(p + vec3<f32>(0.001, 0.0, 0.0)) - sdf(p - vec3<f32>(0.001, 0.0, 0.0)),
        sdf(p + vec3<f32>(0.0, 0.001, 0.0)) - sdf(p - vec3<f32>(0.0, 0.001, 0.0)),
        sdf(p + vec3<f32>(0.0, 0.0, 0.001)) - sdf(p - vec3<f32>(0.0, 0.0, 0.001))));
    let light_dir = normalize(vec3<f32>(0.6, 1.0, 0.4));
    let diff = max(dot(n, light_dir), 0.0);
    let ao = 1.0 / (1.0 + dist * 0.35);
    var col = select(vec3<f32>(0.55, 0.55, 0.58), vec3<f32>(0.86, 0.30, 0.35), p.y > 0.0);
    col = col * (0.25 + 0.75 * diff) * ao;
    return vec4<f32>(col.b, col.g, col.r, 1.0);
}
"#;

pub(in crate::features::pages::shaders) const WARP_WGSL: &str = r#"
struct Params {
    time: f32,
    delta: f32,
    slider: f32,
    _pad0: f32,
    resolution: vec2<f32>,
    mouse: vec2<f32>,
};
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var src_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0));
    return vec4<f32>(pos[vi], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag.xy / params.resolution;
    let w = vec2<f32>(
        sin(uv.y * 10.0 + params.time * 1.5),
        cos(uv.x * 8.0 + params.time)) * 0.08 * params.slider;
    let uv2 = clamp(uv + w, vec2<f32>(0.0), vec2<f32>(1.0));
    var col = textureSample(src_tex, src_sampler, uv2).rgb;
    return vec4<f32>(col.b, col.g, col.r, 1.0);
}
"#;

pub(in crate::features::pages::shaders) const RIPPLE_WGSL: &str = r#"
struct Params {
    time: f32,
    delta: f32,
    slider: f32,
    _pad0: f32,
    resolution: vec2<f32>,
    mouse: vec2<f32>,
};
@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0));
    return vec4<f32>(pos[vi], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag.xy / params.resolution;
    let d = distance(uv, params.mouse);
    let freq = mix(20.0, 90.0, clamp(params.slider, 0.0, 1.0));
    let wave = sin(d * freq - params.time * 6.0);
    let atten = exp(-d * 3.0);
    let intensity = (0.5 + 0.5 * wave) * atten;
    var col = vec3<f32>(intensity, 0.15 + 0.55 * atten, 1.0 - intensity * 0.85);
    return vec4<f32>(col.b, col.g, col.r, 1.0);
}
"#;

#[cfg(test)]
#[path = "demos.test.rs"]
mod demos_test;
