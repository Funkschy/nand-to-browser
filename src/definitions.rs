// see Definitions.java int the official implementation
pub const MEM_SIZE: usize = 24577;
pub const KBD: usize = 24576;

pub const BITS_PER_WORD: usize = 16;
pub const SCREEN_WIDTH_IN_WORDS: usize = 32;
pub const SCREEN_HEIGTH_IN_WORDS: usize = 256;
pub const SCREEN_WIDTH: usize = SCREEN_WIDTH_IN_WORDS * BITS_PER_WORD;
pub const SCREEN_HEIGHT: usize = SCREEN_HEIGTH_IN_WORDS;
pub const SCREEN_SIZE_IN_WORDS: usize = SCREEN_WIDTH_IN_WORDS * SCREEN_HEIGTH_IN_WORDS;
pub const SCREEN_START: usize = 16384;
pub const SCREEN_END: usize = SCREEN_START + SCREEN_SIZE_IN_WORDS - 1;

pub const SP: usize = 0;
pub const LCL: usize = 1;
pub const ARG: usize = 2;
pub const THIS: usize = 3;
pub const THAT: usize = 4;

// a position in the bytecode
pub type Symbol = u16;
// an address in the simulated RAM
pub type Address = usize;
// a register/memory-cell value in the hack architecture
pub type Word = i16;

pub const INIT_SP: Word = 256;

pub const HEAP_START: usize = 2048;
pub const HEAP_END: usize = 16383;
pub const NEWLINE_KEY: Word = 128;
pub const BACKSPACE_KEY: Word = 129;
