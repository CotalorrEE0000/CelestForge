use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameState {
    pub flags: HashMap<String, bool>,
    pub vars: HashMap<String, i64>,
    pub history: Vec<String>,
}

impl GameState {
    pub fn set_flag(&mut self, name: &str, value: bool) {
        self.flags.insert(name.to_string(), value);
    }
    pub fn get_flag(&self, name: &str) -> bool {
        *self.flags.get(name).unwrap_or(&false)
    }
    pub fn clear_flag(&mut self, name: &str) {
        self.flags.remove(name);
    }
    pub fn set_var(&mut self, name: &str, value: i64) {
        self.vars.insert(name.to_string(), value);
    }
    pub fn get_var(&self, name: &str) -> i64 {
        *self.vars.get(name).unwrap_or(&0)
    }
    pub fn push_history(&mut self, line: String) {
        self.history.push(line);
    }
}
