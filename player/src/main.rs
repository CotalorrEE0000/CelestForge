use std::sync::Arc;

use celest_core::{
    command::{ChoiceOption, Command, VmWait},
    loader::{Loader, Script},
    vm::Vm,
};
use celest_renderer::Renderer;
use egui::ahash::HashMap;
use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiState;
use wgpu::{Device, Instance, Queue, SurfaceConfiguration};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{self, EventLoop},
    window::Window,
};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    device: Option<Device>,
    queue: Option<Queue>,
    config: Option<SurfaceConfiguration>,
    egui_state: Option<EguiState>,
    egui_render: Option<EguiRenderer>,
    egui_context: egui::Context,

    renderer: Option<Renderer>,
    vm: Option<Vm>,
    script: Option<Arc<Script>>,
    pending_text: Option<(Option<String>, String)>,
    current_bg: Option<String>,
    active_sprites: HashMap<String, String>,
    show_choice: Option<Vec<ChoiceOption>>,
    chapter_title: Option<(String, u32)>,
    click_pending: bool,
}

impl App {
    fn init_wgpu(&mut self, window: Arc<Window>) {
        let instance = Instance::default();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

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
        self.handle_vm_step();
        let window = self.window.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        let config = self.config.as_ref().unwrap();

        let raw_input = self.egui_state.as_mut().unwrap().take_egui_input(window);
        self.egui_context.begin_pass(raw_input);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(&self.egui_context, |ui| {
                if let Some((title, _)) = &self.chapter_title {
                    ui.vertical_centered(|ui| {
                        ui.add_space(280.0);
                        ui.heading(egui::RichText::new(title).size(48.0));
                    });
                }
            });
        let full_output = self.egui_context.end_pass();

        let egui_render = self.egui_render.as_mut().unwrap();
        for (id, image_delta) in &full_output.textures_delta.set {
            egui_render.update_texture(device, queue, *id, image_delta);
        }

        for id in &full_output.textures_delta.free {
            egui_render.free_texture(id);
        }

        let _clipped = self
            .egui_context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        self.renderer.as_mut().unwrap().render_frame()?;
        Ok(())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && 0 < new_size.height {
            if let Some(r) = self.renderer.as_mut() {
                r.resize(new_size.width, new_size.height);
            }
            if let Some(config) = self.config.as_mut() {
                config.width = new_size.width;
                config.height = new_size.height;
            }
        }
    }

    fn handle_vm_step(&mut self) {
        if let Some((_, remaining)) = self.chapter_title.as_mut() {
            if *remaining <= 16 {
                self.chapter_title = None;
                self.vm.as_mut().unwrap().resume();
            } else {
                *remaining -= 16;
            }
            return;
        }

        if self.click_pending {
            return;
        }

        if self.show_choice.is_some() {
            return;
        }

        let vm = self.vm.as_mut().unwrap();

        if matches!(vm.wait, VmWait::Timer(_)) {
            if !vm.tick_timer(16) {
                return;
            }
        }

        if matches!(vm.wait, VmWait::Finished) {
            return;
        }

        let wait = vm.step();

        match &wait {
            VmWait::Pending(cmd) => match cmd {
                Command::SetBg { asset_id } => {
                    self.renderer.as_mut().unwrap().draw_bg(asset_id);
                    vm.resume();
                }
                Command::ClearBg => {
                    self.renderer.as_mut().unwrap().clear_bg();
                    vm.resume();
                }
                Command::ShowSprite {
                    who,
                    asset_id,
                    position,
                } => {
                    // self.active_sprites.insert(who.clone(), asset_id.clone());
                    eprintln!("[SPRITE] {}: {} (Phase 13)", who, asset_id);
                    vm.resume();
                }
                Command::HideSprite { who } => {
                    // self.active_sprites.remove(who);
                    eprintln!("[SPRITE] hide {}", who);
                    vm.resume();
                }
                Command::ClearSprites => {
                    // self.active_sprites.clear();
                    eprintln!("[SPRITE] clear all");
                    vm.resume();
                }
                Command::Text { speaker, content } => {
                    self.pending_text = Some((speaker.clone(), content.clone()));
                    self.click_pending = true;
                }
                Command::ChapterTitle { title, duration_ms } => {
                    self.chapter_title = Some((title.clone(), *duration_ms));
                    vm.resume();
                }
                Command::PlayVoice { who, asset_id } => {
                    eprintln!("[VOICE] {}: {} (Phase 3)", who, asset_id);
                    vm.resume();
                }
                Command::PlaySe { asset_id } => {
                    eprintln!("[SE] {} (Phase 3)", asset_id);
                    vm.resume();
                }
                Command::PlayBgm { asset_id } => {
                    eprintln!("[BGM] {} (Phase 3)", asset_id);
                    vm.resume();
                }
                Command::Log(text) => {
                    eprintln!("[LOG] {}", text);
                    vm.resume();
                }
                _ => {
                    vm.resume();
                }
            },
            VmWait::Click => {
                self.click_pending = true;
            }
            VmWait::Choice(options) => {
                self.show_choice = Some(options.clone());
            }
            VmWait::Timer(_) => {}
            VmWait::Running => {}
            VmWait::Finished => {
                println!("=== SCRIPT ENDED ===");
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
        self.init_egui();
        self.renderer = Some(Renderer::new(window.clone()));
        let script = Arc::new(
            Loader::from_file("scripts/test_chapter1.json")
                .expect("Load scripts/test_chapter1.json Faild!"),
        );
        self.script = Some(Arc::clone(&script));
        self.vm = Some(Vm::new(Arc::clone(&script)));
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
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                if state == winit::event::ElementState::Pressed
                    && button == winit::event::MouseButton::Left
                    && self.click_pending
                {
                    self.vm.as_mut().unwrap().resume();
                    self.click_pending = false;
                    self.pending_text = None;
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
