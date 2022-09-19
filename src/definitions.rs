pub const MEM_SIZE: usize = 24576;
pub const SCREEN_START: usize = 16384;
pub const KBD: usize = 24576;

pub const SP: usize = 0;
pub const LCL: usize = 1;
pub const ARG: usize = 2;
pub const THIS: usize = 3;
pub const THAT: usize = 4;

pub type Address = usize;
pub type Word = i16;
