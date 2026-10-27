use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;

use anyhow::Result;
use wgpu::util::DeviceExt as _;

use super::engine::WgpuEngine;
use super::math::{self, MESH_VERTEX_STRIDE};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum DemoId {
    Particles,
    Mesh,
    MeshPost,
}

impl DemoId {
    pub(in crate::features::wgpu) fn from_ix(ix: usize) -> Self {
        match ix {
            0 => DemoId::Particles,
            1 => DemoId::Mesh,
            _ => DemoId::MeshPost,
        }
    }
}

struct DemoSpec {
    uses_particles: bool,
    uses_mesh: bool,
    post: bool,
}

static SPECS: [(DemoId, DemoSpec); 3] = [
    (
        DemoId::Particles,
        DemoSpec {
            uses_particles: true,
            uses_mesh: false,
            post: false,
        },
    ),
    (
        DemoId::Mesh,
        DemoSpec {
            uses_particles: false,
            uses_mesh: true,
            post: false,
        },
    ),
    (
        DemoId::MeshPost,
        DemoSpec {
            uses_particles: false,
            uses_mesh: true,
            post: true,
        },
    ),
];

fn spec_for(demo: DemoId) -> &'static DemoSpec {
    let (_, spec) = SPECS
        .iter()
        .find(|(id, _)| *id == demo)
        .expect("SPECS covers every DemoId");
    spec
}

const ALL_STAGES: wgpu::ShaderStages =
    wgpu::ShaderStages::VERTEX_FRAGMENT.union(wgpu::ShaderStages::COMPUTE);

const FRAME_PARAMS_WGSL: &str = r#"
struct FrameParams {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    light_dir: vec4<f32>,
    misc: vec4<f32>,
};
@group(0) @binding(0) var<uniform> params: FrameParams;
"#;

const PARTICLES_COMPUTE_WGSL: &str = r#"
struct Particle { pos: vec4<f32>, vel: vec4<f32> };
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(2) var<uniform> attractor: vec4<f32>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&particles)) { return; }
    var p = particles[i];
    let dt = params.misc.x;
    let to_a = attractor.xyz - p.pos.xyz;
    let dist = max(length(to_a), 0.05);
    let pull = normalize(to_a) * (attractor.w / (dist * dist));
    let vel = p.vel.xyz + (pull - p.vel.xyz * 0.35) * dt;
    p.vel = vec4<f32>(vel, p.vel.w);
    let pos = p.pos.xyz + vel * dt;
    p.pos = vec4<f32>(pos, p.pos.w);
    let r = length(pos);
    if (r > 1.8) {
        let n = pos / r;
        p.pos = vec4<f32>(n * 1.8, p.pos.w);
        let bounced = reflect(vel, n) * 0.9;
        p.vel = vec4<f32>(bounced, p.vel.w);
    }
    particles[i] = p;
}
"#;

const PARTICLES_DRAW_WGSL: &str = r#"
struct Particle { pos: vec4<f32>, vel: vec4<f32> };
@group(0) @binding(1) var<storage, read> particles_draw: array<Particle>;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) speed: f32,
};

@vertex
fn vs_main(@builtin(instance_index) ii: u32,
           @builtin(vertex_index) vi: u32) -> VOut {
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    let p = particles_draw[ii];
    let clip = params.mvp * vec4<f32>(p.pos.xyz, 1.0);
    let ndc = clip.xyz / clip.w;
    let size = p.pos.w * params.misc.z / max(ndc.z, 0.1);
    var out: VOut;
    let offset = corners[vi] * size;
    out.pos = vec4<f32>((ndc.xy + offset) * clip.w, clip.z, clip.w);
    out.speed = length(p.vel.xyz);
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let s = clamp(in.speed * 0.8, 0.0, 1.0);
    let col = mix(vec3<f32>(0.15, 0.45, 0.95), vec3<f32>(0.85, 0.95, 1.0), s);
    return vec4<f32>(col, 0.9);
}
"#;

const MESH_WGSL: &str = r#"
struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};
@vertex
fn vs_main(in: VIn) -> VOut {
    var out: VOut;
    out.pos = params.mvp * vec4<f32>(in.pos, 1.0);
    out.normal = (params.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.uv = in.uv;
    return out;
}
@group(0) @binding(1) var mesh_tex: texture_2d<f32>;
@group(0) @binding(2) var mesh_sampler: sampler;
@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let lambert = max(dot(n, normalize(params.light_dir.xyz)), 0.0);
    let albedo = textureSample(mesh_tex, mesh_sampler, in.uv).rgb;
    let col = albedo * (0.25 + 0.75 * lambert);
    return vec4<f32>(col, 1.0);
}
"#;

