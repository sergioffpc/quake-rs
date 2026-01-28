use crate::bindings::Bindings;
use crate::mappings::Mappings;
use indexmap::IndexSet;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::Instant;
use std::vec::Drain;

mod bindings;
mod mappings;

#[derive(Default)]
pub struct InputManager {
    bindings: Bindings,
    mappings: Mappings,
    pressed_sources: IndexSet<TimedSource>,
    intents: Vec<Intent>,
}

impl InputManager {
    pub fn with_bindings<P>(mut self, path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let bindings_content = fs::read_to_string(path)?;
        self.bindings = Bindings::from_str(&bindings_content)?;
        Ok(self)
    }

    pub fn with_mappings<P>(mut self, path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mappings_content = fs::read_to_string(path)?;
        self.mappings = Mappings::from_str(&mappings_content)?;
        Ok(self)
    }

    pub fn drain(&mut self) -> Drain<'_, Intent> {
        self.intents.drain(..)
    }

    pub fn on_pressed(&mut self, source: Source) {
        let source = self.mappings.get(source);
        self.pressed_sources.insert(TimedSource {
            source,
            timestamp: Instant::now(),
        });

        if let Some(intent) = self.bindings.evaluate(&self.pressed_sources) {
            self.intents.push(intent);
        }
    }

    pub fn on_released(&mut self, source: Source) {
        let source = self.mappings.get(source);
        self.pressed_sources
            .retain(|TimedSource { source: s, .. }| s != &source);
    }

    pub fn on_motion(&mut self, x: f64, y: f64) {
        //TODO
    }

    pub fn on_scroll(&mut self, x: f64, y: f64) {
        //TODO
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Intent(pub String);

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Key(Key),
    Button(Button),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct TimedSource {
    source: Source,
    timestamp: Instant,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub enum Key {
    Backquote,
    Backslash,
    BracketLeft,
    BracketRight,
    Comma,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Equal,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Minus,
    Period,
    Quote,
    Semicolon,
    Slash,
    AltLeft,
    AltRight,
    Backspace,
    ControlLeft,
    ControlRight,
    Enter,
    SuperLeft,
    SuperRight,
    ShiftLeft,
    ShiftRight,
    Space,
    Tab,
    End,
    Home,
    PageDown,
    PageUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub enum Button {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}
