use genome::GenomeHandle;

use crate::{
    config::LifeConfig,
    grid::Grid,
    types::CellDir::{self, *},
    utils::merge_energy,
};

use super::WorldCell;


pub mod genome;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LifeCell {
    Alive(AliveCell),

    #[default]
    Dead,
}

impl LifeCell {
    pub fn texture_id(&self, grid: &Grid<WorldCell>, index: usize) -> u32 {
        match self {
            Self::Alive(alive_life_cell) => alive_life_cell.texture_id(grid, index),
            Self::Dead => 16,
        }
    }

    pub const fn energy_directions_texture_id(&self) -> u32 {
        match self {
            Self::Alive(alive_life_cell) => alive_life_cell.energy_directions_texture_id(),
            Self::Dead => 0,
        }
    }

    pub const fn is_fertile(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.is_fertile(),
            Self::Dead => false,
        }
    }

    pub const fn can_transfer(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.can_transfer(),
            Self::Dead => false,
        }
    }

    pub const fn is_pipe_recipient(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.is_pipe_recipient(),
            Self::Dead => false,
        }
    }

    pub const fn is_pipe(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.is_pipe(),
            Self::Dead => false,
        }
    }

    pub const fn is_energy_generator(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.is_energy_generator(),
            Self::Dead => false,
        }
    }

    pub const fn is_alive(&self) -> bool {
        match self {
            Self::Alive(_) => true,
            Self::Dead => false,
        }
    }

    pub const fn energy(&self) -> f32 {
        match self {
            Self::Alive(alive_cell) => alive_cell.energy,
            Self::Dead => 0.,
        }
    }

    pub fn organics(&self, config: &LifeConfig) -> u8 {
        match self {
            Self::Alive(alive_cell) => alive_cell.organics(config),
            Self::Dead => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AliveCell {
    pub ty: LifeType,
    pub organism_id: u64,

    pub energy: f32,
    pub incoming_energy: f32,
    pub energy_to: EnergyDirections,

    pub parent_dir: Option<CellDir>,

    pub steps_to_death: u16,
}

impl AliveCell {
    pub const fn new(
        ty: LifeType,
        organism_id: u64,
        energy: f32,
        energy_to: EnergyDirections,
        parent_dir: Option<CellDir>,
        steps_to_death: u16,
    ) -> Self {
        Self {
            ty,
            organism_id,

            energy,
            incoming_energy: 0.0,
            energy_to,

            parent_dir,

            steps_to_death,
        }
    }

    pub const fn energy_directions_texture_id(&self) -> u32 {
        match self.energy_to.to_tuple() {
            (false, false, false, false) => 0,
            (true, true, false, false) => 1,
            (false, false, true, true) => 2,
            (true, true, true, true) => 3,
            (false, true, true, false) => 4,
            (true, false, true, false) => 5,
            (true, false, false, true) => 6,
            (false, true, false, true) => 7,
            (false, true, true, true) => 8,
            (true, true, true, false) => 9,
            (true, false, true, true) => 10,
            (true, true, false, true) => 11,
            (false, true, false, false) => 12,
            (false, false, true, false) => 13,
            (true, false, false, false) => 14,
            (false, false, false, true) => 15,
        }
    }

    pub fn texture_id(&self, grid: &Grid<WorldCell>, index: usize) -> u32 {
        match self.ty {
            LifeType::Pipe => match merge_energy(grid, index, self.energy_to).to_tuple() {
                (false, false, false, false) => 0,
                (true, true, false, false) => 1,
                (false, false, true, true) => 2,
                (true, true, true, true) => 3,
                (false, true, true, false) => 4,
                (true, false, true, false) => 5,
                (true, false, false, true) => 6,
                (false, true, false, true) => 7,
                (false, true, true, true) => 8,
                (true, true, true, false) => 9,
                (true, false, true, true) => 10,
                (true, true, false, true) => 11,
                (false, true, false, false) => 12,
                (false, false, true, false) => 13,
                (true, false, false, false) => 14,
                (false, false, false, true) => 15,
            },
            LifeType::Leaf => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 22,
                        Down => 23,
                        Left => 24,
                        Right => 25,
                    }
                } else {
                    16
                }
            }
            LifeType::Stem(_) => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 18,
                        Down => 19,
                        Left => 20,
                        Right => 21,
                    }
                } else {
                    17
                }
            }
            LifeType::Root => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 26,
                        Down => 27,
                        Left => 28,
                        Right => 29,
                    }
                } else {
                    16
                }
            }
            LifeType::Reactor => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 30,
                        Down => 31,
                        Left => 32,
                        Right => 33,
                    }
                } else {
                    16
                }
            }
            LifeType::Filter => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 34,
                        Down => 35,
                        Left => 36,
                        Right => 37,
                    }
                } else {
                    16
                }
            }
        }
    }

    pub const fn can_transfer(&self) -> bool {
        self.ty.can_transfer()
    }

    pub const fn is_pipe_recipient(&self) -> bool {
        self.ty.is_pipe_recipient()
    }

    pub const fn is_pipe(&self) -> bool {
        self.ty.is_pipe()
    }

    pub const fn is_energy_generator(&self) -> bool {
        self.ty.is_energy_generator()
    }

    pub const fn is_fertile(&self) -> bool {
        self.ty.is_fertile()
    }

    pub fn consumption(&self, config: &LifeConfig) -> f32 {
        self.ty.consumption(config)
    }

    pub fn organics(&self, config: &LifeConfig) -> u8 {
        self.ty.organics(config)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeType {
    Pipe,
    Leaf,
    Root,
    Reactor,
    Filter,

    Stem(GenomeHandle),
}

impl LifeType {
    pub const fn can_transfer(&self) -> bool {
        self.is_energy_generator() || self.is_pipe()
    }

    pub const fn is_energy_generator(&self) -> bool {
        match self {
            LifeType::Leaf | LifeType::Root | LifeType::Reactor | LifeType::Filter => true,
            _ => false,
        }
    }

    pub const fn is_pipe(&self) -> bool {
        match self {
            LifeType::Pipe => true,
            _ => false,
        }
    }

    pub const fn is_pipe_recipient(&self) -> bool {
        match self {
            LifeType::Pipe | LifeType::Stem(_) => true,
            _ => false,
        }
    }

    pub const fn is_fertile(&self) -> bool {
        match self {
            LifeType::Stem(_) => true,
            _ => false,
        }
    }

    pub fn consumption(&self, config: &LifeConfig) -> f32 {
        match self {
            LifeType::Pipe => config.consumption.pipe,
            LifeType::Leaf => config.consumption.leaf,
            LifeType::Stem(_) => config.consumption.stem,
            LifeType::Root => config.consumption.root,
            LifeType::Reactor => config.consumption.reactor,
            LifeType::Filter => config.consumption.filter,
        }
    }

    pub fn organics(&self, config: &LifeConfig) -> u8 {
        match self {
            LifeType::Pipe => config.organics.pipe,
            LifeType::Leaf => config.organics.leaf,
            LifeType::Stem(_) => config.organics.stem,
            LifeType::Root => config.organics.root,
            LifeType::Reactor => config.organics.reactor,
            LifeType::Filter => config.organics.filter,
        }
    }

    pub fn make_newborn_cell(
        self,
        organism_id: u64,
        parent_dir: CellDir,
        steps_to_death: u16,
        config: &LifeConfig,
    ) -> LifeCell {
        let new_cell_energy_directions = if self.is_energy_generator() {
            EnergyDirections::from_direction(&parent_dir)
        } else {
            EnergyDirections {
                up: false,
                down: false,
                left: false,
                right: false,
            }
        };

        LifeCell::Alive(AliveCell::new(
            self,
            organism_id,
            config.newborn_energy_consumption_multiplier * self.consumption(config),
            new_cell_energy_directions,
            Some(parent_dir),
            steps_to_death,
        ))
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
            Up => EnergyDirections {
                up: true,
                down: false,
                left: false,
                right: false,
            },
            Down => EnergyDirections {
                up: false,
                down: true,
                left: false,
                right: false,
            },
            Left => EnergyDirections {
                up: false,
                down: false,
                left: true,
                right: false,
            },
            Right => EnergyDirections {
                up: false,
                down: false,
                left: false,
                right: true,
            },
        }
    }

    pub const fn get(&self, dir: CellDir) -> bool {
        match dir {
            Up => self.up,
            Down => self.down,
            Left => self.left,
            Right => self.right,
        }
    }

    pub fn set(&mut self, dir: CellDir, value: bool) {
        match dir {
            Up => self.up = value,
            Down => self.down = value,
            Left => self.left = value,
            Right => self.right = value,
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
