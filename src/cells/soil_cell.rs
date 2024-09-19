pub const MAX_ORGANIC_LIFE: u8 = 16;
pub const MAX_ENERGY_LIFE: f32 = 32.;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SoilCell {
    pub organics: u8,
    pub energy: f32,
}
