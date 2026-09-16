use std::sync::Arc;

use crate::{
    command::{ChoiceOption, Command, VmWait},
    loader::Script,
    state::GameState,
};

pub struct Vm {
    script: Arc<Script>,
    pub pc: usize,
    pub state: GameState,
    pub wait: VmWait,
}

impl Vm {
    pub fn new(script: Arc<Script>) -> Self {
        Self {
            script,
            pc: 0,
            state: GameState::default(),
            wait: VmWait::Running,
        }
    }

    pub fn step(&mut self) -> VmWait {
        self.wait = VmWait::Running;

        while self.pc < self.script.commands.len() {
            let cmd = &self.script.commands[self.pc];

            match cmd {
                Command::Label(_) => {
                    self.pc += 1;
                    continue;
                }
                Command::SetFlag { flag, value } => {
                    self.state.set_flag(flag, *value);
                    self.pc += 1;
                    continue;
                }
                Command::ClearFlag(name) => {
                    self.state.clear_flag(name);
                    self.pc += 1;
                    continue;
                }
                Command::SetVar { name, value } => {
                    self.state.set_var(name, *value);
                    self.pc += 1;
                    continue;
                }
                Command::Jump(dest) => {
                    match self.script.label_index(dest) {
                        Some(idx) => self.pc = idx,
                        None => {
                            self.wait = VmWait::Finished;
                            return self.wait.clone();
                        }
                    }
                    continue;
                }
                Command::JumpIf { flag, dest } => {
                    if self.state.get_flag(flag) {
                        match self.script.label_index(dest) {
                            Some(idx) => self.pc = idx,
                            None => {
                                self.wait = VmWait::Finished;
                                return self.wait.clone();
                            }
                        }
                    } else {
                        self.pc += 1;
                    }
                    continue;
                }
                Command::Log(text) => {
                    self.state.push_history(text.clone());
                    self.pc += 1;
                    continue;
                }
                Command::Wait => {
                    self.wait = VmWait::Click;
                    self.pc += 1;
                    return self.wait.clone();
                }
                Command::WaitMs(ms) => {
                    self.wait = VmWait::Timer(*ms);
                    self.pc += 1;
                    return self.wait.clone();
                }
                Command::Choice { options } => {
                    let visible: Vec<ChoiceOption> = options
                        .iter()
                        .filter(|opt| match &opt.condition {
                            Some(flag) => self.state.get_flag(flag),
                            None => true,
                        })
                        .cloned()
                        .collect();
                    self.wait = VmWait::Choice(visible);
                    self.pc += 1;
                    return self.wait.clone();
                }
                Command::End => {
                    self.wait = VmWait::Finished;
                    self.pc += 1;
                    return self.wait.clone();
                }

                _ => {
                    let result = cmd.clone();
                    self.pc += 1;
                    self.wait = VmWait::Pending(result);
                    return self.wait.clone();
                }
            }
        }

        self.wait = VmWait::Finished;
        self.wait.clone()
    }

    pub fn resume(&mut self) {
        self.wait = VmWait::Running;
    }

    pub fn resume_with_choice(&mut self, jump_to: &str) {
        match self.script.label_index(jump_to) {
            Some(idx) => self.pc = idx,
            None => self.wait = VmWait::Finished,
        }
        self.wait = VmWait::Running;
    }
    pub fn tick_timer(&mut self, delta_ms: u32) -> bool {
        if let VmWait::Timer(remaining) = &mut self.wait {
            if *remaining < delta_ms {
                *remaining = 0;
                self.wait = VmWait::Running;
                true
            } else {
                *remaining -= delta_ms;
                false
            }
        } else {
            false
        }
    }
}
