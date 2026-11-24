use std::path::Path;

use crate::{
    chrome::{Chrome, ChromeIntent},
    game::GameSession,
    save::SaveData,
};

const DIALOG_W_RATIO: f32 = 0.78;
const DIALOG_MAX_W: f32 = 960.0;
const DIALOG_H: f32 = 148.0;
const DIALOG_PAD: f32 = 18.0;
const NAME_H: f32 = 26.0;
const BODY_SIZE: f32 = 18.0;
const SPEAKER_DEFAULT: egui::Color32 = egui::Color32::LIGHT_BLUE;

pub enum UiAction {
    None,
    StartGame,
    LoadGame,
    BackToTitle,
    ClickContinue,
    PickChoice { jump_to: String },
    BackToGame,
    BackToMenu,
    OpenSlots,
    LoadSlot(usize),
    SaveSlot(usize),
    ConfirmOverwrite(usize),
    CancelOverwrite,
    SetAutoplay(u64),
    SetBgmVolume(f32),
    SetSeVolume(f32),
    OpenBacklog,
    OpenSettings,
    CloseBacklog,
    ConfirmQuit,
    CancelQuit,
    Quit,
}

#[derive(Clone, Copy)]
pub enum UiMode {
    Title,
    Normal,
    Menu,
    Slots,
    BackLog,
    Settings,
}

