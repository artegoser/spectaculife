use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TileTextureIndex;

use crate::types::CellDir;

#[derive(Component)]
pub struct LifeCell {
    cell: LifeCellType,

    energy: f32,

    energy_to: (bool, bool, bool),
    parent: Option<CellDir>,
}

impl LifeCell {
    pub fn new(cell: LifeCellType, energy: f32, parent: Option<CellDir>) -> Self {
        Self {
            cell,
            energy,

            energy_to: (false, false, false),
            parent,
        }
    }

    pub fn texture_id(&self) -> TileTextureIndex {
        TileTextureIndex(match self.cell {
            LifeCellType::Cancer => 6,
        })
    }
}

pub enum LifeCellType {
    // Pipe(PipeCell),
    Cancer,
}
