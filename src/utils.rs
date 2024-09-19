use bevy::{
    asset::{Assets, Handle},
    prelude::Query,
};
use bevy_fast_tilemap::{Map, MapIndexer};

use crate::{
    cells::{
        life_cell::{EnergyDirections, LifeCell::*},
        WorldCell,
    },
    grid::Area,
};

pub fn get_continual_coord(n: i64, max: u32) -> u32 {
    (n).rem_euclid(max as i64) as u32
}

pub fn get_map<'a>(
    maps: &Query<&Handle<Map>>,
    map_materials: *mut Assets<Map>,
    id: usize,
) -> MapIndexer<'a> {
    let map_handle = maps.iter().nth(id).unwrap();
    let map_materials = unsafe { &mut *map_materials };

    let Some(map) = map_materials.get_mut(map_handle) else {
        panic!("No map material");
    };

    map.indexer_mut()
}

pub const fn merge_energy(
    area: &Area<WorldCell>,
    mut directions: EnergyDirections,
) -> EnergyDirections {
    if let Alive(life) = area.up.life {
        if life.energy_to.down {
            directions.up = true
        }
    }

    if let Alive(life) = area.down.life {
        if life.energy_to.up {
            directions.down = true
        }
    }

    if let Alive(life) = area.left.life {
        if life.energy_to.right {
            directions.left = true
        }
    }

    if let Alive(life) = area.right.life {
        if life.energy_to.left {
            directions.right = true
        }
    }

    directions
}

#[macro_export]
macro_rules! all_directions {
    ($macro: ident) => {
        $macro!(center);
        $macro!(up);
        $macro!(down);
        $macro!(left);
        $macro!(right);

        $macro!(up_left);
        $macro!(up_right);

        $macro!(down_left);
        $macro!(down_right);
    };
}

#[macro_export]
macro_rules! cell_directions {
    ($macro: ident) => {
        $macro!(up);
        $macro!(down);
        $macro!(left);
        $macro!(right);
    };
}

#[macro_export]
macro_rules! cell_op_directions {
    ($macro: ident) => {
        $macro!(up, down);
        $macro!(down, up);
        $macro!(left, right);
        $macro!(right, left);
    };
}
