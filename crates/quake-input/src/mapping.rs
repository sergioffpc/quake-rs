use std::collections::HashMap;

#[derive(Debug)]
pub struct Mappings {
    mappings: HashMap<String, String>,
}

impl Mappings {
    pub fn map(&mut self, key: &str, map: &str) {
        self.mappings.insert(key.to_string(), map.to_string());
    }

    pub fn unmap(&mut self, key: &str) {
        self.mappings.remove(key);
    }

    pub fn get(&self, key: &str) -> String {
        self.mappings
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    pub fn clear(&mut self) {
        self.mappings.clear();
    }
}

impl Default for Mappings {
    fn default() -> Self {
        let mut key_mapping = HashMap::new();
        key_mapping.insert("\t".to_string(), "TAB".to_string());
        key_mapping.insert("\r".to_string(), "ENTER".to_string());
        key_mapping.insert("\u{1b}".to_string(), "ESCAPE".to_string());
        key_mapping.insert(" ".to_string(), "SPACE".to_string());
        key_mapping.insert("\u{8}".to_string(), "BACKSPACE".to_string());
        key_mapping.insert("ArrowUp".to_string(), "UPARROW".to_string());
        key_mapping.insert("ArrowDown".to_string(), "DOWNARROW".to_string());
        key_mapping.insert("ArrowLeft".to_string(), "LEFTARROW".to_string());
        key_mapping.insert("ArrowRight".to_string(), "RIGHTARROW".to_string());

        key_mapping.insert("AltLeft".to_string(), "ALT".to_string());
        key_mapping.insert("AltRight".to_string(), "ALT".to_string());
        key_mapping.insert("ControlLeft".to_string(), "CTRL".to_string());
        key_mapping.insert("ControlRight".to_string(), "CTRL".to_string());
        key_mapping.insert("ShiftLeft".to_string(), "SHIFT".to_string());
        key_mapping.insert("ShiftRight".to_string(), "SHIFT".to_string());

        key_mapping.insert("Insert".to_string(), "INS".to_string());
        key_mapping.insert("Delete".to_string(), "DEL".to_string());
        key_mapping.insert("PageDown".to_string(), "PGDN".to_string());
        key_mapping.insert("PageUp".to_string(), "PGUP".to_string());
        key_mapping.insert("Home".to_string(), "HOME".to_string());
        key_mapping.insert("End".to_string(), "END".to_string());

        key_mapping.insert("ButtonLeft".to_string(), "MOUSE1".to_string());
        key_mapping.insert("ButtonRight".to_string(), "MOUSE2".to_string());
        key_mapping.insert("ButtonMiddle".to_string(), "MOUSE3".to_string());

        key_mapping.insert("Pause".to_string(), "PAUSE".to_string());

        key_mapping.insert("ScrollUp".to_string(), "MWHEELUP".to_string());
        key_mapping.insert("ScrollDown".to_string(), "MWHEELDOWN".to_string());

        Self {
            mappings: key_mapping,
        }
    }
}
