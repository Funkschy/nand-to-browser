use super::StdlibError::{
    OutputBlockedAddressMutex, OutputBlockedFirstInWordMutex, OutputBlockedWordInLineMutex,
};
use super::*;
use crate::definitions::{
    Word, BACKSPACE_KEY, NEWLINE_KEY, SCREEN_HEIGHT, SCREEN_START, SCREEN_WIDTH,
};
use crate::simulators::vm::VM;
use lazy_static::lazy_static;
use std::sync::Mutex;

const N_COLS: usize = SCREEN_WIDTH / 8;
const N_ROWS: usize = SCREEN_HEIGHT / 11;
const START_ADDRESS: usize = SCREEN_WIDTH >> 4;

// at the point of writing this, wasm has no atomics support, so we have to use a mutexe instead
lazy_static! {
    static ref WORD_IN_LINE: Mutex<usize> = Mutex::new(0);
    static ref ADDRESS: Mutex<usize> = Mutex::new(START_ADDRESS);
    static ref FIRST_IN_WORD: Mutex<bool> = Mutex::new(true);
}

lazy_static! {
    static ref MAP: HashMap<u32, &'static [Word; 11]> = {
        let mut map = HashMap::new();

        map.insert(0, &[63, 63, 63, 63, 63, 63, 63, 63, 63, 0, 0]);
        map.insert(32, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        map.insert(33, &[12, 30, 30, 30, 12, 12, 0, 12, 12, 0, 0]);
        map.insert(34, &[54, 54, 20, 0, 0, 0, 0, 0, 0, 0, 0]);
        map.insert(35, &[0, 18, 18, 63, 18, 18, 63, 18, 18, 0, 0]);
        map.insert(36, &[12, 30, 51, 3, 30, 48, 51, 30, 12, 12, 0]);
        map.insert(37, &[0, 0, 35, 51, 24, 12, 6, 51, 49, 0, 0]);
        map.insert(38, &[12, 30, 30, 12, 54, 27, 27, 27, 54, 0, 0]);
        map.insert(39, &[12, 12, 6, 0, 0, 0, 0, 0, 0, 0, 0]);
        map.insert(40, &[24, 12, 6, 6, 6, 6, 6, 12, 24, 0, 0]);
        map.insert(41, &[6, 12, 24, 24, 24, 24, 24, 12, 6, 0, 0]);
        map.insert(42, &[0, 0, 0, 51, 30, 63, 30, 51, 0, 0, 0]);
        map.insert(43, &[0, 0, 0, 12, 12, 63, 12, 12, 0, 0, 0]);
        map.insert(44, &[0, 0, 0, 0, 0, 0, 0, 12, 12, 6, 0]);
        map.insert(45, &[0, 0, 0, 0, 0, 63, 0, 0, 0, 0, 0]);
        map.insert(46, &[0, 0, 0, 0, 0, 0, 0, 12, 12, 0, 0]);
        map.insert(47, &[0, 0, 32, 48, 24, 12, 6, 3, 1, 0, 0]);
        map.insert(48, &[12, 30, 51, 51, 51, 51, 51, 30, 12, 0, 0]);
        map.insert(49, &[12, 14, 15, 12, 12, 12, 12, 12, 63, 0, 0]);
        map.insert(50, &[30, 51, 48, 24, 12, 6, 3, 51, 63, 0, 0]);
        map.insert(51, &[30, 51, 48, 48, 28, 48, 48, 51, 30, 0, 0]);
        map.insert(52, &[16, 24, 28, 26, 25, 63, 24, 24, 60, 0, 0]);
        map.insert(53, &[63, 3, 3, 31, 48, 48, 48, 51, 30, 0, 0]);
        map.insert(54, &[28, 6, 3, 3, 31, 51, 51, 51, 30, 0, 0]);
        map.insert(55, &[63, 49, 48, 48, 24, 12, 12, 12, 12, 0, 0]);
        map.insert(56, &[30, 51, 51, 51, 30, 51, 51, 51, 30, 0, 0]);
        map.insert(57, &[30, 51, 51, 51, 62, 48, 48, 24, 14, 0, 0]);
        map.insert(58, &[0, 0, 12, 12, 0, 0, 12, 12, 0, 0, 0]);
        map.insert(59, &[0, 0, 12, 12, 0, 0, 12, 12, 6, 0, 0]);
        map.insert(60, &[0, 0, 24, 12, 6, 3, 6, 12, 24, 0, 0]);
        map.insert(61, &[0, 0, 0, 63, 0, 0, 63, 0, 0, 0, 0]);
        map.insert(62, &[0, 0, 3, 6, 12, 24, 12, 6, 3, 0, 0]);
        map.insert(64, &[30, 51, 51, 59, 59, 59, 27, 3, 30, 0, 0]);
        map.insert(63, &[30, 51, 51, 24, 12, 12, 0, 12, 12, 0, 0]);
        map.insert(65, &[12, 30, 51, 51, 63, 51, 51, 51, 51, 0, 0]);
        map.insert(66, &[31, 51, 51, 51, 31, 51, 51, 51, 31, 0, 0]);
        map.insert(67, &[28, 54, 35, 3, 3, 3, 35, 54, 28, 0, 0]);
        map.insert(68, &[15, 27, 51, 51, 51, 51, 51, 27, 15, 0, 0]);
        map.insert(69, &[63, 51, 35, 11, 15, 11, 35, 51, 63, 0, 0]);
        map.insert(70, &[63, 51, 35, 11, 15, 11, 3, 3, 3, 0, 0]);
        map.insert(71, &[28, 54, 35, 3, 59, 51, 51, 54, 44, 0, 0]);
        map.insert(72, &[51, 51, 51, 51, 63, 51, 51, 51, 51, 0, 0]);
        map.insert(73, &[30, 12, 12, 12, 12, 12, 12, 12, 30, 0, 0]);
        map.insert(74, &[60, 24, 24, 24, 24, 24, 27, 27, 14, 0, 0]);
        map.insert(75, &[51, 51, 51, 27, 15, 27, 51, 51, 51, 0, 0]);
        map.insert(76, &[3, 3, 3, 3, 3, 3, 35, 51, 63, 0, 0]);
        map.insert(77, &[33, 51, 63, 63, 51, 51, 51, 51, 51, 0, 0]);
        map.insert(78, &[51, 51, 55, 55, 63, 59, 59, 51, 51, 0, 0]);
        map.insert(79, &[30, 51, 51, 51, 51, 51, 51, 51, 30, 0, 0]);
        map.insert(80, &[31, 51, 51, 51, 31, 3, 3, 3, 3, 0, 0]);
        map.insert(81, &[30, 51, 51, 51, 51, 51, 63, 59, 30, 48, 0]);
        map.insert(82, &[31, 51, 51, 51, 31, 27, 51, 51, 51, 0, 0]);
        map.insert(83, &[30, 51, 51, 6, 28, 48, 51, 51, 30, 0, 0]);
        map.insert(84, &[63, 63, 45, 12, 12, 12, 12, 12, 30, 0, 0]);
        map.insert(85, &[51, 51, 51, 51, 51, 51, 51, 51, 30, 0, 0]);
        map.insert(86, &[51, 51, 51, 51, 51, 30, 30, 12, 12, 0, 0]);
        map.insert(87, &[51, 51, 51, 51, 51, 63, 63, 63, 18, 0, 0]);
        map.insert(88, &[51, 51, 30, 30, 12, 30, 30, 51, 51, 0, 0]);
        map.insert(89, &[51, 51, 51, 51, 30, 12, 12, 12, 30, 0, 0]);
        map.insert(90, &[63, 51, 49, 24, 12, 6, 35, 51, 63, 0, 0]);
        map.insert(91, &[30, 6, 6, 6, 6, 6, 6, 6, 30, 0, 0]);
        map.insert(92, &[0, 0, 1, 3, 6, 12, 24, 48, 32, 0, 0]);
        map.insert(93, &[30, 24, 24, 24, 24, 24, 24, 24, 30, 0, 0]);
        map.insert(94, &[8, 28, 54, 0, 0, 0, 0, 0, 0, 0, 0]);
        map.insert(95, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 63, 0]);
        map.insert(96, &[6, 12, 24, 0, 0, 0, 0, 0, 0, 0, 0]);
        map.insert(97, &[0, 0, 0, 14, 24, 30, 27, 27, 54, 0, 0]);
        map.insert(98, &[3, 3, 3, 15, 27, 51, 51, 51, 30, 0, 0]);
        map.insert(99, &[0, 0, 0, 30, 51, 3, 3, 51, 30, 0, 0]);
        map.insert(100, &[48, 48, 48, 60, 54, 51, 51, 51, 30, 0, 0]);
        map.insert(101, &[0, 0, 0, 30, 51, 63, 3, 51, 30, 0, 0]);
        map.insert(102, &[28, 54, 38, 6, 15, 6, 6, 6, 15, 0, 0]);
        map.insert(103, &[0, 0, 30, 51, 51, 51, 62, 48, 51, 30, 0]);
        map.insert(104, &[3, 3, 3, 27, 55, 51, 51, 51, 51, 0, 0]);
        map.insert(105, &[12, 12, 0, 14, 12, 12, 12, 12, 30, 0, 0]);
        map.insert(106, &[48, 48, 0, 56, 48, 48, 48, 48, 51, 30, 0]);
        map.insert(107, &[3, 3, 3, 51, 27, 15, 15, 27, 51, 0, 0]);
        map.insert(108, &[14, 12, 12, 12, 12, 12, 12, 12, 30, 0, 0]);
        map.insert(109, &[0, 0, 0, 29, 63, 43, 43, 43, 43, 0, 0]);
        map.insert(110, &[0, 0, 0, 29, 51, 51, 51, 51, 51, 0, 0]);
        map.insert(111, &[0, 0, 0, 30, 51, 51, 51, 51, 30, 0, 0]);
        map.insert(112, &[0, 0, 0, 30, 51, 51, 51, 31, 3, 3, 0]);
        map.insert(113, &[0, 0, 0, 30, 51, 51, 51, 62, 48, 48, 0]);
        map.insert(114, &[0, 0, 0, 29, 55, 51, 3, 3, 7, 0, 0]);
        map.insert(115, &[0, 0, 0, 30, 51, 6, 24, 51, 30, 0, 0]);
        map.insert(116, &[4, 6, 6, 15, 6, 6, 6, 54, 28, 0, 0]);
        map.insert(117, &[0, 0, 0, 27, 27, 27, 27, 27, 54, 0, 0]);
        map.insert(118, &[0, 0, 0, 51, 51, 51, 51, 30, 12, 0, 0]);
        map.insert(119, &[0, 0, 0, 51, 51, 51, 63, 63, 18, 0, 0]);
        map.insert(120, &[0, 0, 0, 51, 30, 12, 12, 30, 51, 0, 0]);
        map.insert(121, &[0, 0, 0, 51, 51, 51, 62, 48, 24, 15, 0]);
        map.insert(122, &[0, 0, 0, 63, 27, 12, 6, 51, 63, 0, 0]);
        map.insert(123, &[56, 12, 12, 12, 7, 12, 12, 12, 56, 0, 0]);
        map.insert(124, &[12, 12, 12, 12, 12, 12, 12, 12, 12, 0, 0]);
        map.insert(125, &[7, 12, 12, 12, 56, 12, 12, 12, 7, 0, 0]);
        map.insert(126, &[38, 45, 25, 0, 0, 0, 0, 0, 0, 0, 0]);

        map
    };
}

