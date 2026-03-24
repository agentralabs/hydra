//! Event dispatch — maps crossterm events to Actions.
//! Pure function: no state mutation, just produces actions.
//! Uses keybinding config for customizable shortcuts.

use crate::v2::action::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

/// Dispatch a crossterm event into actions.
/// `modal_open` determines whether we're in modal or conversation context.
pub fn dispatch_event(event: &Event, modal_open: bool) -> Vec<Action> {
    match event {
        Event::Key(key) => {
            if modal_open {
                dispatch_modal_key(key)
            } else {
                dispatch_conversation_key(key)
            }
        }
        Event::Mouse(mouse) => dispatch_mouse(mouse),
        Event::Resize(w, h) => vec![Action::System(SystemAction::Resize {
            width: *w,
            height: *h,
        })],
        _ => vec![],
    }
}

/// Key dispatch in conversation mode (normal input).
fn dispatch_conversation_key(key: &KeyEvent) -> Vec<Action> {
    // Ctrl combinations
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('d') => vec![Action::System(SystemAction::Quit)],
            KeyCode::Char('k') => vec![Action::Modal(ModalAction::OpenPalette)],
            KeyCode::Char('l') => vec![Action::Stream(StreamAction::Clear)],
            KeyCode::Char('a') => vec![Action::Input(InputAction::MoveHome)],
            KeyCode::Char('e') => vec![Action::Input(InputAction::MoveEnd)],
            KeyCode::Char('u') => vec![Action::Input(InputAction::KillLine)],
            KeyCode::Char('w') => vec![Action::Input(InputAction::KillWord)],
            KeyCode::Char('y') => vec![Action::Input(InputAction::Yank)],
            KeyCode::Char('r') => vec![Action::Input(InputAction::SearchStart)],
            KeyCode::Char('v') => vec![Action::Voice(VoiceAction::Toggle)],
            KeyCode::Char('o') => vec![Action::System(SystemAction::ToggleExpand)],
            KeyCode::Char('b') => vec![Action::Companion(CompanionAction::Pause)], // companion status
            KeyCode::Char('p') => vec![Action::Voice(VoiceAction::Toggle)], // TTS toggle
            _ => vec![],
        };
    }

    // Alt combinations
    if key.modifiers.contains(KeyModifiers::ALT) {
        return match key.code {
            KeyCode::Char('b') => vec![Action::Input(InputAction::MoveWordLeft)],
            KeyCode::Char('f') => vec![Action::Input(InputAction::MoveWordRight)],
            KeyCode::Enter => vec![Action::Input(InputAction::InsertChar('\n'))],
            _ => vec![],
        };
    }

    // Shift+Enter = newline
    if key.modifiers.contains(KeyModifiers::SHIFT) && key.code == KeyCode::Enter {
        return vec![Action::Input(InputAction::InsertChar('\n'))];
    }

    // Plain keys
    match key.code {
        KeyCode::Enter => vec![Action::Input(InputAction::Submit)],
        KeyCode::Backspace => vec![Action::Input(InputAction::Backspace)],
        KeyCode::Delete => vec![Action::Input(InputAction::Delete)],
        KeyCode::Left => vec![Action::Input(InputAction::MoveLeft)],
        KeyCode::Right => vec![Action::Input(InputAction::MoveRight)],
        KeyCode::Home => vec![Action::Input(InputAction::MoveHome)],
        KeyCode::End => vec![Action::Input(InputAction::MoveEnd)],
        KeyCode::Up => vec![Action::Input(InputAction::HistoryUp)],
        KeyCode::Down => vec![Action::Input(InputAction::HistoryDown)],
        KeyCode::PageUp => vec![Action::Stream(StreamAction::ScrollUp(10))],
        KeyCode::PageDown => vec![Action::Stream(StreamAction::ScrollDown(10))],
        KeyCode::Esc => vec![], // no-op in conversation mode
        KeyCode::Char(c) => vec![Action::Input(InputAction::InsertChar(c))],
        _ => vec![],
    }
}

/// Key dispatch in modal mode (palette, config editor, etc.).
fn dispatch_modal_key(key: &KeyEvent) -> Vec<Action> {
    // Escape always closes modal
    if key.code == KeyCode::Esc {
        return vec![Action::Modal(ModalAction::Close)];
    }

    // Ctrl+C also closes modal
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return vec![Action::Modal(ModalAction::Close)];
    }

    match key.code {
        KeyCode::Enter => vec![Action::Modal(ModalAction::Select)],
        KeyCode::Up => vec![Action::Modal(ModalAction::NavigateUp)],
        KeyCode::Down => vec![Action::Modal(ModalAction::NavigateDown)],
        KeyCode::Backspace => vec![Action::Modal(ModalAction::Backspace)],
        KeyCode::Char(c) => vec![Action::Modal(ModalAction::TypeChar(c))],
        _ => vec![],
    }
}

/// Mouse event dispatch.
fn dispatch_mouse(mouse: &crossterm::event::MouseEvent) -> Vec<Action> {
    match mouse.kind {
        MouseEventKind::ScrollUp => vec![Action::Stream(StreamAction::ScrollUp(3))],
        MouseEventKind::ScrollDown => vec![Action::Stream(StreamAction::ScrollDown(3))],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn ctrl_k_opens_palette() {
        let actions = dispatch_conversation_key(&key(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert!(matches!(actions[0], Action::Modal(ModalAction::OpenPalette)));
    }

    #[test]
    fn ctrl_c_quits() {
        let actions = dispatch_conversation_key(&key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(matches!(actions[0], Action::System(SystemAction::Quit)));
    }

    #[test]
    fn enter_submits() {
        let actions = dispatch_conversation_key(&key(KeyCode::Enter, KeyModifiers::empty()));
        assert!(matches!(actions[0], Action::Input(InputAction::Submit)));
    }

    #[test]
    fn shift_enter_inserts_newline() {
        let actions = dispatch_conversation_key(&key(KeyCode::Enter, KeyModifiers::SHIFT));
        assert!(matches!(actions[0], Action::Input(InputAction::InsertChar('\n'))));
    }

    #[test]
    fn escape_closes_modal() {
        let actions = dispatch_modal_key(&key(KeyCode::Esc, KeyModifiers::empty()));
        assert!(matches!(actions[0], Action::Modal(ModalAction::Close)));
    }

    #[test]
    fn pageup_scrolls() {
        let actions = dispatch_conversation_key(&key(KeyCode::PageUp, KeyModifiers::empty()));
        assert!(matches!(actions[0], Action::Stream(StreamAction::ScrollUp(10))));
    }

    #[test]
    fn char_inserts() {
        let actions = dispatch_conversation_key(&key(KeyCode::Char('a'), KeyModifiers::empty()));
        assert!(matches!(actions[0], Action::Input(InputAction::InsertChar('a'))));
    }

    #[test]
    fn modal_enter_selects() {
        let actions = dispatch_modal_key(&key(KeyCode::Enter, KeyModifiers::empty()));
        assert!(matches!(actions[0], Action::Modal(ModalAction::Select)));
    }
}
