pub fn split_i16(value: i16) -> (u8, u8) {
    let values = value.to_le_bytes();
    (values[0], values[1])
}

pub fn split_u16(value: u16) -> (u8, u8) {
    let values = value.to_le_bytes();
    (values[0], values[1])
}