pub fn init(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    set_mutex!(WORD_IN_LINE, 0, OutputBlockedWordInLineMutex);
    set_mutex!(ADDRESS, START_ADDRESS, OutputBlockedAddressMutex);
    set_mutex!(FIRST_IN_WORD, true, OutputBlockedFirstInWordMutex);
    Ok(StdlibOk::Finished(0))
}

fn draw_char(vm: &mut VM, c: char) -> Result<(), StdlibError> {
    let c = c as u32;
    let c = if !(32..127).contains(&c) { 0 } else { c };

    let first_in_word = get_mutex!(FIRST_IN_WORD, OutputBlockedFirstInWordMutex);
    let (mask, shift) = if first_in_word {
        (0xFF00u16 as i16, 0)
    } else {
        (0x00FF, 8)
    };

    let mut j = get_mutex!(ADDRESS, OutputBlockedAddressMutex);
    for i in 0..11 {
        let old_value = vm.mem(SCREEN_START + j)?;
        let map_value = MAP[&c][i];
        let new_value = (old_value & mask) | (map_value << shift);
        vm.set_mem(SCREEN_START + j, new_value)?;
        j += SCREEN_WIDTH >> 4;
    }

    Ok(())
}

fn println_impl() -> Result<(), StdlibError> {
    let address = get_mutex!(ADDRESS, OutputBlockedAddressMutex);
    let word_in_line = get_mutex!(WORD_IN_LINE, OutputBlockedWordInLineMutex);

    let new_address = (address + 11 * (SCREEN_WIDTH >> 4)) - word_in_line;

    let a = if new_address == START_ADDRESS + N_ROWS * 11 * (SCREEN_WIDTH >> 4) {
        START_ADDRESS
    } else {
        new_address
    };

    set_mutex!(WORD_IN_LINE, 0, OutputBlockedWordInLineMutex);
    set_mutex!(FIRST_IN_WORD, true, OutputBlockedFirstInWordMutex);
    set_mutex!(ADDRESS, a, OutputBlockedAddressMutex);

    Ok(())
}

