use crate::{grid::Area, types::CellDir, utils::merge_energy};

use super::WorldCell;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LifeCell {
    Alive(AliveCell),

    #[default]
    Dead,
}

impl LifeCell {
    pub const fn texture_id(&self, area: Area<WorldCell>) -> u32 {
        match self {
            Self::Alive(alive_life_cell) => alive_life_cell.texture_id(area),
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

    pub const fn texture_id(&self, area: Area<WorldCell>) -> u32 {
        match self.ty {
            LifeType::Pipe => match merge_energy(&area, self.energy_to) {
                (false, false, false, false) => 7,
                (true, true, false, false) => 8,
                (false, false, true, true) => 9,
                (true, true, true, true) => 10,
                (false, true, true, false) => 11,
                (true, false, true, false) => 12,
                (true, false, false, true) => 13,
                (false, true, false, true) => 14,
                (false, true, true, true) => 15,
                (true, true, true, false) => 16,
                (true, false, true, true) => 17,
                (true, true, false, true) => 18,
                (false, true, false, false) => 19,
                (false, false, true, false) => 20,
                (true, false, false, false) => 21,
                (false, false, false, true) => 22,
            },
            LifeType::Leaf => 2,
            LifeType::Cancer => 6,
        }
    }

    pub const fn consumption(&self) -> f32 {
        self.ty.consumption()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeType {
    Pipe,
    Leaf,
    Cancer,
}

impl LifeType {
    pub const fn consumption(&self) -> f32 {
        match self {
            LifeType::Pipe => 0.1,
            LifeType::Leaf => 0.5,
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
    pub const fn from_direction(dir: &CellDir) -> Self {
        match dir {
            CellDir::Up => EnergyDirections {
                up: true,
                down: false,
                left: false,
                right: false,
            },
            CellDir::Down => EnergyDirections {
                up: false,
                down: true,
                left: false,
                right: false,
            },
            CellDir::Left => EnergyDirections {
                up: false,
                down: false,
                left: true,
                right: false,
            },
            CellDir::Right => EnergyDirections {
                up: false,
                down: false,
                left: false,
                right: true,
            },
        }
    }

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

    pub const fn to_tuple(&self) -> (bool, bool, bool, bool) {
        (self.up, self.down, self.left, self.right)
    }
}
