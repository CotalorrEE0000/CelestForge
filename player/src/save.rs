use celest_renderer::{DrawCommand, Renderer};

use crate::game::GameSession;
use std::collections::HashMap;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct SaveData {
    pub pc: usize,
    pub flags: HashMap<String, bool>,
    pub vars: HashMap<String, i64>,
    pub draw_queue: Vec<DrawCommand>,
    pub chapter_title: Option<String>,
    pub preview_text: Option<String>,
    pub timestamp: u64,
}

impl GameSession {
    pub fn quick_save(&self, renderer: &Renderer) {
        self.save_to_slot(0, renderer);
    }
    pub fn quick_load(&mut self, renderer: &mut Renderer) {
        self.load_from_slot(0, renderer);
    }

    fn save_path(slot: usize) -> String {
        if slot == 0 {
            "saves/quick.json".to_string()
        } else {
            format!("saves/slot_{}.json", slot)
        }
    }

    fn build_save(&self, renderer: &Renderer) -> SaveData {
        SaveData {
            pc: self.vm.pc.saturating_sub(1),
            flags: self.vm.state.flags.clone(),
            vars: self.vm.state.vars.clone(),
            draw_queue: renderer.draw_queue.clone(),
            chapter_title: self.chapter_title.as_ref().map(|(t, _, _)| t.clone()),
            preview_text: self
                .pending_text
                .as_ref()
                .map(|(_, t)| t.clone())
                .or_else(|| self.chapter_title.as_ref().map(|(t, _, _)| t.clone())),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    pub fn save_to_slot(&self, slot: usize, renderer: &Renderer) {
        let savedata = self.build_save(renderer);
        let json = serde_json::to_string(&savedata).unwrap();
        std::fs::create_dir_all("saves").unwrap();
        std::fs::write(Self::save_path(slot), json).unwrap();
        println!("[SAVE] saved to {}", Self::save_path(slot));
    }

    pub fn load_from_slot(&mut self, slot: usize, renderer: &mut Renderer) {
        let json = std::fs::read_to_string(Self::save_path(slot)).unwrap();
        let data: SaveData = serde_json::from_str(&json).unwrap();
        self.vm.pc = data.pc;
        self.vm.state.flags = data.flags;
        self.vm.state.vars = data.vars;
        self.vm.wait = celest_core::command::VmWait::Running;
        self.click_pending = false;
        self.pending_text = None;
        self.show_choice = None;
        self.finished = false;
        self.transition = None;
        renderer.draw_queue = data.draw_queue;
        renderer.end_transition();
        renderer.ensure_textures_loaded();
        println!("[LOAD] loaded from {}", Self::save_path(slot));
    }
}