const OUTPUT_VS_WGSL: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(pos[vi], 0.0, 1.0);
}
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var scene_sampler: sampler;
"#;

const OUTPUT_IDENTITY_WGSL: &str = r#"
@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag.xy / vec2<f32>(textureDimensions(scene_tex, 0));
    let c = textureSample(scene_tex, scene_sampler, uv);
    // BGRA byte order into the Rgba8Unorm target: RenderImage wants BGRA.
    return vec4<f32>(c.b, c.g, c.r, c.a);
}
"#;

const OUTPUT_POST_WGSL: &str = r#"
@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(scene_tex, 0));
    let uv = frag.xy / dims;
    let d = distance(uv, vec2<f32>(0.5));
    let pulse = 0.5 + 0.5 * sin(params.misc.y * 1.5);
    let amt = (0.002 + 0.004 * pulse) * d;
    let dir = normalize(uv - vec2<f32>(0.5) + vec2<f32>(1e-5));
    let r = textureSample(scene_tex, scene_sampler, uv + dir * amt).r;
    let g = textureSample(scene_tex, scene_sampler, uv).g;
    let b = textureSample(scene_tex, scene_sampler, uv - dir * amt).b;
    let vign = smoothstep(0.95, 0.35, d);
    let c = vec3<f32>(r, g, b) * mix(0.55, 1.0, vign);
    return vec4<f32>(c.b, c.g, c.r, 1.0);
}
"#;

#[cfg(test)]
pub(in crate::features::wgpu) fn wgsl_modules() -> [(&'static str, String); 5] {
    [
        (
            "particles-compute",
            [FRAME_PARAMS_WGSL, PARTICLES_COMPUTE_WGSL].concat(),
        ),
        (
            "particles-draw",
            [FRAME_PARAMS_WGSL, PARTICLES_DRAW_WGSL].concat(),
        ),
        ("mesh", [FRAME_PARAMS_WGSL, MESH_WGSL].concat()),
        (
            "output-identity",
            [FRAME_PARAMS_WGSL, OUTPUT_VS_WGSL, OUTPUT_IDENTITY_WGSL].concat(),
        ),
        (
            "output-post",
            [FRAME_PARAMS_WGSL, OUTPUT_VS_WGSL, OUTPUT_POST_WGSL].concat(),
        ),
    ]
}

#[derive(Copy, Clone, Debug)]
pub(in crate::features::wgpu) struct RenderJob {
    pub(in crate::features::wgpu) demo: DemoId,
    pub(in crate::features::wgpu) particle_count: u32,
    pub(in crate::features::wgpu) dt: f32,
    pub(in crate::features::wgpu) elapsed: f32,
    pub(in crate::features::wgpu) width: u32,
    pub(in crate::features::wgpu) height: u32,
}

struct Targets {
    width: u32,
    height: u32,
    needs_depth: bool,
    scene_view: wgpu::TextureView,
    depth_view: Option<wgpu::TextureView>,
    out: wgpu::Texture,
    out_view: wgpu::TextureView,
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn storage_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn scene_entries<'a>(
    frame_params: &'a wgpu::Buffer,
    scene_view: &'a wgpu::TextureView,
    scene_sampler: &'a wgpu::Sampler,
) -> Vec<wgpu::BindGroupEntry<'a>> {
    vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer_resource(frame_params),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::TextureView(scene_view),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(scene_sampler),
        },
    ]
}

fn color_attachment<'a>(
    view: &'a wgpu::TextureView,
    clear: wgpu::Color,
) -> Option<wgpu::RenderPassColorAttachment<'a>> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(clear),
            store: wgpu::StoreOp::Store,
        },
    })
}

fn buffer_resource(buffer: &wgpu::Buffer) -> wgpu::BindingResource<'_> {
    wgpu::BindingResource::Buffer(wgpu::BufferBinding {
        buffer,
        offset: 0,
        size: None,
    })
}

