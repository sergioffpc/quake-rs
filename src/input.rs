use std::collections::HashMap;

use legion::system;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use crate::console::{Console, ConsoleCmd};

#[derive(Clone, Copy, Debug)]
pub enum InputEvent {
    KeyboardInput {
        code: KeyCode,
        state: ElementState,
    },
    MouseInput {
        button: MouseButton,
        state: ElementState,
    },
    MouseWheel {
        delta: MouseScrollDelta,
    },
}

#[derive(Debug, Default)]
pub struct Input {
    bindings: HashMap<String, String>,
}

impl Input {
    pub fn bind(&mut self, key: &str, action: &str) {
        self.bindings
            .insert(key.trim().to_lowercase(), action.trim().to_lowercase());
    }

    pub fn unbind(&mut self, key: &str) {
        self.bindings.remove(key.trim().to_lowercase().as_str());
    }

    pub fn unbind_all(&mut self) {
        self.bindings.clear();
    }

    pub fn handle_input_event(&self, input_event: InputEvent) -> Option<&String> {
        match input_event {
            InputEvent::KeyboardInput {
                code,
                state: ElementState::Pressed,
            } => Self::from_key_code(code),
            InputEvent::MouseInput {
                button,
                state: ElementState::Pressed,
            } => Self::from_mouse_button(button),
            InputEvent::MouseWheel { delta } => Self::from_mouse_scroll_delta(delta),
            _ => None,
        }
        .map_or(None, |key| {
            self.bindings.get(key.trim().to_lowercase().as_str())
        })
    }

    fn execute_command(&mut self, command: &ConsoleCmd) {
        match &command[..] {
            // Used to bind a set of commands to a key.
            [ref cmd, key, command] if cmd == "bind" => {
                self.bind(key, &command);
            }
            // Remove the current commands bound to a key.
            [ref cmd, key] if cmd == "unbind" => {
                self.unbind(&key);
            }
            // Remove every key binding.
            [ref cmd] if cmd == "unbindall" => {
                self.unbind_all();
            }
            _ => (),
        }
    }

    fn from_key_code(key_code: KeyCode) -> Option<&'static str> {
        match key_code {
            KeyCode::Backquote => Some("`"),
            KeyCode::Backslash => Some("\\"),
            KeyCode::BracketLeft => Some("["),
            KeyCode::BracketRight => Some("]"),
            KeyCode::Comma => Some(","),
            KeyCode::Digit0 => Some("0"),
            KeyCode::Digit1 => Some("1"),
            KeyCode::Digit2 => Some("2"),
            KeyCode::Digit3 => Some("3"),
            KeyCode::Digit4 => Some("4"),
            KeyCode::Digit5 => Some("5"),
            KeyCode::Digit6 => Some("6"),
            KeyCode::Digit7 => Some("7"),
            KeyCode::Digit8 => Some("8"),
            KeyCode::Digit9 => Some("9"),
            KeyCode::Equal => Some("="),
            KeyCode::KeyA => Some("a"),
            KeyCode::KeyB => Some("b"),
            KeyCode::KeyC => Some("c"),
            KeyCode::KeyD => Some("d"),
            KeyCode::KeyE => Some("e"),
            KeyCode::KeyF => Some("f"),
            KeyCode::KeyG => Some("g"),
            KeyCode::KeyH => Some("h"),
            KeyCode::KeyI => Some("i"),
            KeyCode::KeyJ => Some("j"),
            KeyCode::KeyK => Some("k"),
            KeyCode::KeyL => Some("l"),
            KeyCode::KeyM => Some("m"),
            KeyCode::KeyN => Some("n"),
            KeyCode::KeyO => Some("o"),
            KeyCode::KeyP => Some("p"),
            KeyCode::KeyQ => Some("q"),
            KeyCode::KeyR => Some("r"),
            KeyCode::KeyS => Some("s"),
            KeyCode::KeyT => Some("t"),
            KeyCode::KeyU => Some("u"),
            KeyCode::KeyV => Some("v"),
            KeyCode::KeyW => Some("w"),
            KeyCode::KeyX => Some("x"),
            KeyCode::KeyY => Some("y"),
            KeyCode::KeyZ => Some("z"),
            KeyCode::Minus => Some("-"),
            KeyCode::Period => Some("."),
            KeyCode::Quote => Some("'"),
            KeyCode::Semicolon => Some(";"),
            KeyCode::Slash => Some("/"),
            KeyCode::AltLeft => Some("alt"),
            KeyCode::AltRight => Some("alt"),
            KeyCode::Backspace => Some("backspace"),
            KeyCode::ControlLeft => Some("ctrl"),
            KeyCode::ControlRight => Some("ctrl"),
            KeyCode::Enter => Some("enter"),
            KeyCode::SuperLeft => Some("command"),
            KeyCode::SuperRight => Some("command"),
            KeyCode::ShiftLeft => Some("shift"),
            KeyCode::ShiftRight => Some("shift"),
            KeyCode::Space => Some("space"),
            KeyCode::Tab => Some("tab"),
            KeyCode::End => Some("end"),
            KeyCode::Home => Some("home"),
            KeyCode::PageDown => Some("pgdn"),
            KeyCode::PageUp => Some("pgup"),
            KeyCode::ArrowDown => Some("downarrow"),
            KeyCode::ArrowLeft => Some("leftarrow"),
            KeyCode::ArrowRight => Some("rightarrow"),
            KeyCode::ArrowUp => Some("uparrow"),
            KeyCode::Escape => Some("escape"),
            KeyCode::F1 => Some("f1"),
            KeyCode::F2 => Some("f2"),
            KeyCode::F3 => Some("f3"),
            KeyCode::F4 => Some("f4"),
            KeyCode::F5 => Some("f5"),
            KeyCode::F6 => Some("f6"),
            KeyCode::F7 => Some("f7"),
            KeyCode::F8 => Some("f8"),
            KeyCode::F9 => Some("f9"),
            KeyCode::F10 => Some("f10"),
            KeyCode::F11 => Some("f11"),
            KeyCode::F12 => Some("f12"),
            _ => None,
        }
    }

    fn from_mouse_button(mouse_button: MouseButton) -> Option<&'static str> {
        match mouse_button {
            MouseButton::Left => Some("mouse1"),
            MouseButton::Right => Some("mouse2"),
            MouseButton::Middle => Some("mouse3"),
            _ => None,
        }
    }

    fn from_mouse_scroll_delta(mouse_wheel: MouseScrollDelta) -> Option<&'static str> {
        match mouse_wheel {
            MouseScrollDelta::LineDelta(_, y) => {
                if y > 0.0 {
                    Some("mwheelup")
                } else {
                    Some("mwheeldown")
                }
            }
            MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => {
                if y > 0.0 {
                    Some("mwheelup")
                } else {
                    Some("mwheeldown")
                }
            }
        }
    }
}

#[system]
pub fn input_handler(
    #[resource] input_event: &Option<InputEvent>,
    #[resource] input: &Input,
    #[resource] console: &mut Console,
) {
    if let Some(input_event) = input_event {
        input
            .handle_input_event(*input_event)
            .map(|action| console.push_command(&action));
    }
}

#[system]
pub fn input_command_executor(#[resource] input: &mut Input, #[resource] console: &mut Console) {
    console
        .commands()
        .for_each(|command| input.execute_command(command));
}
