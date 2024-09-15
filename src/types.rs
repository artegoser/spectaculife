use bevy::prelude::{Entity, Query};
use bevy_ecs_tilemap::{
    map::TilemapSize,
    tiles::{TilePos, TileStorage, TileTextureIndex},
};

use crate::{components::life_cell::LifeCell, utils::get_continual_coord};

#[derive(Debug, Clone)]
pub enum CellDir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct CellNeighbors<'a> {
    up: TilePos,
    down: TilePos,
    left: TilePos,
    right: TilePos,

    storage: &'a TileStorage,
}

impl<'a> CellNeighbors<'a> {
    pub fn new(position: &TilePos, storage: &'a TileStorage) -> CellNeighbors<'a> {
        let x = position.x;
        let y = position.y;

        let width = storage.size.x;
        let height = storage.size.y;

        CellNeighbors {
            up: TilePos::new(x, get_continual_coord(y as i32 + 1, height)),
            down: TilePos::new(x, get_continual_coord(y as i32 - 1, height)),
            left: TilePos::new(get_continual_coord(x as i32 - 1, width), y),
            right: TilePos::new(get_continual_coord(x as i32 + 1, width), y),

            storage,
        }
    }

    pub fn get(&self, dir: CellDir) -> Option<Entity> {
        match dir {
            CellDir::Up => self.storage.get(&self.up),
            CellDir::Down => self.storage.get(&self.down),
            CellDir::Left => self.storage.get(&self.left),
            CellDir::Right => self.storage.get(&self.right),
        }
    }
}
