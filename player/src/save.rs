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
        let savedata = SaveData {
            pc: self.vm.pc.saturating_sub(1),
            flags: self.vm.state.flags.clone(),
            vars: self.vm.state.vars.clone(),
            draw_queue: renderer.draw_queue.clone(),
            chapter_title: self.chapter_title.as_ref().map(|(t, _)| t.clone()),
            preview_text: self.pending_text.as_ref().map(|(_, t)| t.clone()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        let json = serde_json::to_string(&savedata).unwrap();
        std::fs::create_dir_all("saves").unwrap();
        std::fs::write("saves/quick.json", json).unwrap();
        println!("[SAVE] saved to saves/quick.json");
    }
    pub fn quick_load(&mut self, renderer: &mut Renderer) {
        let json = std::fs::read_to_string("saves/quick.json").unwrap();
        let data: SaveData = serde_json::from_str(&json).unwrap();
        self.vm.pc = data.pc;
        self.vm.state.flags = data.flags;
        self.vm.state.vars = data.vars;
        self.vm.wait = celest_core::command::VmWait::Running;
        self.click_pending = false;
        self.pending_text = None;
        self.show_choice = None;
        renderer.draw_queue = data.draw_queue;
        println!("[LOAD] loaded from saves/quick.json");
    }
    pub fn save_to_slot(&self, slot: usize) {}
    pub fn load_from_slot(&mut self, slot: usize) {}
}
