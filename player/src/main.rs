use std::sync::Arc;

pub mod game;
pub mod input;
pub mod save;
pub mod ui;

use celest_renderer::Renderer;
use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiState;
use wgpu::{Device, Instance, Queue, SurfaceConfiguration};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{self, EventLoop},
    window::Window,
};

use crate::{
    game::GameSession,
    ui::{UiAction, UiMode, UiState},
};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    device: Option<Arc<Device>>,
    queue: Option<Arc<Queue>>,
    config: Option<SurfaceConfiguration>,
    surface: Option<wgpu::Surface<'static>>,
    egui_state: Option<EguiState>,
    egui_render: Option<EguiRenderer>,
    egui_context: egui::Context,

    session: Option<GameSession>,
    renderer: Option<Renderer>,
    ui: Option<UiState>,
    last_frame: Option<std::time::Instant>,
    exit_requested: bool,
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
        let format = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        self.device = Some(Arc::new(device));
        self.queue = Some(Arc::new(queue));
        self.config = Some(config);
        self.surface = Some(surface);
    }

    fn init_egui(&mut self, device: &Arc<wgpu::Device>, _queue: &Arc<wgpu::Queue>) {
        let window = self.window.as_ref().unwrap();
        let format = self.config.as_ref().unwrap().format;

        let state = EguiState::new(
            self.egui_context.clone(),
            egui::ViewportId::ROOT,
            window,
            None,
            None,
            None,
        );

        let font_path = "assets/font.ttf";
        let font_data = std::fs::read(font_path)
            .expect("Can't load assets/font.ttf，please confirm file is exist");
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("my_font".to_owned(), egui::FontData::from_owned(font_data));
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "my_font".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "my_font".to_owned());
        self.egui_context.set_fonts(fonts);

        let renderer = EguiRenderer::new(device.as_ref(), format, None, 1, false);

        self.egui_state = Some(state);
        self.egui_render = Some(renderer);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let window = self.window.as_ref().unwrap();
        let scale_factor = window.scale_factor() as f32;
        let now = std::time::Instant::now();
        let dt_ms = self
            .last_frame
            .map(|t| now.duration_since(t).as_millis() as u32)
            .unwrap_or(0);
        self.last_frame = Some(now);

        let raw_input = self.egui_state.as_mut().unwrap().take_egui_input(window);
        self.egui_context.begin_pass(raw_input);
        if matches!(self.ui.as_ref().unwrap().mode, UiMode::Normal) {
            self.session.as_mut().unwrap().auto_tick(dt_ms);
            self.session
                .as_mut()
                .unwrap()
                .step(self.renderer.as_mut().unwrap(), dt_ms);
        }

        if matches!(self.ui.as_ref().unwrap().mode, UiMode::Normal)
            && self.session.as_ref().unwrap().finished
        {
            self.ui.as_mut().unwrap().mode = UiMode::Title;
        }

        let session = self.session.as_ref().unwrap();

        let action = self.ui.as_mut().unwrap().draw(&self.egui_context, session);

        match action {
            UiAction::None => {}
            UiAction::ClickContinue => {
                self.session.as_mut().unwrap().resume_click();
            }
            UiAction::PickChoice { jump_to } => {
                self.session.as_mut().unwrap().pick_choice(&jump_to);
            }
            UiAction::StartGame => {
                self.session = Some(
                    GameSession::from_script_path("scripts/test_chapter1.json")
                        .expect("Load script failed!"),
                );
                self.renderer.as_mut().unwrap().draw_queue.clear();
                self.ui.as_mut().unwrap().mode = UiMode::Normal;
            }
            UiAction::LoadGame => {
                self.session
                    .as_mut()
                    .unwrap()
                    .quick_load(self.renderer.as_mut().unwrap());
                self.ui.as_mut().unwrap().mode = UiMode::Normal;
            }
            UiAction::BackToTitle => {
                self.ui.as_mut().unwrap().mode = UiMode::Title;
            }
            UiAction::BackToGame => {
                self.ui.as_mut().unwrap().mode = UiMode::Normal;
            }
            UiAction::OpenSlots => {
                self.ui.as_mut().unwrap().mode = UiMode::Slots;
            }
            UiAction::LoadSlot(slot) => {
                self.session
                    .as_mut()
                    .unwrap()
                    .load_from_slot(slot, self.renderer.as_mut().unwrap());
                self.ui.as_mut().unwrap().mode = UiMode::Normal;
            }
            UiAction::SaveSlot(slot) => {
                self.session
                    .as_mut()
                    .unwrap()
                    .save_to_slot(slot, self.renderer.as_ref().unwrap());
                let back_to_menu = matches!(self.ui.as_ref().unwrap().slots_return, UiMode::Menu);
                self.ui.as_mut().unwrap().mode = if back_to_menu {
                    UiMode::Menu
                } else {
                    UiMode::Title
                };
            }
            UiAction::CloseBacklog => {
                let back = self.ui.as_ref().unwrap().backlog_return;
                self.ui.as_mut().unwrap().mode = back;
            }
            UiAction::BackToMenu => {
                self.ui.as_mut().unwrap().mode = UiMode::Menu;
            }
            UiAction::OpenBacklog => {
                self.ui.as_mut().unwrap().mode = UiMode::BackLog;
            }
            UiAction::OpenSettings => {
                self.ui.as_mut().unwrap().mode = UiMode::Settings;
            }
            UiAction::ConfirmOverwrite(slot) => {
                self.session
                    .as_mut()
                    .unwrap()
                    .save_to_slot(slot, self.renderer.as_ref().unwrap());
                self.ui.as_mut().unwrap().pending_overwrite = None;
            }
            UiAction::CancelOverwrite => {
                self.ui.as_mut().unwrap().pending_overwrite = None;
            }
            UiAction::SetAutoplay(ms) => {
                self.session.as_mut().unwrap().auto.interval_ms = ms;
            }
            UiAction::Quit => {
                self.exit_requested = true;
            }
        }

        let full_output = self.egui_context.end_pass();

        let egui_render = self.egui_render.as_mut().unwrap();
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();

        for (id, image_delta) in &full_output.textures_delta.set {
            egui_render.update_texture(device.as_ref(), queue.as_ref(), *id, image_delta);
        }
        for id in &full_output.textures_delta.free {
            egui_render.free_texture(id);
        }

        let clipped_primitives = self
            .egui_context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let (width, height) = {
            let r = self.renderer.as_ref().unwrap();
            (r.surface_config.width, r.surface_config.height)
        };
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: scale_factor,
        };

        self.renderer
            .as_mut()
            .unwrap()
            .render_frame_with_egui(|encoder, view| {
                egui_render.update_buffers(
                    device.as_ref(),
                    queue.as_ref(),
                    encoder,
                    &clipped_primitives,
                    &screen_descriptor,
                );

                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                let mut pass_ref = pass.forget_lifetime();
                egui_render.render(&mut pass_ref, &clipped_primitives, &screen_descriptor);
                drop(pass_ref);
            })?;

        Ok(())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && 0 < new_size.height {
            if let Some(r) = self.renderer.as_mut() {
                r.resize(new_size.width, new_size.height);
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let attr = Window::default_attributes().with_title("Celest Forge v0.1.0");
        let window = Arc::new(event_loop.create_window(attr).unwrap());
        self.window = Some(window.clone());
        self.init_wgpu(window.clone());
        let device = Arc::clone(self.device.as_ref().unwrap());
        let queue = Arc::clone(self.queue.as_ref().unwrap());
        self.init_egui(&device, &queue);
        let surface = self.surface.take().unwrap();
        let config = self.config.take().unwrap();
        self.renderer = Some(Renderer::new(device, queue, surface, config));
        self.session = Some(
            GameSession::from_script_path("scripts/test_chapter1.json")
                .expect("Load script failed!"),
        );
        self.ui = Some(UiState::new());
        event_loop.set_control_flow(event_loop::ControlFlow::Poll);
    }
    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let (Some(window), Some(egui_state)) = (self.window.as_ref(), self.egui_state.as_mut()) {
            let _ = egui_state.on_window_event(window, &event);
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
            WindowEvent::KeyboardInput { event, .. } => {
                input::handle(
                    &event,
                    self.ui.as_mut().unwrap(),
                    self.session.as_mut().unwrap(),
                    self.renderer.as_mut().unwrap(),
                );
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.render() {
                    eprintln!("RENDER ERROR: {e}")
                }
                if self.exit_requested {
                    println!("Celest Forge Closed!");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &event_loop::ActiveEventLoop) {
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
