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
            6..=16 => 2,
            17..=128 => 3,
            129..=160 => 4,
            161..=192 => 5,
            193..=224 => 6,
            _ => 7,
        }
    }
}