/// Owns every wgpu pipeline and buffer for the three demos. Reached only
/// from background tasks; mutation is serialized by the page's one-frame-
/// in-flight rule through the enclosing `Mutex`.
pub(in crate::features::wgpu) struct SceneRenderer {
    engine: Arc<WgpuEngine>,
    frame_params: wgpu::Buffer,
    attractor: wgpu::Buffer,
    particles_compute_pipeline: wgpu::ComputePipeline,
    particles_compute_group: wgpu::BindGroup,
    particles_draw_pipeline: wgpu::RenderPipeline,
    particles_draw_group: wgpu::BindGroup,
    particles: wgpu::Buffer,
    particle_count: u32,
    mesh_pipeline: wgpu::RenderPipeline,
    mesh_group: wgpu::BindGroup,
    mesh_vertices: wgpu::Buffer,
    mesh_indices: wgpu::Buffer,
    index_count: u32,
    output_identity_pipeline: wgpu::RenderPipeline,
    output_post_pipeline: wgpu::RenderPipeline,
    output_identity_group: wgpu::BindGroup,
    output_post_group: wgpu::BindGroup,
    scene_sampler: wgpu::Sampler,
    compute_layout: wgpu::BindGroupLayout,
    draw_layout: wgpu::BindGroupLayout,
    output_layout: wgpu::BindGroupLayout,
    targets: Targets,
}

