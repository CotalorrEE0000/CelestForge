use std::path::Path;

use crate::{game::GameSession, save::SaveData};

pub enum UiAction {
    None,
    StartGame,
    LoadGame,
    BackToTitle,
    ClickContinue,
    PickChoice { jump_to: String },
    BackToGame,
    OpenSlots,
    LoadSlot(usize),
    SaveSlot(usize),
    CloseBacklog,
}

pub enum UiMode {
    Title,
    Normal,
    Menu,
    Slots,
    BackLog,
}

pub struct UiState {
    pub mode: UiMode,
    pub slots_return: UiMode,
    pub slots_mode: SlotsMode,
}

pub enum SlotsMode {
    Load,
    Save,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            mode: UiMode::Title,
            slots_return: UiMode::Title,
            slots_mode: SlotsMode::Load,
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context, session: &GameSession) -> UiAction {
        match self.mode {
            UiMode::Title => self.draw_title(ctx),
            UiMode::Menu => self.draw_menu(ctx),
            UiMode::Normal => self.draw_game(ctx, session),
            UiMode::Slots => self.draw_slots(ctx),
            UiMode::BackLog => self.draw_backlog(ctx, session),
        }
    }

    fn draw_game(&mut self, ctx: &egui::Context, session: &GameSession) -> UiAction {
        let mut action = UiAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                if let Some((title, _)) = &session.chapter_title {
                    ui.vertical_centered(|ui| {
                        ui.add_space(280.0);
                        ui.heading(
                            egui::RichText::new(title)
                                .size(48.0)
                                .color(egui::Color32::WHITE),
                        );
                    });
                }

                if session.chapter_title.is_none() {
                    if session.pending_text.is_some() && session.show_choice.is_none() {
                        let area_resp = egui::Area::new(egui::Id::new("dialog_box"))
                            .order(egui::Order::Middle)
                            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -40.0])
                            .show(ctx, |ui| {
                                egui::Frame::none()
                                    .fill(egui::Color32::from_black_alpha(180))
                                    .rounding(8.0)
                                    .inner_margin(egui::Margin::same(16.0))
                                    .show(ui, |ui| {
                                        ui.set_min_width(400.0);
                                        if let Some((speaker, content)) = &session.pending_text {
                                            if let Some(name) = speaker {
                                                ui.label(
                                                    egui::RichText::new(name)
                                                        .strong()
                                                        .size(20.0)
                                                        .color(egui::Color32::LIGHT_BLUE),
                                                );
                                            }
                                            ui.label(
                                                egui::RichText::new(content)
                                                    .size(18.0)
                                                    .color(egui::Color32::WHITE),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::RIGHT),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new("▼ click to continue")
                                                            .small()
                                                            .color(egui::Color32::GRAY),
                                                    );
                                                    if session.auto.enabled {
                                                        ui.label(
                                                            egui::RichText::new("Auto")
                                                                .small()
                                                                .strong()
                                                                .color(egui::Color32::LIGHT_GREEN),
                                                        );
                                                    }
                                                },
                                            );
                                        }
                                    });
                            });
                        if area_resp.response.clicked() {
                            action = UiAction::ClickContinue;
                        }
                    }

                    if let Some(options) = &session.show_choice {
                        egui::Area::new(egui::Id::new("choice_box"))
                            .order(egui::Order::Middle)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, -80.0])
                            .show(ctx, |ui| {
                                ui.vertical(|ui| {
                                    for (i, opt) in options.iter().enumerate() {
                                        let label = format!("{}. {}", i + 1, opt.text);
                                        if ui
                                            .button(egui::RichText::new(label).size(22.0))
                                            .clicked()
                                        {
                                            action = UiAction::PickChoice {
                                                jump_to: opt.jump_to.clone(),
                                            };
                                        }
                                    }
                                });
                            });
                    }
                }
            });
        action
    }

    fn draw_menu(&mut self, ctx: &egui::Context) -> UiAction {
        let mut action = UiAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_black_alpha(160)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(160.0);
                    ui.heading(
                        egui::RichText::new("菜单")
                            .size(40.0)
                            .color(egui::Color32::WHITE),
                    );

                    ui.add_space(48.0);
                    if ui
                        .button(egui::RichText::new("返回游戏").size(26.0))
                        .clicked()
                    {
                        action = UiAction::BackToGame;
                    }

                    ui.add_space(16.0);
                    if ui
                        .button(egui::RichText::new("返回标题").size(26.0))
                        .clicked()
                    {
                        action = UiAction::BackToTitle;
                    }

                    ui.add_space(16.0);
                    if ui
                        .button(egui::RichText::new("读取存档").size(26.0))
                        .clicked()
                    {
                        self.slots_mode = SlotsMode::Load;
                        self.slots_return = UiMode::Menu;
                        action = UiAction::OpenSlots;
                    }

                    ui.add_space(16.0);
                    if ui
                        .button(egui::RichText::new("保存存档").size(26.0))
                        .clicked()
                    {
                        self.slots_mode = SlotsMode::Save;
                        self.slots_return = UiMode::Menu;
                        action = UiAction::OpenSlots
                    }
                });
            });
        action
    }

    fn draw_title(&mut self, ctx: &egui::Context) -> UiAction {
        let mut action = UiAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_rgb(12, 12, 24)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(200.0);
                    ui.heading(
                        egui::RichText::new("Celest Forge")
                            .size(64.0)
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(88.0);
                    if ui
                        .button(egui::RichText::new("开始游戏").size(28.0))
                        .clicked()
                    {
                        action = UiAction::StartGame;
                    }
                    ui.add_space(16.0);
                    if ui
                        .button(egui::RichText::new("继续游戏").size(28.0))
                        .clicked()
                    {
                        action = UiAction::LoadGame;
                    }
                    ui.add_space(16.0);
                    if ui
                        .button(egui::RichText::new("读取存档").size(28.0))
                        .clicked()
                    {
                        self.slots_return = UiMode::Title;
                        self.slots_mode = SlotsMode::Load;
                        action = UiAction::OpenSlots;
                    }
                });
            });
        action
    }

    fn draw_slots(&mut self, ctx: &egui::Context) -> UiAction {
        let mut action = UiAction::None;
        let saving = matches!(self.slots_mode, SlotsMode::Save);
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_rgb(12, 12, 24)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.heading(
                        egui::RichText::new(if saving {
                            "保存存档"
                        } else {
                            "读取存档"
                        })
                        .size(40.0)
                        .color(egui::Color32::WHITE),
                    );
                    ui.add_space(32.0);
                    for slot in 0..6 {
                        let path = if slot == 0 {
                            "saves/quick.json".to_string()
                        } else {
                            format!("saves/slot_{}.json", slot)
                        };
                        let exists = Path::new(&path).exists();
                        let label = if exists {
                            match std::fs::read_to_string(&path)
                                .ok()
                                .and_then(|j| serde_json::from_str::<SaveData>(&j).ok())
                            {
                                Some(d) => {
                                    format!("SLOT {}: {}", slot, d.preview_text.unwrap_or_default())
                                }
                                None => format!("SLOT {} (LOAD FAILED)", slot),
                            }
                        } else {
                            format!("SLOT {}(EMPTY)", slot)
                        };
                        let btn = ui.button(egui::RichText::new(label).size(22.0));
                        if btn.clicked() {
                            if saving {
                                action = UiAction::SaveSlot(slot);
                            } else if exists {
                                action = UiAction::LoadSlot(slot);
                            }
                        }
                    }
                    ui.add_space(24.0);
                    let back = if matches!(self.slots_return, UiMode::Menu) {
                        UiAction::BackToGame
                    } else {
                        UiAction::BackToTitle
                    };
                    if ui.button(egui::RichText::new("Back").size(22.0)).clicked() {
                        action = back;
                    }
                })
            });
        action
    }

    fn draw_backlog(&mut self, ctx: &egui::Context, session: &GameSession) -> UiAction {
        let mut action = UiAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_black_alpha(230)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.heading(
                        egui::RichText::new("History")
                            .size(32.0)
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(12.0);
                });

                let history = &session.vm.state.history;
                let start = history.len().saturating_sub(100);
                egui::ScrollArea::vertical()
                    .max_height(520.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if history.is_empty() {
                            ui.label(
                                egui::RichText::new("(Don't have any dialogs)")
                                    .size(18.0)
                                    .color(egui::Color32::GRAY),
                            );
                        };
                        for line in &history[start..] {
                            ui.label(
                                egui::RichText::new(line)
                                    .size(18.0)
                                    .color(egui::Color32::WHITE),
                            );
                            ui.add_space(6.0);
                        }
                    });
                ui.add_space(16.0);
                ui.vertical_centered(|ui| {
                    if ui
                        .button(egui::RichText::new("Close [ESC]").size(20.0))
                        .clicked()
                    {
                        action = UiAction::CloseBacklog;
                    }
                });
            });
        action
    }
}
