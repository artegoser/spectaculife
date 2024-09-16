use crate::types::CellDir;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LifeCell {
    Alive(AliveCell),

    #[default]
    Dead,
}

impl LifeCell {
    pub const fn texture_id(&self) -> u32 {
        match self {
            Self::Alive(alive_life_cell) => alive_life_cell.texture_id(),
            Self::Dead => 0,
        }
    }

    pub const fn energy(&self) -> f32 {
        match self {
            Self::Alive(alive_cell) => alive_cell.energy,
            Self::Dead => 0.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AliveCell {
    pub ty: LifeType,

    pub energy: f32,

    pub energy_to: EnergyDirections,
    pub parent: Option<CellDir>,
}

impl AliveCell {
    pub fn new(
        ty: LifeType,
        energy: f32,
        parent: Option<CellDir>,
        energy_to: EnergyDirections,
    ) -> Self {
        Self {
            ty,
            energy,

            energy_to,
            parent,
        }
    }

    pub const fn texture_id(&self) -> u32 {
        self.ty.texture_id()
    }

    pub const fn consumption(&self) -> f32 {
        self.ty.consumption()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeType {
    // Pipe(PipeCell),
    Cancer,
}

impl LifeType {
    pub const fn texture_id(&self) -> u32 {
        match self {
            LifeType::Cancer => 7,
        }
    }

    pub const fn consumption(&self) -> f32 {
        match self {
            LifeType::Cancer => 1.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnergyDirections {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}