impl SceneRenderer {
    pub(in crate::features::wgpu) fn new(
        engine: Arc<WgpuEngine>,
        initial: &RenderJob,
    ) -> Result<Self> {
        let device = engine.device();
        let module = |label: &str, source: String| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        };
        let layout = |label: &str, entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries,
            })
        };
        let pipeline_layout = |label: &str, bind_group_layout: &wgpu::BindGroupLayout| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(bind_group_layout)],
                immediate_size: 0,
            })
        };

        let frame_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wgpu-frame-params"),
            size: 160,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let attractor = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wgpu-attractor"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_layout = layout(
            "wgpu-particles-compute-layout",
            &[
                uniform_entry(0, ALL_STAGES),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(2, wgpu::ShaderStages::COMPUTE),
            ],
        );
        let compute_module = module(
            "wgpu-particles-compute",
            [FRAME_PARAMS_WGSL, PARTICLES_COMPUTE_WGSL].concat(),
        );
        let particles_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("wgpu-particles-compute-pipeline"),
                layout: Some(&pipeline_layout(
                    "wgpu-particles-compute-layout-object",
                    &compute_layout,
                )),
                module: &compute_module,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        let draw_layout = layout(
            "wgpu-particles-draw-layout",
            &[
                uniform_entry(0, ALL_STAGES),
                storage_entry(1, wgpu::ShaderStages::VERTEX, true),
            ],
        );
        let draw_module = module(
            "wgpu-particles-draw",
            [FRAME_PARAMS_WGSL, PARTICLES_DRAW_WGSL].concat(),
        );
        let particles_draw_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("wgpu-particles-draw-pipeline"),
                layout: Some(&pipeline_layout(
                    "wgpu-particles-draw-layout-object",
                    &draw_layout,
                )),
                vertex: wgpu::VertexState {
                    module: &draw_module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &draw_module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        let mesh_layout = layout(
            "wgpu-mesh-layout",
            &[
                uniform_entry(0, ALL_STAGES),
                texture_entry(1),
                sampler_entry(2),
            ],
        );
        let mesh_module = module("wgpu-mesh", [FRAME_PARAMS_WGSL, MESH_WGSL].concat());
        let (mesh_vertex_bytes, mesh_indices) = math::sphere_mesh(24, 36);
        let index_count = mesh_indices.len() as u32;
        let mesh_index_bytes: Vec<u8> = mesh_indices
            .iter()
            .flat_map(|index| index.to_ne_bytes())
            .collect();
        let mesh_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wgpu-mesh-vertices"),
            contents: &mesh_vertex_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mesh_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wgpu-mesh-indices"),
            contents: &mesh_index_bytes,
            usage: wgpu::BufferUsages::INDEX,
        });
        let linear_sampler = |label: &str| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            })
        };
        let mesh_sampler = linear_sampler("wgpu-mesh-sampler");
        let mesh_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wgpu-mesh-texture"),
            size: wgpu::Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        engine.queue().write_texture(
            mesh_texture.as_image_copy(),
            &math::checker_texture(256),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256 * 4),
                rows_per_image: Some(256),
            },
            wgpu::Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: 1,
            },
        );
        let mesh_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-mesh-group"),
            layout: &mesh_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_resource(&frame_params),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &mesh_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&mesh_sampler),
                },
            ],
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wgpu-mesh-pipeline"),
            layout: Some(&pipeline_layout("wgpu-mesh-layout-object", &mesh_layout)),
            vertex: wgpu::VertexState {
                module: &mesh_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: MESH_VERTEX_STRIDE as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 24,
                            shader_location: 2,
                        },
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &mesh_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let output_layout = layout(
            "wgpu-output-layout",
            &[
                uniform_entry(0, ALL_STAGES),
                texture_entry(1),
                sampler_entry(2),
            ],
        );
        let output_pipeline_layout = pipeline_layout("wgpu-output-layout-object", &output_layout);
        let identity_module = module(
            "wgpu-output-identity",
            [FRAME_PARAMS_WGSL, OUTPUT_VS_WGSL, OUTPUT_IDENTITY_WGSL].concat(),
        );
        let output_identity_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("wgpu-output-identity-pipeline"),
                layout: Some(&output_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &identity_module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &identity_module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
        let post_module = module(
            "wgpu-output-post",
            [FRAME_PARAMS_WGSL, OUTPUT_VS_WGSL, OUTPUT_POST_WGSL].concat(),
        );
        let output_post_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wgpu-output-post-pipeline"),
            layout: Some(&output_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &post_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &post_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let scene_sampler = linear_sampler("wgpu-scene-sampler");

        let particles = particle_buffers(&engine, initial.particle_count);
        let particles_compute_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-particles-compute-group"),
            layout: &compute_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_resource(&frame_params),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_resource(&particles),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_resource(&attractor),
                },
            ],
        });
        let particles_draw_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-particles-draw-group"),
            layout: &draw_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_resource(&frame_params),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_resource(&particles),
                },
            ],
        });

        let targets = create_targets(
            device,
            initial.width,
            initial.height,
            spec_for(initial.demo).uses_mesh,
        );
        let (output_identity_group, output_post_group) = {
            let identity_entries =
                scene_entries(&frame_params, &targets.scene_view, &scene_sampler);
            let post_entries = scene_entries(&frame_params, &targets.scene_view, &scene_sampler);
            (
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("wgpu-output-identity-group"),
                    layout: &output_layout,
                    entries: &identity_entries,
                }),
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("wgpu-output-post-group"),
                    layout: &output_layout,
                    entries: &post_entries,
                }),
            )
        };

        Ok(Self {
            engine,
            frame_params,
            attractor,
            particles_compute_pipeline,
            particles_compute_group,
            particles_draw_pipeline,
            particles_draw_group,
            particles,
            particle_count: initial.particle_count,
            mesh_pipeline,
            mesh_group,
            mesh_vertices,
            mesh_indices,
            index_count,
            output_identity_pipeline,
            output_post_pipeline,
            output_identity_group,
            output_post_group,
            scene_sampler,
            compute_layout,
            draw_layout,
            output_layout,
            targets,
        })
    }

    pub(in crate::features::wgpu) fn set_particle_count(&mut self, count: u32) {
        if count == self.particle_count {
            return;
        }
        let device = self.engine.device();
        self.particles = particle_buffers(&self.engine, count);
        self.particle_count = count;
        self.particles_compute_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-particles-compute-group"),
            layout: &self.compute_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_resource(&self.frame_params),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_resource(&self.particles),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_resource(&self.attractor),
                },
            ],
        });
        self.particles_draw_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-particles-draw-group"),
            layout: &self.draw_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_resource(&self.frame_params),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_resource(&self.particles),
                },
            ],
        });
    }

    pub(in crate::features::wgpu) fn reconfigure(
        &mut self,
        job_width: u32,
        job_height: u32,
        needs_depth: bool,
    ) {
        if self.targets.width == job_width
            && self.targets.height == job_height
            && self.targets.needs_depth == needs_depth
        {
            return;
        }
        self.targets = create_targets(self.engine.device(), job_width, job_height, needs_depth);
        let device = self.engine.device();
        let identity_entries = scene_entries(
            &self.frame_params,
            &self.targets.scene_view,
            &self.scene_sampler,
        );
        self.output_identity_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-output-identity-group"),
            layout: &self.output_layout,
            entries: &identity_entries,
        });
        let post_entries = scene_entries(
            &self.frame_params,
            &self.targets.scene_view,
            &self.scene_sampler,
        );
        self.output_post_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-output-post-group"),
            layout: &self.output_layout,
            entries: &post_entries,
        });
    }

    /// Renders one frame and reads it back as tightly packed BGRA bytes.
    /// BLOCKING: call from a background thread only.
    pub(in crate::features::wgpu) fn render(&mut self, job: RenderJob) -> Result<Vec<u8>> {
        let spec = spec_for(job.demo);
        self.reconfigure(job.width, job.height, spec.uses_mesh);
        if spec.uses_particles {
            self.set_particle_count(job.particle_count);
        }

        let aspect = job.width as f32 / job.height as f32;
        let view = math::mat4_translate(0.0, 0.0, -3.0);
        let model = if spec.uses_mesh {
            math::mat4_mul(
                &math::mat4_rotate_y(job.elapsed * 0.5),
                &math::mat4_rotate_x(0.5),
            )
        } else {
            math::mat4_identity()
        };
        let projection = math::perspective(FRAC_PI_4, aspect, 0.1, 100.0);
        let mvp = math::mat4_mul(&projection, &math::mat4_mul(&view, &model));
        let light_dir = [0.6f32, 1.0, 0.4, 0.0];
        let misc = [job.dt, job.elapsed, 0.02, 0.0];
        let attractor = [
            (job.elapsed * 0.7).sin() * 0.5,
            (job.elapsed * 1.1).sin() * 0.3,
            (job.elapsed * 0.7).cos() * 0.5,
            2.2,
        ];
        let mut params_bytes = [0u8; 160];
        params_bytes[0..64].copy_from_slice(&math::encode_f32_rows(&mvp));
        params_bytes[64..128].copy_from_slice(&math::encode_f32_rows(&model));
        params_bytes[128..144].copy_from_slice(&math::encode_f32_rows(&light_dir));
        params_bytes[144..160].copy_from_slice(&math::encode_f32_rows(&misc));
        self.engine
            .queue()
            .write_buffer(&self.frame_params, 0, &params_bytes);
        self.engine
            .queue()
            .write_buffer(&self.attractor, 0, &math::encode_f32_rows(&attractor));

        let mut encoder =
            self.engine
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("wgpu-scene-encode"),
                });
        let clear = wgpu::Color {
            r: 0.02,
            g: 0.02,
            b: 0.04,
            a: 1.0,
        };

        match job.demo {
            DemoId::Particles => {
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("wgpu-particles-compute"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.particles_compute_pipeline);
                    pass.set_bind_group(0, &self.particles_compute_group, &[]);
                    pass.dispatch_workgroups(self.particle_count.div_ceil(64), 1, 1);
                }
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("wgpu-particles-scene"),
                    color_attachments: &[color_attachment(&self.targets.scene_view, clear)],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.particles_draw_pipeline);
                pass.set_bind_group(0, &self.particles_draw_group, &[]);
                pass.draw(0..4, 0..self.particle_count);
            }
            DemoId::Mesh | DemoId::MeshPost => {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("wgpu-mesh-scene"),
                    color_attachments: &[color_attachment(&self.targets.scene_view, clear)],
                    depth_stencil_attachment: self.targets.depth_view.as_ref().map(|view| {
                        wgpu::RenderPassDepthStencilAttachment {
                            view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_bind_group(0, &self.mesh_group, &[]);
                pass.set_vertex_buffer(0, self.mesh_vertices.slice(..));
                pass.set_index_buffer(self.mesh_indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.index_count, 0, 0..1);
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wgpu-output"),
                color_attachments: &[color_attachment(&self.targets.out_view, clear)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if spec.post {
                pass.set_pipeline(&self.output_post_pipeline);
                pass.set_bind_group(0, &self.output_post_group, &[]);
            } else {
                pass.set_pipeline(&self.output_identity_pipeline);
                pass.set_bind_group(0, &self.output_identity_group, &[]);
            }
            pass.draw(0..3, 0..1);
        }

        self.engine.queue().submit(Some(encoder.finish()));
        self.engine
            .read_texture_bytes(&self.targets.out, job.width, job.height)
    }
}

fn particle_buffers(engine: &WgpuEngine, count: u32) -> wgpu::Buffer {
    let (positions, velocities) = math::particle_init(count);
    let mut interleaved = Vec::with_capacity(count as usize * 32);
    for (pos, vel) in positions
        .as_chunks::<16>()
        .0
        .iter()
        .zip(velocities.as_chunks::<16>().0)
    {
        interleaved.extend_from_slice(pos);
        interleaved.extend_from_slice(vel);
    }
    engine
        .device()
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wgpu-particles"),
            contents: &interleaved,
            usage: wgpu::BufferUsages::STORAGE,
        })
}

fn create_targets(device: &wgpu::Device, width: u32, height: u32, needs_depth: bool) -> Targets {
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("wgpu-scene"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = needs_depth.then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wgpu-depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    });
    let depth_view = depth
        .as_ref()
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
    let out = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("wgpu-out"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let out_view = out.create_view(&wgpu::TextureViewDescriptor::default());
    Targets {
        width,
        height,
        needs_depth,
        scene_view,
        depth_view,
        out,
        out_view,
    }
}

#[cfg(test)]
#[path = "demos.test.rs"]
mod demos_test;
