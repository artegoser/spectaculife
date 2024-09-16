pub mod air_cell;
pub mod life_cell;
pub mod soil_cell;

use air_cell::AirCell;
use life_cell::LifeCell;
use soil_cell::SoilCell;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldCell {
    pub life: LifeCell,
    pub soil: SoilCell,
    pub air: AirCell,
}