pub struct UiState {
    pub mode: UiMode,
    pub slots_return: UiMode,
    pub slots_mode: SlotsMode,
    pub backlog_return: UiMode,
    pub pending_overwrite: Option<usize>,
    pub speaker_colors: std::collections::HashMap<String, egui::Color32>,
    pub settings_return: UiMode,
    pub chrome: Chrome,
    pub pending_quit: bool,
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
            backlog_return: UiMode::Normal,
            pending_overwrite: None,
            speaker_colors: load_speakr_colors(Path::new("assets/speakers.json")),
            settings_return: UiMode::Menu,
            chrome: Chrome::new(),
            pending_quit: false,
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context, session: &mut GameSession) -> UiAction {
        let mut action = match self.mode {
            UiMode::Title => self.draw_title(ctx),
            UiMode::Menu => self.draw_menu(ctx),
            UiMode::Normal => self.draw_game(ctx, session),
            UiMode::Slots => self.draw_slots(ctx),
            UiMode::BackLog => self.draw_backlog(ctx, session),
            UiMode::Settings => self.draw_settings(ctx, session),
        };

        if !self.pending_quit {
            let mode = self.mode;
            match self.chrome.draw(ctx, mode, session.auto.enabled) {
                ChromeIntent::None => {}
                ChromeIntent::ToggleAuto => session.auto.toggle(),
                other => action = self.apply_chrome_intent(other),
            }
        }

        if self.pending_quit {
            match self.draw_quit_confirm(ctx) {
                UiAction::None => {
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                        action = UiAction::CancelQuit;
                    }
                }
                other => action = other,
            }
        }

        action
    }

    fn apply_chrome_intent(&mut self, intent: ChromeIntent) -> UiAction {
        match intent {
            ChromeIntent::None | ChromeIntent::ToggleAuto => UiAction::None,
            ChromeIntent::Save => {
                self.slots_mode = SlotsMode::Save;
                self.slots_return = self.return_target();
                UiAction::OpenSlots
            }
            ChromeIntent::Load => {
                self.slots_mode = SlotsMode::Load;
                self.slots_return = self.return_target();
                UiAction::OpenSlots
            }
            ChromeIntent::History => {
                self.backlog_return = self.return_target();
                UiAction::OpenBacklog
            }
            ChromeIntent::Settings => UiAction::OpenSettings,
            ChromeIntent::BackToTitle => UiAction::BackToTitle,
            ChromeIntent::Quit => UiAction::Quit,
        }
    }

    fn return_target(&self) -> UiMode {
        match self.mode {
            UiMode::Normal => UiMode::Normal,
            _ => UiMode::Menu,
        }
    }

    fn draw_game(&mut self, ctx: &egui::Context, session: &mut GameSession) -> UiAction {
        let mut action = UiAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                if let Some((title, remaining, total)) = &session.chapter_title {
                    let elapsed = (total - remaining) as f32;
                    let alpha = (elapsed / 400.0)
                        .min(1.0)
                        .min(*remaining as f32 / 400.0)
                        .clamp(0.0, 1.0);
                    ui.vertical_centered(|ui| {
                        ui.add_space(280.0);
                        ui.heading(
                            egui::RichText::new(title)
                                .size(48.0)
                                .color(egui::Color32::from_white_alpha((alpha * 255.0) as u8)),
                        );
                    });
                }

                if session.chapter_title.is_none() {
                    let pending = session.pending_text.clone();
                    let no_choice = session.show_choice.is_none();
                    if let Some((speaker, content)) = pending {
                        if no_choice {
                            let screen_w = ctx.screen_rect().width();
                            let box_w = (screen_w * DIALOG_W_RATIO).min(DIALOG_MAX_W);
                            let inner_w = box_w - DIALOG_PAD * 2.0;
                            let body_h = DIALOG_H - DIALOG_PAD * 2.0 - NAME_H;

                            // resize each frame in real window size
                            let pages = paginate_text(ctx, &content, inner_w, body_h);
                            let count = pages.len().max(1);
                            session.page_count = count;
                            let idx = session.page.min(count - 1);
                            let body = pages.get(idx).map(|s| s.as_str()).unwrap_or("");

                            let colors = &self.speaker_colors;
                            let box_resp = egui::Area::new(egui::Id::new("dialog_box"))
                                .order(egui::Order::Middle)
                                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -56.0])
                                .show(ctx, |ui| {
                                    egui::Frame::none()
                                        .fill(egui::Color32::from_black_alpha(180))
                                        .rounding(8.0)
                                        .inner_margin(egui::Margin::same(DIALOG_PAD))
                                        .show(ui, |ui| {
                                            ui.set_min_size(egui::vec2(
                                                inner_w,
                                                DIALOG_H - DIALOG_PAD * 2.0,
                                            ));
                                            ui.set_max_width(inner_w);
                                            ui.set_max_height(DIALOG_H - DIALOG_PAD * 2.0);

                                            ui.allocate_ui(egui::vec2(inner_w, NAME_H), |ui| {
                                                if let Some(name) = &speaker {
                                                    let color = colors
                                                        .get(name)
                                                        .copied()
                                                        .unwrap_or(SPEAKER_DEFAULT);
                                                    ui.label(
                                                        egui::RichText::new(name)
                                                            .strong()
                                                            .size(20.0)
                                                            .color(color),
                                                    );
                                                }
                                            });

                                            ui.label(
                                                egui::RichText::new(body)
                                                    .size(BODY_SIZE)
                                                    .color(egui::Color32::WHITE),
                                            );

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::RIGHT),
                                                |ui| {
                                                    let hint = if count > 1 {
                                                        format!("{} / {}  ▼", idx + 1, count)
                                                    } else {
                                                        "▼".to_string()
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(hint)
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
                                        });
                                });
                            if box_resp.response.clicked() {
                                action = UiAction::ClickContinue;
                            }
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
                    ui.add_space(120.0);
                    ui.heading(
                        egui::RichText::new("菜单")
                            .size(40.0)
                            .color(egui::Color32::WHITE),
                    );

                    ui.add_space(38.0);
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

                    ui.add_space(16.0);
                    if ui
                        .button(egui::RichText::new("回顾历史").size(26.0))
                        .clicked()
                    {
                        self.backlog_return = UiMode::Menu;
                        action = UiAction::OpenBacklog;
                    }

                    ui.add_space(16.0);
                    if ui.button(egui::RichText::new("设置").size(26.0)).clicked() {
                        action = UiAction::OpenSettings;
                    }

                    ui.add_space(16.0);
                    if ui
                        .button(egui::RichText::new("退出游戏").size(26.0))
                        .clicked()
                    {
                        action = UiAction::Quit;
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

                    ui.add_space(16.0);
                    if ui
                        .button(egui::RichText::new("退出游戏").size(26.0))
                        .clicked()
                    {
                        action = UiAction::Quit;
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
                    let blocked = self.pending_overwrite.is_some();
                    ui.add_enabled_ui(!blocked, |ui| {
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
                                        format!(
                                            "SLOT {}: {}",
                                            slot,
                                            d.preview_text.unwrap_or_default()
                                        )
                                    }
                                    None => format!("SLOT {} (LOAD FAILED)", slot),
                                }
                            } else {
                                format!("SLOT {}(EMPTY)", slot)
                            };
                            let btn = ui.button(egui::RichText::new(label).size(22.0));
                            if btn.clicked() {
                                if saving {
                                    if exists {
                                        self.pending_overwrite = Some(slot);
                                    } else {
                                        action = UiAction::SaveSlot(slot);
                                    }
                                } else if exists {
                                    action = UiAction::LoadSlot(slot);
                                }
                            }
                        }
                    });

                    ui.add_space(24.0);
                    let back = match self.slots_return {
                        UiMode::Normal => UiAction::BackToGame,
                        UiMode::Menu => UiAction::BackToMenu,
                        _ => UiAction::BackToTitle,
                    };

                    if ui.button(egui::RichText::new("Back").size(22.0)).clicked() {
                        action = back;
                    }

                    if let Some(slot) = self.pending_overwrite {
                        egui::Window::new("确认覆盖")
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                ui.label(egui::RichText::new(format!(
                                    "SLOT {} 已有存档，确定覆盖？",
                                    slot
                                )));
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    if ui.button(egui::RichText::new("确定").size(20.0)).clicked()
                                    {
                                        action = UiAction::ConfirmOverwrite(slot);
                                    }
                                    if ui.button(egui::RichText::new("取消").size(20.0)).clicked()
                                    {
                                        action = UiAction::CancelOverwrite;
                                    }
                                });
                            });
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

    fn draw_settings(&mut self, ctx: &egui::Context, session: &GameSession) -> UiAction {
        let mut action = UiAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_rgb(12, 12, 24)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().slider_width = 300.0;
                    ui.add_space(80.0);
                    ui.heading(
                        egui::RichText::new("设置")
                            .size(40.0)
                            .color(egui::Color32::WHITE),
                    );

                    ui.add_space(48.0);

                    ui.label(
                        egui::RichText::new("自动播放间隔")
                            .size(18.0)
                            .color(egui::Color32::WHITE),
                    );

                    let mut ms = session.auto.interval_ms as u32;
                    if ui
                        .add_sized(
                            [360.0, 24.0],
                            egui::Slider::new(&mut ms, 300..=3000).suffix("ms"),
                        )
                        .changed()
                    {
                        action = UiAction::SetAutoplay(ms as u64);
                    }

                    ui.add_space(32.0);
                    ui.label(
                        egui::RichText::new("文字速度")
                            .size(18.0)
                            .color(egui::Color32::WHITE),
                    );

                    ui.add_enabled_ui(false, |ui| {
                        let mut cps = 40.0_f32;
                        ui.add_sized(
                            [360.0, 24.0],
                            egui::Slider::new(&mut cps, 10.0..=100.0).suffix(" cps"),
                        );
                    });

                    ui.label(
                        egui::RichText::new("打字机尚未实现，占位中")
                            .size(14.0)
                            .color(egui::Color32::GRAY),
                    );

                    ui.add_space(32.0);
                    ui.label(
                        egui::RichText::new("BGM 音量")
                            .size(18.0)
                            .color(egui::Color32::WHITE),
                    );
                    let mut bgm = session.audio.bgm_volume();
                    if ui
                        .add_sized([360.0, 24.0], egui::Slider::new(&mut bgm, 0.0..=1.0))
                        .changed()
                    {
                        action = UiAction::SetBgmVolume(bgm);
                    }

                    ui.add_space(32.0);
                    ui.label(
                        egui::RichText::new("Se 音量")
                            .size(18.0)
                            .color(egui::Color32::WHITE),
                    );
                    let mut se = session.audio.se_volume();
                    if ui
                        .add_sized([360.0, 24.0], egui::Slider::new(&mut se, 0.0..=1.0))
                        .changed()
                    {
                        action = UiAction::SetSeVolume(se);
                    }

                    ui.add_space(48.0);
                    if ui.button(egui::RichText::new("返回").size(24.0)).clicked() {
                        action = UiAction::BackToMenu;
                    }
                });
            });
        action
    }

    fn draw_quit_confirm(&mut self, ctx: &egui::Context) -> UiAction {
        let mut action = UiAction::None;
        egui::Area::new(egui::Id::new("cf_quit_confirm"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_black_alpha(236))
                    .rounding(10.0)
                    .inner_margin(egui::Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(320.0, 0.0));
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("确定要退出 Celest Forge 吗？")
                                    .size(20.0)
                                    .color(egui::Color32::WHITE),
                            );
                            ui.add_space(18.0);
                            ui.horizontal(|ui| {
                                let btn_w: f32 = 96.0;
                                let gap: f32 = 16.0;
                                let pad = ((320.0 - btn_w * 2.0 - gap) * 0.5).max(0.0);
                                ui.add_space(pad);
                                if ui
                                    .add_sized(
                                        [btn_w, 32.0],
                                        egui::Button::new(egui::RichText::new("退出").size(18.0)),
                                    )
                                    .clicked()
                                {
                                    action = UiAction::ConfirmQuit;
                                }
                                ui.add_space(gap);
                                if ui
                                    .add_sized(
                                        [btn_w, 32.0],
                                        egui::Button::new(egui::RichText::new("取消").size(18.0)),
                                    )
                                    .clicked()
                                {
                                    action = UiAction::CancelQuit;
                                }
                            });
                        });
                    });
            });
        action
    }
}

