pub mod pipeline;
pub mod sprite;
pub mod texture_cache;
pub mod vertex;
use celest_core::command::{self, Position};
use std::path::{Path, PathBuf};
use texture_cache::TextureCache;
use vertex::{FULL_QUAD, Vertex};

use wgpu::{TextureDescriptor, TextureViewDescriptor, util::DeviceExt};

use crate::{
    pipeline::create_pipeline_layout,
    sprite::{build_quad_vertices, position_to_ndc},
};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum DrawCommand {
    Background {
        asset_id: String,
    },
    Sprite {
        asset_id: String,
        position: Position,
    },
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
    pub draw_queue: Vec<DrawCommand>,
    clear_color: wgpu::Color,
    asset_root: PathBuf,
}

impl Renderer {
    pub fn new(
        device: std::sync::Arc<wgpu::Device>,
        queue: std::sync::Arc<wgpu::Queue>,
        surface: wgpu::Surface<'static>,
        surface_config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let format = surface_config.format;
        surface.configure(&*device, &surface_config);

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

        let pipeline_set = create_pipeline_layout(&device, format);
        Self {
            device,
            queue,
            surface,
            surface_config: surface_config,
            pipeline: pipeline_set.pipeline,
            bind_group_layout: pipeline_set.bind_group_layout,
            sampler: pipeline_set.sampler,
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

    pub fn draw_sprite(&mut self, asset_id: &str, position: command::Position) {
        if let Err(e) =
            self.texture_cache
                .get_or_load(&self.device, &self.queue, asset_id, &self.asset_root)
        {
            eprintln!("draw_sprite: load {asset_id} failed: {e}");
            return;
        }

        self.draw_queue
            .retain(|c| !matches!(c, DrawCommand::Sprite { asset_id: a, .. } if a == asset_id));
        self.draw_queue.push(DrawCommand::Sprite {
            asset_id: asset_id.to_owned(),
            position,
        });
    }

    pub fn hide_sprite(&mut self, asset_id: &str) {
        self.draw_queue
            .retain(|c| !matches!(c, DrawCommand::Sprite { asset_id: a, .. } if a == asset_id));
    }

    pub fn clear_sprite(&mut self) {
        self.draw_queue
            .retain(|c| !matches!(c, DrawCommand::Sprite { .. }));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn render_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                match cmd {
                    DrawCommand::Background { asset_id } => {
                        let Some(tex_view) = self.texture_cache.get(asset_id) else {
                            continue;
                        };
                        let bind_group =
                            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                    DrawCommand::Sprite { asset_id, position } => {
                        let Some(tex_view) = self.texture_cache.get(asset_id) else {
                            continue;
                        };

                        let (img_w, img_h) = match self.texture_cache.get_size(asset_id) {
                            None => return Ok(()),
                            Some((img_w, img_h)) => (img_w, img_h),
                        };
                        let (x, y, w, h) = position_to_ndc(
                            *position,
                            img_w,
                            img_h,
                            self.surface_config.width,
                            self.surface_config.height,
                        );

                        let sprite_vbo =
                            self.device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some("sprite_quad_vbo"),
                                    contents: bytemuck::cast_slice(&build_quad_vertices(
                                        x, y, w, h,
                                    )),
                                    usage: wgpu::BufferUsages::VERTEX,
                                });
                        let bind_group =
                            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("sprite_bind_group"),
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
                        pass.set_vertex_buffer(0, sprite_vbo.slice(..));
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.draw(0..6, 0..1);
                    }
                };
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn render_frame_with_egui<E>(&mut self, egui_draw: E) -> Result<(), wgpu::SurfaceError>
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
                match cmd {
                    DrawCommand::Background { asset_id } => {
                        let Some(tex_view) = self.texture_cache.get(asset_id) else {
                            continue;
                        };

                        let bind_group =
                            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.draw(0..6, 0..1);
                    }
                    DrawCommand::Sprite { asset_id, position } => {
                        let Some(tex_view) = self.texture_cache.get(asset_id) else {
                            continue;
                        };
                        let (img_w, img_h) = match self.texture_cache.get_size(asset_id) {
                            None => return Ok(()),
                            Some((img_w, img_h)) => (img_w, img_h),
                        };
                        let (x, y, w, h) = position_to_ndc(
                            *position,
                            img_w,
                            img_h,
                            self.surface_config.width,
                            self.surface_config.height,
                        );

                        let sprites: [Vertex; 6] = [
                            Vertex {
                                position: [x, y],
                                tex_coord: [0.0, 0.0],
                            },
                            Vertex {
                                position: [x + w, y],
                                tex_coord: [1.0, 0.0],
                            },
                            Vertex {
                                position: [x + w, y - h],
                                tex_coord: [1.0, 1.0],
                            },
                            Vertex {
                                position: [x, y],
                                tex_coord: [0.0, 0.0],
                            },
                            Vertex {
                                position: [x + w, y - h],
                                tex_coord: [1.0, 1.0],
                            },
                            Vertex {
                                position: [x, y - h],
                                tex_coord: [0.0, 1.0],
                            },
                        ];
                        let sprite_vbo =
                            self.device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some("sprite_quad_vbo"),
                                    contents: bytemuck::cast_slice(&sprites),
                                    usage: wgpu::BufferUsages::VERTEX,
                                });
                        let bind_group =
                            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("sprite_bind_group"),
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
                        pass.set_vertex_buffer(0, sprite_vbo.slice(..));
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.draw(0..6, 0..1);
                    }
                }
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

    pub fn ensure_textures_loaded(&mut self) {
        let ids: Vec<String> = self
            .draw_queue
            .iter()
            .map(|c| match c {
                DrawCommand::Background { asset_id } => asset_id.clone(),
                DrawCommand::Sprite { asset_id, .. } => asset_id.clone(),
            })
            .collect();
        for id in ids {
            let _ =
                self.texture_cache
                    .get_or_load(&self.device, &self.queue, &id, &self.asset_root);
        }
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
