pub fn ratio_to_u8(i: f32) -> u8 {
    ((i - 0.01) / (1.00 - 0.01) * 255.0) as u8
}
