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
