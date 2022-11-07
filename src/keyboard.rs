use crate::definitions::{Word, BACKSPACE_KEY, NEWLINE_KEY};
use std::collections::HashMap;

use lazy_static::lazy_static;

const SPACE_KEY: Word = 32;
const LEFT_KEY: Word = 130;
const UP_KEY: Word = 131;
const RIGHT_KEY: Word = 132;
const DOWN_KEY: Word = 133;
const HOME_KEY: Word = 134;
const END_KEY: Word = 135;
const PAGE_UP_KEY: Word = 136;
const PAGE_DOWN_KEY: Word = 137;
const INSERT_KEY: Word = 138;
const DELETE_KEY: Word = 139;
const ESC_KEY: Word = 140;
const F1_KEY: Word = 141;
const F2_KEY: Word = 142;
const F3_KEY: Word = 143;
const F4_KEY: Word = 144;
const F5_KEY: Word = 145;
const F6_KEY: Word = 146;
const F7_KEY: Word = 147;
const F8_KEY: Word = 148;
const F9_KEY: Word = 149;
const F10_KEY: Word = 150;
const F11_KEY: Word = 151;
const F12_KEY: Word = 152;

lazy_static! {
    static ref ACTION_KEY_CODES: HashMap<&'static str, Word> = {
        let mut map = HashMap::new();
        map.insert("Space", SPACE_KEY);
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

// this function is only needed when we actually have a UI,
// so the other compilations report it as unused
#[allow(dead_code)]
pub fn get_key_code(letter: &str) -> Option<Word> {
    if letter.len() == 1 {
        letter
            .chars()
            .next()
            // in my opinion, this behaviour is actually a bug in the original emulator
            // a lot of games rely on all keys being uppercase however, so it would be
            // a bad move to change that
            .map(|c| c.to_ascii_uppercase())
            .and_then(|l| l.try_into().ok())
            .and_then(|l: u32| l.try_into().ok())
    } else {
        ACTION_KEY_CODES.get(letter).copied()
    }
}
