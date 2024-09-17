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
        match self.ty {
            LifeType::Cancer => 7,
        }
    }

    pub const fn consumption(&self) -> f32 {
        self.ty.consumption()
    }

    pub fn energy_flow(&self) -> f32 {
        match self.ty {
            LifeType::Cancer => self.energy * 0.25,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeType {
    // Pipe(PipeCell),
    Cancer,
}

impl LifeType {
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

impl EnergyDirections {
    pub const fn branches_amount(&self) -> u8 {
        let mut total = 0;

        if self.up {
            total += 1
        };

        if self.down {
            total += 1
        };

        if self.left {
            total += 1
        };

        if self.right {
            total += 1
        };

        total
    }
}
