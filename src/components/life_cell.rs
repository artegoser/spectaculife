use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::types::CellDir;

#[derive(Debug, Clone, Component)]
pub struct LifeCell {
    pub cell: LifeCellType,

    pub energy: f32,

    pub energy_to: (bool, bool, bool, bool),
    pub parent: Option<CellDir>,
}

impl LifeCell {
    pub fn new(cell: LifeCellType, energy: f32, parent: Option<CellDir>) -> Self {
        Self {
            cell,
            energy,

            energy_to: (false, false, false, false),
            parent,
        }
    }

    pub fn texture_id(&self) -> TileTextureIndex {
        self.cell.texture_id()
    }
}

#[derive(Debug, Clone)]
pub enum LifeCellType {
    // Pipe(PipeCell),
    Cancer,
}

impl LifeCellType {
    pub const fn texture_id(&self) -> TileTextureIndex {
        TileTextureIndex(match self {
            LifeCellType::Cancer => 6,
        })
    }

    pub const fn consumption(&self) -> f32 {
        match self {
            LifeCellType::Cancer => 1.,
        }
    }
}