fn backspace_impl(vm: &mut VM) -> Result<(), StdlibError> {
    let mut address = get_mutex!(ADDRESS, OutputBlockedAddressMutex);
    let mut word_in_line = get_mutex!(WORD_IN_LINE, OutputBlockedWordInLineMutex);
    let mut first_in_word = get_mutex!(FIRST_IN_WORD, OutputBlockedFirstInWordMutex);

    if first_in_word {
        if word_in_line > 0 {
            word_in_line -= 1;
            address -= 1;
        } else {
            word_in_line = (SCREEN_WIDTH >> 4) - 1;
            if address == START_ADDRESS {
                address = START_ADDRESS + N_ROWS * 11 * (SCREEN_WIDTH >> 4);
            }
            address -= 10 * (SCREEN_WIDTH >> 4) + 1;
        }
        first_in_word = false;
    } else {
        first_in_word = true;
    }

    set_mutex!(ADDRESS, address, OutputBlockedAddressMutex);
    set_mutex!(WORD_IN_LINE, word_in_line, OutputBlockedWordInLineMutex);
    set_mutex!(FIRST_IN_WORD, first_in_word, OutputBlockedFirstInWordMutex);

    draw_char(vm, ' ')
}

fn print_char_impl(vm: &mut VM, c: Word) -> Result<(), StdlibError> {
    match c {
        NEWLINE_KEY => println_impl()?,
        BACKSPACE_KEY => backspace_impl(vm)?,
        _ => {
            draw_char(vm, c as u8 as char)?;

            let mut address = get_mutex!(ADDRESS, OutputBlockedAddressMutex);
            let mut word_in_line = get_mutex!(WORD_IN_LINE, OutputBlockedWordInLineMutex);
            let mut first_in_word = get_mutex!(FIRST_IN_WORD, OutputBlockedFirstInWordMutex);

            if !first_in_word {
                word_in_line += 1;
                address += 1;
                if word_in_line == SCREEN_WIDTH >> 4 {
                    set_mutex!(ADDRESS, address, OutputBlockedAddressMutex);
                    set_mutex!(WORD_IN_LINE, word_in_line, OutputBlockedWordInLineMutex);
                    set_mutex!(FIRST_IN_WORD, first_in_word, OutputBlockedFirstInWordMutex);
                    println_impl()?;
                    // println can change the values
                    address = get_mutex!(ADDRESS, OutputBlockedAddressMutex);
                    word_in_line = get_mutex!(WORD_IN_LINE, OutputBlockedWordInLineMutex);
                    first_in_word = get_mutex!(FIRST_IN_WORD, OutputBlockedFirstInWordMutex);
                } else {
                    first_in_word = true;
                }
            } else {
                first_in_word = false;
            }

            set_mutex!(ADDRESS, address, OutputBlockedAddressMutex);
            set_mutex!(WORD_IN_LINE, word_in_line, OutputBlockedWordInLineMutex);
            set_mutex!(FIRST_IN_WORD, first_in_word, OutputBlockedFirstInWordMutex);
        }
    }
    Ok(())
}

