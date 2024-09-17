#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SoilCell {
    pub organic: u8,
    pub energy: f32,
}

impl SoilCell {
    pub const fn texture_id(&self) -> u32 {
        match self.organic {
            0..=1 => 0,
            2..=5 => 1,
            6..=16 => 3,
            17..=128 => 4,
            129..=160 => 5,
            161..=192 => 6,
            193..=224 => 7,
            _ => 8,
        }
    }
}
