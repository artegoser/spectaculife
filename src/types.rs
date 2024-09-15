use bevy::prelude::Entity;
use bevy_ecs_tilemap::{
    map::TilemapSize,
    tiles::{TilePos, TileStorage},
};

use crate::utils::get_continual_coord;

pub enum CellDir {
    Up,
    Down,
    Left,
    Right,
}

pub struct CellNeighbors<'a> {
    up: TilePos,
    down: TilePos,
    left: TilePos,
    right: TilePos,

    storage: &'a TileStorage,
}

impl<'a> CellNeighbors<'a> {
    pub fn new(tile_pos: &TilePos, storage: &'a TileStorage) -> CellNeighbors<'a> {
        let x = tile_pos.x;
        let y = tile_pos.y;

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

    pub fn get_up(&self) -> Entity {
        self.storage.get(&self.up).unwrap()
    }

    pub fn get_down(&self) -> Entity {
        self.storage.get(&self.down).unwrap()
    }

    pub fn get_left(&self) -> Entity {
        self.storage.get(&self.down).unwrap()
    }

    pub fn get_right(&self) -> Entity {
        self.storage.get(&self.down).unwrap()
    }
}