fn paginate_text(ctx: &egui::Context, text: &str, wrap_width: f32, max_h: f32) -> Vec<String> {
    let galley = ctx.fonts(|f| {
        f.layout(
            text.to_owned(),
            egui::FontId::proportional(BODY_SIZE),
            egui::Color32::WHITE,
            wrap_width,
        )
    });

    let mut pages = Vec::new();
    let (mut start, mut acc, mut h) = (0usize, 0usize, 0.0f32);
    for row in &galley.rows {
        let rh = row.rect.height();
        if max_h < rh + h && 0 < acc {
            pages.push(text.chars().skip(start).take(acc).collect());
            start += acc;
            acc = 0;
            h = 0.0;
        }
        h += rh;
        acc += row.glyphs.len();
        if row.ends_with_newline {
            acc += 1;
        }
    }

    let tail: String = text.chars().skip(start).collect();
    if !tail.is_empty() {
        pages.push(tail);
    }
    pages
}

fn load_speakr_colors(path: &Path) -> std::collections::HashMap<String, egui::Color32> {
    let mut out = std::collections::HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return out;
    };
    match serde_json::from_str::<std::collections::HashMap<String, String>>(&text) {
        Ok(map) => {
            for (name, hex) in map {
                match parse_hex_color(&hex) {
                    Some(c) => {
                        out.insert(name, c);
                    }
                    None => eprintln!("[SPEAKER] COLOR FORMAT INCORRECT {name} = {hex}"),
                }
            }
        }
        Err(e) => eprintln!("[SPEAKER] {path:?} LOAD FAILED: {e}"),
    }
    out
}

fn parse_hex_color(s: &str) -> Option<egui::Color32> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(egui::Color32::from_rgb(r, g, b))
}
