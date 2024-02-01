pub fn f32_from_le_f16_bytes(byte_0: u8, byte_1: u8) -> f32 {
    let sign: u8 = byte_1 & 0b1000_0000;
    let exponent: u8 = ((byte_1 & 0b0111_1100) >> 2) + 0b01110000;
    let fration_l: u8 = (byte_1 & 0b0000_0011) << 5 | byte_0 >> 3;
    let fration_r: u8 = byte_0 << 5;

    f32::from_le_bytes([
        0,
        fration_r,
        exponent << 7 | fration_l,
        sign | exponent >> 1,
    ])
}
