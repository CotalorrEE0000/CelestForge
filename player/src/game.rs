use std::sync::Arc;

use celest_core::{
    command::{ChoiceOption, Command, VmWait},
    loader::Loader,
    vm::Vm,
};
use celest_renderer::Renderer;

pub struct GameSession {
    pub vm: Vm,

    pub pending_text: Option<(Option<String>, String)>,
    pub show_choice: Option<Vec<ChoiceOption>>,
    pub chapter_title: Option<(String, u32)>,
    pub click_pending: bool,
    pub auto: Autoplay,
}

pub struct Autoplay {
    pub enabled: bool,
    pub interval_ms: u64,
    elapsed_ms: u64,
}

impl Autoplay {
    pub fn new() -> Self {
        Self {
            enabled: false,
            interval_ms: 1200,
            elapsed_ms: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.elapsed_ms = 0;
    }

    pub fn reset(&mut self) {
        self.elapsed_ms = 0;
    }

    fn tick(&mut self, dt_ms: u32) -> bool {
        if !self.enabled {
            return false;
        }

        self.elapsed_ms += dt_ms as u64;
        if self.elapsed_ms >= self.interval_ms {
            self.elapsed_ms = 0;
            true
        } else {
            false
        }
    }
}

impl GameSession {
    pub fn from_script_path(path: &str) -> Result<Self, String> {
        let script = Arc::new(Loader::from_file(path)?);
        let vm = Vm::new(Arc::clone(&script));
        Ok(Self {
            vm,
            pending_text: None,
            show_choice: None,
            chapter_title: None,
            click_pending: false,
            auto: Autoplay::new(),
        })
    }

    pub fn step(&mut self, renderer: &mut Renderer) {
        if let Some((_, remaining)) = self.chapter_title.as_mut() {
            if *remaining <= 16 {
                self.chapter_title = None;
                self.vm.resume();
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

        loop {
            let vm = &mut self.vm;

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
                        renderer.draw_bg(asset_id);
                        vm.resume();
                        continue;
                    }
                    Command::ClearBg => {
                        renderer.clear_bg();
                        vm.resume();
                        continue;
                    }
                    Command::ShowSprite {
                        who,
                        asset_id,
                        position,
                    } => {
                        // self.active_sprites.insert(who.clone(), asset_id.clone());
                        eprintln!("[SPRITE] {}: {} (Phase 13)", who.clone(), asset_id.clone());
                        renderer.draw_sprite(asset_id, *position);
                        vm.resume();
                        continue;
                    }
                    Command::HideSprite { who } => {
                        // self.active_sprites.remove(who);
                        eprintln!("[SPRITE] hide {}", who);
                        renderer.hide_sprite(who);
                        vm.resume();
                        continue;
                    }
                    Command::ClearSprites => {
                        // self.active_sprites.clear();
                        eprintln!("[SPRITE] clear all");
                        renderer.clear_sprite();
                        vm.resume();
                        continue;
                    }
                    Command::Text { speaker, content } => {
                        self.pending_text = Some((speaker.clone(), content.clone()));
                        self.click_pending = true;
                        return;
                    }
                    Command::ChapterTitle { title, duration_ms } => {
                        self.chapter_title = Some((title.clone(), *duration_ms));
                        vm.resume();
                        return;
                    }
                    Command::PlayVoice { who, asset_id } => {
                        eprintln!("[VOICE] {}: {} (Phase 3)", who, asset_id);
                        vm.resume();
                        continue;
                    }
                    Command::PlaySe { asset_id } => {
                        eprintln!("[SE] {} (Phase 3)", asset_id);
                        vm.resume();
                        continue;
                    }
                    Command::PlayBgm { asset_id } => {
                        eprintln!("[BGM] {} (Phase 3)", asset_id);
                        vm.resume();
                        continue;
                    }
                    Command::Log(text) => {
                        eprintln!("[LOG] {}", text);
                        vm.resume();
                        continue;
                    }
                    _ => {
                        vm.resume();
                        return;
                    }
                },
                VmWait::Click => {
                    self.click_pending = true;
                    return;
                }
                VmWait::Choice(options) => {
                    self.show_choice = Some(options.clone());
                    return;
                }
                VmWait::Timer(_) => return,
                VmWait::Running => return,
                VmWait::Finished => {
                    println!("=== GameEvent ENDED ===");
                    return;
                }
            }
        }
    }

    pub fn resume_click(&mut self) {
        if self.click_pending {
            self.vm.resume();
            self.click_pending = false;
            self.pending_text = None;
        }
        self.auto.reset();
    }

    pub fn pick_choice(&mut self, jump_to: &str) {
        self.vm.resume_with_choice(jump_to);
        self.show_choice = None;
        self.click_pending = false;
        self.auto.reset();
    }

    pub fn auto_tick(&mut self, dt_ms: u32) {
        if !self.auto.tick(dt_ms) {
            return;
        }

        if self.click_pending && self.show_choice.is_none() && self.chapter_title.is_none() {
            self.resume_click();
        }
    }
}
