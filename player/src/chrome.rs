use crate::ui::UiMode;

pub const BAR_H: f32 = 40.0;

const ITEM_W: f32 = 72.0;
const ITEM_H: f32 = 30.0;
const LABEL_SIZE: f32 = 15.0;
const BAR_BG_ALPHA: u8 = 150;
const BAR_PAD_R: f32 = 12.0;
const ITEM_GAP: f32 = 8.0;
const TOPBAR_H: f32 = 44.0;
const HOT_ZONE_H: f32 = 20.0;
const HIDE_DELAY: f64 = 0.28;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChromeIntent {
    None,
    Save,
    Load,
    History,
    ToggleAuto,
    Settings,
    BackToTitle,
    Quit,
}

pub struct Chrome {
    topbar_visible: bool,
    hide_since: Option<f64>,
}

impl Default for Chrome {
    fn default() -> Self {
        Self::new()
    }
}

impl Chrome {
    pub fn new() -> Self {
        Self {
            topbar_visible: false,
            hide_since: None,
        }
    }

    pub fn tick(&mut self, now: f64, pointer_y: Option<f32>) {
        let in_hot = pointer_y.map_or(false, |y| y <= HOT_ZONE_H);
        let in_panel = self.topbar_visible && pointer_y.map_or(false, |y| y <= TOPBAR_H);

        if in_hot || in_panel {
            self.topbar_visible = true;
            self.hide_since = None;
        } else if self.topbar_visible {
            match self.hide_since {
                None => self.hide_since = Some(now),
                Some(t) if now - t >= HIDE_DELAY => {
                    self.topbar_visible = false;
                    self.hide_since = None;
                }
                _ => {}
            }
        }
    }

    pub fn topbar_visible(&self) -> bool {
        self.topbar_visible
    }

    pub fn draw(&mut self, ctx: &egui::Context, mode: UiMode, auto_enabled: bool) -> ChromeIntent {
        if !chrome_visible(mode) {
            self.topbar_visible = false;
            self.hide_since = None;
            return ChromeIntent::None;
        }

        let now = ctx.input(|i| i.time);
        let pointer_y = ctx.input(|i| i.pointer.hover_pos()).map(|p| p.y);
        self.tick(now, pointer_y);

        let mut intent = ChromeIntent::None;
        let screen_w = ctx.screen_rect().width();

        egui::Area::new(egui::Id::new("cf_chrome_bottom"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::LEFT_BOTTOM, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(screen_w);
                egui::Frame::none()
                    .fill(egui::Color32::from_black_alpha(BAR_BG_ALPHA))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(screen_w, BAR_H));
                        ui.spacing_mut().item_spacing.x = ITEM_GAP;
                        ui.horizontal_centered(|ui| {
                            let total = ITEM_W * 5.0 + ITEM_GAP * 4.0 + BAR_PAD_R;
                            ui.add_space((screen_w - total).max(0.0));
                            if bar_item(ui, "存档", false).clicked() {
                                intent = ChromeIntent::Save;
                            }
                            if bar_item(ui, "读档", false).clicked() {
                                intent = ChromeIntent::Load;
                            }
                            if bar_item(ui, "历史", false).clicked() {
                                intent = ChromeIntent::History;
                            }
                            if bar_item(ui, "自动", auto_enabled).clicked() {
                                intent = ChromeIntent::ToggleAuto;
                            }
                            if bar_item(ui, "设置", false).clicked() {
                                intent = ChromeIntent::Settings;
                            }
                        });
                    });
            });

        // Top bar
        if self.topbar_visible {
            egui::Area::new(egui::Id::new("cf_chrome_top"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(screen_w);
                    egui::Frame::none()
                        .fill(egui::Color32::from_black_alpha(170))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(screen_w, TOPBAR_H));
                            let half = screen_w * 0.5;
                            ui.scope_builder(
                                egui::UiBuilder::new()
                                    .max_rect(egui::Rect::from_min_size(
                                        egui::pos2(0.0, 0.0),
                                        egui::vec2(half, TOPBAR_H),
                                    ))
                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                                |ui| {
                                    ui.add_space(16.0);
                                    ui.label(
                                        egui::RichText::new("Celest Forge")
                                            .size(15.0)
                                            .color(egui::Color32::from_gray(200)),
                                    );
                                },
                            );
                            ui.scope_builder(
                                egui::UiBuilder::new()
                                    .max_rect(egui::Rect::from_min_size(
                                        egui::pos2(half, 0.0),
                                        egui::vec2(half, TOPBAR_H),
                                    ))
                                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                                |ui| {
                                    ui.add_space(12.0);
                                    if bar_item(ui, "退出", false).clicked() {
                                        intent = ChromeIntent::Quit;
                                    }
                                    if bar_item(ui, "返回标题", false).clicked() {
                                        intent = ChromeIntent::BackToTitle;
                                    }
                                },
                            );
                        });
                });
        }

        intent
    }
}

pub fn chrome_visible(mode: UiMode) -> bool {
    matches!(mode, UiMode::Normal)
}

fn bar_item(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ITEM_W, ITEM_H), egui::Sense::click());
    if active || resp.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(6.0),
            egui::Color32::from_white_alpha(if active { 46 } else { 24 }),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(LABEL_SIZE),
        if active {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(215, 215, 225)
        },
    );
    resp
}
