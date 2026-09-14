use std::sync::Arc;

use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiState;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{self, EventLoop},
    window::Window,
};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    surface: Option<Surface<'static>>,
    device: Option<Device>,
    queue: Option<Queue>,
    config: Option<SurfaceConfiguration>,
    clear_color: wgpu::Color,
    egui_state: Option<EguiState>,
    egui_render: Option<EguiRenderer>,
    egui_context: egui::Context,
}

impl App {
    fn init_wgpu(&mut self, window: Arc<Window>) {
        let instance = Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let info = adapter.get_info();
        println!("SELECTED GPU: {} | BACKEND: {:?}", info.name, info.backend);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("celest device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .unwrap();

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
    }

    fn init_egui(&mut self) {
        let window = self.window.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();
        let format = self.config.as_ref().unwrap().format;

        let state = EguiState::new(
            self.egui_context.clone(),
            egui::ViewportId::ROOT,
            window,
            None,
            None,
            None,
        );
        let renderer = EguiRenderer::new(device, format, None, 1, false);

        self.egui_state = Some(state);
        self.egui_render = Some(renderer);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let window = self.window.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        let config = self.config.as_ref().unwrap();

        let raw_input = self.egui_state.as_mut().unwrap().take_egui_input(window);
        self.egui_context.begin_pass(raw_input);
        let mut bg = self.clear_color;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(&self.egui_context, |ui| {
                ui.heading("Celest Forge Debug");
                ui.add(egui::Slider::new(&mut bg.r, 0.0..=1.0).text("R"));
                ui.add(egui::Slider::new(&mut bg.g, 0.0..=1.0).text("G"));
                ui.add(egui::Slider::new(&mut bg.b, 0.0..=1.0).text("B"));
            });
        self.clear_color = bg;
        let full_output = self.egui_context.end_pass();

        let egui_render = self.egui_render.as_mut().unwrap();
        for (id, image_delta) in &full_output.textures_delta.set {
            egui_render.update_texture(device, queue, *id, image_delta);
        }

        for id in &full_output.textures_delta.free {
            egui_render.free_texture(id);
        }

        let clipped_primitives = self
            .egui_context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let frame = surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear encoder"),
        });

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        let egui_state = self.egui_state.as_mut().unwrap();
        // let egui_render = self.egui_render.as_mut().unwrap();
        egui_render.update_buffers(
            device,
            queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main pass"),
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
            let mut pass_ref = pass.forget_lifetime();
            egui_render.render(&mut pass_ref, &clipped_primitives, &screen_descriptor);
        }

        queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && 0 < new_size.height {
            if let (Some(surface), Some(config), Some(device)) = (
                self.surface.as_ref(),
                self.config.as_mut(),
                self.device.as_ref(),
            ) {
                config.width = new_size.width;
                config.height = new_size.height;
                surface.configure(device, config);
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let attr = Window::default_attributes().with_title("Celest Forge v0.1.0");
        let window = Arc::new(event_loop.create_window(attr).unwrap());
        self.window = Some(window.clone());
        self.init_wgpu(window);
        self.clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.35,
            a: 1.0,
        };
        self.init_egui();
        event_loop.set_control_flow(event_loop::ControlFlow::Poll);
    }
    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let (Some(window), Some(egui_state)) = (self.window.as_ref(), self.egui_state.as_mut()) {
            egui_state.on_window_event(window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                println!("Celest Forge Closed!");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.resize(size);
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.render() {
                    eprintln!("RENDER ERROR: {e}")
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
