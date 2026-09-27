pub mod texture_cache;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use texture_cache::TextureCache;

use wgpu::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState, ColorTargetState,
    ColorWrites, Instance, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderStages, TextureDescriptor,
    TextureViewDescriptor, util::DeviceExt,
};

use winit::window::Window;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const FULL_QUAD: [Vertex; 6] = [
    Vertex {
        position: [-1.0, -1.0],
        tex_coord: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
        tex_coord: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coord: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0],
        tex_coord: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coord: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, 1.0],
        tex_coord: [0.0, 0.0],
    },
];

enum DrawCommand {
    Background { asset_id: String },
}

pub struct Renderer {
    device: std::sync::Arc<wgpu::Device>,
    queue: std::sync::Arc<wgpu::Queue>,
    surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,

    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture_cache: TextureCache,

    vertex_buffer: wgpu::Buffer,
    draw_queue: Vec<DrawCommand>,
    clear_color: wgpu::Color,
    asset_root: PathBuf,
}

impl Renderer {
    pub fn new(
        window: Arc<Window>,
        device: std::sync::Arc<wgpu::Device>,
        queue: std::sync::Arc<wgpu::Queue>,
        surface: wgpu::Surface<'static>,
        surface_config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let format = surface_config.format;
        surface.configure(&*device, &surface_config);

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("texture_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("texture.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("texture_bind_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("texture_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bg_quad_vbo"),
            contents: bytemuck::cast_slice(&FULL_QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let clear_color = wgpu::Color {
            r: 0.05,
            g: 0.05,
            b: 0.08,
            a: 1.0,
        };
        let asset_root = PathBuf::from("assets");

        Self {
            device,
            queue,
            surface,
            surface_config: surface_config,
            pipeline,
            bind_group_layout,
            sampler,
            texture_cache: TextureCache::new(),
            asset_root,
            vertex_buffer,
            draw_queue: Vec::new(),
            clear_color,
        }
    }

    pub fn clear(&mut self, color: [f32; 4]) {
        self.clear_color = wgpu::Color {
            r: color[0] as f64,
            g: color[1] as f64,
            b: color[2] as f64,
            a: color[3] as f64,
        };
    }

    pub fn draw_bg(&mut self, asset_id: &str) {
        if let Err(e) =
            self.texture_cache
                .get_or_load(&self.device, &self.queue, asset_id, &self.asset_root)
        {
            eprintln!("draw_bg: load {asset_id} faild: {e}");
            return;
        }

        self.draw_queue
            .retain(|c| !matches!(c, DrawCommand::Background { .. }));
        self.draw_queue.push(DrawCommand::Background {
            asset_id: asset_id.to_owned(),
        });
    }

    pub fn clear_bg(&mut self) {
        self.draw_queue
            .retain(|c| !matches!(c, DrawCommand::Background { .. }));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    /// 不画 egui，仅画背景（render_text 临时 bin 用）
    pub fn render_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("celest_bg"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("celest_bg_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            for cmd in &self.draw_queue {
                let DrawCommand::Background { asset_id } = cmd;
                let Some(tex_view) = self.texture_cache.get(asset_id) else { continue; };
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("texture_bind_group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(tex_view) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    ],
                });
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..6, 0..1);
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn render_frame_with_egui<E>(
        &mut self,
        egui_draw: E,
    ) -> Result<(), wgpu::SurfaceError>
    where
        E: FnOnce(&mut wgpu::CommandEncoder, &wgpu::TextureView),
    {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("celest_frame"),
            });

        // pass 1: background
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("celest_bg_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            for cmd in &self.draw_queue {
                let DrawCommand::Background { asset_id } = cmd;
                let Some(tex_view) = self.texture_cache.get(asset_id) else {
                    continue;
                };

                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("texture_bind_group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(tex_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..6, 0..1);
            }
        }

        // pass 2: egui (same encoder, same view)
        egui_draw(&mut encoder, &view);

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn device_clone(&self) -> std::sync::Arc<wgpu::Device> {
        std::sync::Arc::clone(&self.device)
    }

    pub fn queue_clone(&self) -> std::sync::Arc<wgpu::Queue> {
        std::sync::Arc::clone(&self.queue)
    }
}

fn load_texture_from_file(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &Path,
) -> Result<(wgpu::Texture, wgpu::TextureView, u32, u32), String> {
    let img = image::open(path)
        .map_err(|e| format!("Image load faild {path:?}: {e}"))?
        .to_rgba8();
    let (width, height) = img.dimensions();

    let texture = device.create_texture(&TextureDescriptor {
        label: Some(&format!("texture: {path:?}")),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &img,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&TextureViewDescriptor::default());
    Ok((texture, view, width, height))
}

