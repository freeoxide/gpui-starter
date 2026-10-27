use std::sync::Arc;

use anyhow::{Context as _, Result};
use gpui_kit::RenderImage;

use super::demos::{DEMOS, DemoSpec, ShaderParams, WARP_SOURCE_SIZE, warp_source_bytes};
use crate::features::wgpu::{WgpuEngine, bgra_to_render_image};

struct RenderTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// Pipelines, bind groups, and fixed-resolution render targets for the four
/// demos over the shared bridge. `render_frame` takes `&self`: encoding,
/// submission, and readback never mutate the stored handles.
pub(in crate::features::pages::shaders) struct ShaderRuntime {
    engine: Arc<WgpuEngine>,
    uniform: wgpu::Buffer,
    pipelines: [wgpu::RenderPipeline; 4],
    bind_groups: [wgpu::BindGroup; 4],
    targets: [RenderTarget; 4],
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
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

fn buffer_resource(buffer: &wgpu::Buffer) -> wgpu::BindingResource<'_> {
    wgpu::BindingResource::Buffer(wgpu::BufferBinding {
        buffer,
        offset: 0,
        size: None,
    })
}

fn array_of_four<T>(mut build: impl FnMut(usize) -> T) -> [T; 4] {
    [build(0), build(1), build(2), build(3)]
}

impl ShaderRuntime {
    /// Synchronous and infallible: wgpu `create_*` calls report through the
    /// uncaptured-error handler, and the naga tests pin the shader sources.
    pub(in crate::features::pages::shaders) fn new(engine: Arc<WgpuEngine>) -> Self {
        let device = engine.device();
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader-canvas-uniform"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let plain_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shader-canvas-plain-layout"),
            entries: &[uniform_entry(0)],
        });
        let warp_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shader-canvas-warp-layout"),
            entries: &[uniform_entry(0), texture_entry(1), sampler_entry(2)],
        });

        let warp_source = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shader-canvas-warp-source"),
            size: wgpu::Extent3d {
                width: WARP_SOURCE_SIZE,
                height: WARP_SOURCE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        engine.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &warp_source,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &warp_source_bytes(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WARP_SOURCE_SIZE * 4),
                rows_per_image: Some(WARP_SOURCE_SIZE),
            },
            wgpu::Extent3d {
                width: WARP_SOURCE_SIZE,
                height: WARP_SOURCE_SIZE,
                depth_or_array_layers: 1,
            },
        );
        let warp_view = warp_source.create_view(&wgpu::TextureViewDescriptor::default());
        let warp_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shader-canvas sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipelines = array_of_four(|ix| {
            let spec = &DEMOS[ix];
            let layout = if spec.uses_texture {
                &warp_layout
            } else {
                &plain_layout
            };
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shader-canvas demo"),
                source: wgpu::ShaderSource::Wgsl(spec.wgsl.into()),
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shader-canvas-pipeline-layout"),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shader-canvas pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
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
            })
        });

        let bind_groups = array_of_four(|ix| {
            let spec = &DEMOS[ix];
            let layout = if spec.uses_texture {
                &warp_layout
            } else {
                &plain_layout
            };
            let mut entries = vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_resource(&uniform),
            }];
            if spec.uses_texture {
                entries.push(wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&warp_view),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&warp_sampler),
                });
            }
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shader-canvas bind group"),
                layout,
                entries: &entries,
            })
        });

        let targets = array_of_four(|ix| {
            let spec = &DEMOS[ix];
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shader-canvas target"),
                size: wgpu::Extent3d {
                    width: spec.width,
                    height: spec.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            RenderTarget {
                view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
                texture,
            }
        });

        Self {
            engine,
            uniform,
            pipelines,
            bind_groups,
            targets,
        }
    }

    /// Renders one frame; blocking (the bridge readback polls), so background
    /// thread only.
    pub(in crate::features::pages::shaders) fn render_frame(
        &self,
        spec: &DemoSpec,
        params: ShaderParams,
    ) -> Result<Arc<RenderImage>> {
        self.engine
            .queue()
            .write_buffer(&self.uniform, 0, &params.to_bytes());
        let ix = spec.id.ix();
        let target = &self.targets[ix];
        let mut encoder =
            self.engine
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("shader-canvas-encode"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipelines[ix]);
            pass.set_bind_group(0, &self.bind_groups[ix], &[]);
            pass.draw(0..3, 0..1);
        }
        self.engine.queue().submit(Some(encoder.finish()));
        let bgra = self
            .engine
            .read_texture_bytes(&target.texture, spec.width, spec.height)
            .context("reading shader frame back")?;
        Ok(bgra_to_render_image(spec.width, spec.height, bgra))
    }
}

#[cfg(test)]
#[path = "runtime.test.rs"]
mod runtime_test;
