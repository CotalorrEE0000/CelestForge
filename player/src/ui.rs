use crate::game::GameSession;

pub enum UiAction {
    None,
    ClickContinue,
    PickChoice { jump_to: String },
}

pub struct UiState;

impl UiState {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(ctx: &egui::Context, session: &GameSession) -> UiAction {
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
                        egui::Area::new(egui::Id::new("dialog_box"))
                            .order(egui::Order::Foreground)
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
                                                },
                                            );
                                        }
                                        if ui.input(|i| i.pointer.primary_clicked()) {
                                            action = UiAction::ClickContinue
                                        }
                                    });
                            });
                    }

                    if let Some(options) = &session.show_choice {
                        egui::Area::new(egui::Id::new("choice_box"))
                            .order(egui::Order::Foreground)
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
}
