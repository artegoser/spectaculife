pub const MAX_ORGANIC_LIFE: u8 = 16;
const M: u8 = MAX_ORGANIC_LIFE + 1;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SoilCell {
    pub organic: u8,
    pub energy: f32,
}
