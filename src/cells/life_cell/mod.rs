use genome::Genome;

use crate::{
    grid::Area,
    types::CellDir::{self, *},
};

use super::WorldCell;

pub const MAX_ENERGY_TRANSFER: f32 = 1.;

pub mod genome;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LifeCell {
    Alive(LifeType),

    #[default]
    Dead,
}

impl LifeCell {
    pub const fn is_alive(&self) -> bool {
        match self {
            LifeCell::Alive(life_type) => true,
            LifeCell::Dead => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeType {
    // Pipe,
    // Leaf,
    // Root,
    // Reactor,
    // Filter,
    Stem { genome: Genome, energy: f32 },
}
