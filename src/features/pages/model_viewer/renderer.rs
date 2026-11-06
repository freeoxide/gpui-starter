//! Native-only mesh renderer for the model viewer. One pipeline: Lambert
//! shading with a UV-checker albedo, rendered into an RGBA8 offscreen
//! texture (plus depth) and read back for `RenderImage`. The shader-canvas
//! page's runtime is the structural template (shaders/runtime.rs).

use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;

use anyhow::Result;
use wgpu::util::DeviceExt as _;

use crate::features::wgpu::{
    MESH_VERTEX_STRIDE, Mat4, WgpuEngine, encode_f32_rows, mat4_look_at, mat4_mul, mat4_rotate_y,
    perspective,
};

use super::obj::Mesh;
use super::state::RenderJob;

const FRAME_PARAMS_WGSL: &str = r#"
struct FrameParams {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    light_dir: vec4<f32>,
    misc: vec4<f32>,
};
@group(0) @binding(0) var<uniform> params: FrameParams;
"#;

const MODEL_WGSL: &str = r#"
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
@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let lambert = max(dot(n, normalize(params.light_dir.xyz)), 0.0);
    let cell = floor(in.uv * 8.0);
    let checker = ((cell.x + cell.y) % 2.0) < 1.0;
    let albedo = select(vec3<f32>(0.30, 0.50, 0.85), vec3<f32>(0.78, 0.80, 0.85), checker);
    return vec4<f32>(albedo * (0.25 + 0.75 * lambert), 1.0);
}
"#;

struct Targets {
    width: u32,
    height: u32,
    depth_view: wgpu::TextureView,
    out: wgpu::Texture,
    out_view: wgpu::TextureView,
}

fn mesh_buffers(engine: &WgpuEngine, mesh: &Mesh) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let index_bytes: Vec<u8> = mesh
        .indices
        .iter()
        .flat_map(|index| index.to_ne_bytes())
        .collect();
    let vertices = engine
        .device()
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model-vertices"),
            contents: &mesh.vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });
    let indices = engine
        .device()
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model-indices"),
            contents: &index_bytes,
            usage: wgpu::BufferUsages::INDEX,
        });
    (vertices, indices, mesh.indices.len() as u32)
}

/// Reached only from background tasks; mutation is serialized by the page's
/// one-frame-in-flight rule through the enclosing `Mutex`.
pub(in crate::features::pages::model_viewer) struct ModelRenderer {
    engine: Arc<WgpuEngine>,
    frame_params: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    mesh_pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    targets: Option<Targets>,
}

impl ModelRenderer {
    pub(in crate::features::pages::model_viewer) fn new(
        engine: Arc<WgpuEngine>,
        mesh: &Mesh,
    ) -> Result<Self> {
        let device = engine.device();
        let frame_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model-frame-params"),
            contents: &[0u8; 160],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model-group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &frame_params,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model-wgsl"),
            source: wgpu::ShaderSource::Wgsl([FRAME_PARAMS_WGSL, MODEL_WGSL].concat().into()),
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
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
                module: &module,
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

        let (vertices, indices, index_count) = mesh_buffers(&engine, mesh);
        Ok(Self {
            engine,
            frame_params,
            bind_group,
            mesh_pipeline,
            vertices,
            indices,
            index_count,
            targets: None,
        })
    }

    fn set_mesh(&mut self, mesh: &Mesh) {
        let (vertices, indices, index_count) = mesh_buffers(&self.engine, mesh);
        self.vertices = vertices;
        self.indices = indices;
        self.index_count = index_count;
    }

    fn reconfigure(&mut self, width: u32, height: u32) {
        if self
            .targets
            .as_ref()
            .is_some_and(|targets| targets.width == width && targets.height == height)
        {
            return;
        }
        let device = self.engine.device();
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("model-depth"),
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
        });
        let out = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("model-out"),
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
        self.targets = Some(Targets {
            width,
            height,
            depth_view: depth.create_view(&wgpu::TextureViewDescriptor::default()),
            out_view: out.create_view(&wgpu::TextureViewDescriptor::default()),
            out,
        });
    }

    /// Renders one frame and reads it back as tightly packed RGBA bytes.
    /// BLOCKING: call from a background thread only.
    pub(in crate::features::pages::model_viewer) fn render(
        &mut self,
        job: RenderJob,
    ) -> Result<Vec<u8>> {
        if job.width == 0 || job.height == 0 {
            anyhow::bail!("zero-sized viewport");
        }
        self.reconfigure(job.width, job.height);
        if let Some(mesh) = job.swap_mesh {
            self.set_mesh(&mesh);
        }

        let aspect = job.width as f32 / job.height as f32;
        let (sin_pitch, cos_pitch) = job.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = job.yaw.sin_cos();
        let eye = [
            job.distance * cos_pitch * cos_yaw,
            job.distance * sin_pitch,
            job.distance * cos_pitch * sin_yaw,
        ];
        let view = mat4_look_at(eye, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let model: Mat4 = mat4_rotate_y(job.spin);
        let projection = perspective(FRAC_PI_4, aspect, 0.1, 100.0);
        let mvp = mat4_mul(&projection, &mat4_mul(&view, &model));
        let light_dir = [0.6f32, 1.0, 0.4, 0.0];
        let mut params_bytes = [0u8; 160];
        params_bytes[0..64].copy_from_slice(&encode_f32_rows(&mvp));
        params_bytes[64..128].copy_from_slice(&encode_f32_rows(&model));
        params_bytes[128..144].copy_from_slice(&encode_f32_rows(&light_dir));
        self.engine
            .queue()
            .write_buffer(&self.frame_params, 0, &params_bytes);

        let targets = self.targets.as_ref().expect("reconfigure ran");
        let mut encoder =
            self.engine
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("model-encode"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model-scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.out_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.04,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.mesh_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            if self.index_count > 0 {
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.index_count, 0, 0..1);
            }
        }

        self.engine.queue().submit(Some(encoder.finish()));
        self.engine
            .read_texture_bytes(&targets.out, job.width, job.height)
    }
}

#[cfg(test)]
pub(in crate::features::pages::model_viewer) fn wgsl_source() -> String {
    [FRAME_PARAMS_WGSL, MODEL_WGSL].concat()
}

#[cfg(test)]
#[path = "renderer.test.rs"]
mod renderer_test;