pub fn move_cursor(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let row = params[0];
    let col = params[1];

    let row_u = row as usize;
    let col_u = col as usize;

    if row < 0 || row_u > N_ROWS || col < 0 || col_u >= N_COLS {
        return Err(StdlibError::OutputMoveCursorIllegalPosition);
    }

    let word_in_line = col_u / 2;
    set_mutex!(WORD_IN_LINE, word_in_line, OutputBlockedWordInLineMutex);

    let a = START_ADDRESS + row_u * 11 * (SCREEN_WIDTH >> 4) + word_in_line;
    set_mutex!(ADDRESS, a, OutputBlockedAddressMutex);

    set_mutex!(FIRST_IN_WORD, col & 1 == 0, OutputBlockedFirstInWordMutex);

    draw_char(vm, ' ')?;
    Ok(StdlibOk::Finished(0))
}

pub fn print_char(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let c = params[0];
    print_char_impl(vm, c)?;

    Ok(StdlibOk::Finished(0))
}

pub fn print_string(vm: &mut VM, state: State, params: &[Word]) -> StdResult {
    // State is 32 bits wide. The upper 16 bits are used to keep the length information, while
    // the lower 16 bits are used for the actual state counter
    let string = params[0];
    let real_state = state & 0xFFFF;

    if state == 0 {
        return call_vm!(vm, state, "String.length", &[string]);
    }

    // the second call of this function is a special case, because the string length is on the
    // stack instead of in the state
    let (len, last) = if state == 1 {
        (vm.pop()? as State, 0)
    } else {
        ((state >> 16) & 0xFFFF, vm.pop()?)
    };

    // we need 2 ticks for each i, so divide by 2
    // also subtract 1 because we will not execute this for state == 0
    let i = (real_state as Word - 1) / 2;

    // our exit condition
    if i as State >= len {
        return Ok(StdlibOk::Finished(0));
    }

    // keep alternating between these 2 while i < len
    if state % 2 == 1 {
        // on uneven states call charAt
        vm.call("String.charAt", &[string, i])?;
        Ok(StdlibOk::ContinueInNextStep((len << 16) | (state + 1)))
    } else {
        // on even states we actually print
        vm.call("Output.printChar", &[last])?;
        Ok(StdlibOk::ContinueInNextStep((len << 16) | (state + 1)))
    }
}

pub fn print_int(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let i = params[0];
    let s = i.to_string();

    for c in s.chars().take_while(|&c| c == '-' || c.is_ascii_digit()) {
        print_char_impl(vm, c as Word)?;
    }

    Ok(StdlibOk::Finished(0))
}

pub fn println(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    println_impl()?;
    Ok(StdlibOk::Finished(0))
}

pub fn backspace(vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    backspace_impl(vm)?;
    Ok(StdlibOk::Finished(0))
}
