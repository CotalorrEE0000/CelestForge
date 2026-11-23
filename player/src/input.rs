use celest_renderer::Renderer;
use winit::{event::KeyEvent, keyboard::PhysicalKey};

use crate::{
    game::GameSession,
    ui::{UiMode, UiState},
};

pub enum InputAction {
    None,
    Advance,
    ToggleMenu,
    ToggleAuto,
    QuickSave,
    QuickLoad,
    BacklogUp,
    BacklogDown,
}

pub fn map_key(event: &KeyEvent) -> InputAction {
    use winit::keyboard::KeyCode;

    if event.state != winit::event::ElementState::Pressed {
        return InputAction::None;
    }

    let PhysicalKey::Code(code) = event.physical_key else {
        return InputAction::None;
    };

    match code {
        KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter => InputAction::Advance,
        KeyCode::Escape => InputAction::ToggleMenu,
        KeyCode::Tab => InputAction::ToggleAuto,
        KeyCode::F5 => InputAction::QuickSave,
        KeyCode::F9 => InputAction::QuickLoad,
        KeyCode::ArrowLeft | KeyCode::PageUp => InputAction::BacklogUp,
        KeyCode::ArrowRight | KeyCode::PageDown => InputAction::BacklogDown,
        _ => InputAction::None,
    }
}

pub fn handle(
    event: &KeyEvent,
    ui: &mut UiState,
    session: &mut GameSession,
    renderer: &mut Renderer,
) {
    let action = map_key(event);
    match action {
        InputAction::None => {}
        InputAction::Advance => {
            if matches!(ui.mode, UiMode::Normal) {
                session.resume_click();
            }
        }
        InputAction::QuickSave => {
            if matches!(ui.mode, UiMode::Normal) && !session.finished {
                session.quick_save(renderer);
            }
        }
        InputAction::QuickLoad => {
            if matches!(ui.mode, UiMode::Normal) {
                session.quick_load(renderer);
            }
        }
        InputAction::ToggleMenu => {
            ui.mode = match ui.mode {
                UiMode::Normal => UiMode::Menu,
                UiMode::Menu => UiMode::Normal,
                UiMode::BackLog => UiMode::Normal,
                UiMode::Settings => ui.settings_return,
                _ => return,
            };
        }
        InputAction::ToggleAuto => {
            if matches!(ui.mode, UiMode::Normal) {
                session.auto.toggle();
            }
        }
        InputAction::BacklogUp => {
            if matches!(ui.mode, UiMode::Normal) {
                ui.backlog_return = UiMode::Normal;
                ui.mode = UiMode::BackLog;
            }
        }
        InputAction::BacklogDown => {
            if matches!(ui.mode, UiMode::BackLog) {
                ui.mode = UiMode::Normal;
            }
        }
    }
}
