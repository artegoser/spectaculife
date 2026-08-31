use bevy::asset::{Assets, Handle};
use bevy_fast_tilemap::prelude::*;

use crate::{
    cells::{
        life_cell::{EnergyDirections, LifeCell},
        WorldCell,
    },
    grid::Grid,
    types::CellDir,
};

pub fn get_continual_coord(n: i64, max: u32) -> u32 {
    n.rem_euclid(max as i64) as u32
}

pub fn get_map<'a>(
    map_handle: &Handle<Map>,
    map_materials: *mut Assets<Map>,
) -> MapIndexerMut<'a> {
    let map_materials = unsafe { &mut *map_materials };

    let Some(map) = map_materials.get_mut(map_handle) else {
        panic!("No map material");
    };

    map.indexer_mut()
}

pub fn merge_energy(
    grid: &Grid<WorldCell>,
    index: usize,
    mut directions: EnergyDirections,
) -> EnergyDirections {
    for dir in CellDir::ALL {
        let (dx, dy) = dir.offset();
        let neighbor = grid.offset_index(index, dx, dy);
        if let LifeCell::Alive(life) = grid.cells()[neighbor].life {
            if life.energy_to.get(dir.opposite()) {
                directions.set(dir, true);
            }
        }
    }
    directions
}
