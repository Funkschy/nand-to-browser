use crate::definitions::*;
use std::collections::HashMap;

lazy_static! {
    static ref ACTION_KEY_CODES: HashMap<&'static str, Word> = {
        let mut map = HashMap::new();
        map.insert("PageUp", PAGE_UP_KEY);
        map.insert("PageDown", PAGE_DOWN_KEY);
        map.insert("End", END_KEY);
        map.insert("Home", HOME_KEY);
        map.insert("ArrowLeft", LEFT_KEY);
        map.insert("ArrowUp", UP_KEY);
        map.insert("ArrowRight", RIGHT_KEY);
        map.insert("ArrowDown", DOWN_KEY);
        map.insert("F1", F1_KEY);
        map.insert("F2", F2_KEY);
        map.insert("F3", F3_KEY);
        map.insert("F4", F4_KEY);
        map.insert("F5", F5_KEY);
        map.insert("F6", F6_KEY);
        map.insert("F7", F7_KEY);
        map.insert("F8", F8_KEY);
        map.insert("F9", F9_KEY);
        map.insert("F10", F10_KEY);
        map.insert("F11", F11_KEY);
        map.insert("F12", F12_KEY);
        map.insert("Insert", INSERT_KEY);
        map.insert("Backspace", BACKSPACE_KEY);
        map.insert("Enter", NEWLINE_KEY);
        map.insert("Escape", ESC_KEY);
        map.insert("Delete", DELETE_KEY);
        map
    };
}

pub fn get_special_code(letter: &str) -> Option<Word> {
    get_key_code(letter, 0)
}

pub fn get_key_code(letter: &str, keycode: i32) -> Option<Word> {
    if letter.len() <= 1 {
        let b: u8 = keycode.try_into().ok()?;
        let c: char = b.try_into().ok()?;
        Some(c.to_ascii_uppercase() as Word)
    } else {
        ACTION_KEY_CODES.get(letter).copied()
    }
}