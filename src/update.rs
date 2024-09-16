use bevy::log::warn;
use rand::Rng;

use crate::{
    cells::{
        life_cell::{
            AliveCell, EnergyDirections,
            LifeCell::*,
            LifeType::{self, *},
        },
        WorldCell,
    },
    grid::Area,
    types::CellDir::{self, *},
};

pub fn update_area(mut area: Area<WorldCell>) -> Area<WorldCell> {
    let mut rng = rand::thread_rng();

    if let Alive(mut life) = area.center.life {
        if life.energy < life.consumption() {
            return kill(area);
        }

        life.energy -= life.consumption();

        //
        if matches!(
            life.energy_to,
            EnergyDirections {
                up: false,
                down: false,
                left: false,
                right: false
            }
        ) {
            match life.parent {
                Some(dir) => match dir {
                    Up => life.energy_to.up = true,
                    Down => life.energy_to.down = true,
                    Left => life.energy_to.left = true,
                    Right => life.energy_to.right = true,
                },
                None => return kill(area),
            }
        }

        match life.ty {
            Cancer => {
                let n = rng.gen_range(0..4);

                if matches!((area.up.life, n), (Dead, 0)) {
                    area.up.life = Alive(AliveCell::new(
                        LifeType::Cancer,
                        5.,
                        Some(Down),
                        EnergyDirections::default(),
                    ))
                }

                if matches!((area.down.life, n), (Dead, 1)) {
                    area.down.life = Alive(AliveCell::new(
                        LifeType::Cancer,
                        5.,
                        Some(Up),
                        EnergyDirections::default(),
                    ))
                }

                if matches!((area.left.life, n), (Dead, 2)) {
                    area.left.life = Alive(AliveCell::new(
                        LifeType::Cancer,
                        5.,
                        Some(Right),
                        EnergyDirections::default(),
                    ))
                }

                if matches!((area.right.life, n), (Dead, 3)) {
                    area.right.life = Alive(AliveCell::new(
                        LifeType::Cancer,
                        5.,
                        Some(Left),
                        EnergyDirections::default(),
                    ))
                }
            }
        }

        area.center.life = Alive(life);
    }

    area
}

/// Try to born cell at given direction
fn try_born(mut area: Area<WorldCell>, direction: CellDir, life_type: LifeType) -> Area<WorldCell> {
    if let Alive(mut life) = area.center.life {
        let to_produce_energy = 2. * life_type.consumption();

        if life.energy > to_produce_energy {
            life.energy -= to_produce_energy;

            life.ty = match life.ty {
                Cancer => Cancer,

                ty => {
                    warn!("The sterile cell gave birth");
                    ty
                }
            };

            let new_cell = {
                let new_cell_energy_directions = match life_type {
                    Cancer => EnergyDirections::default(),
                };

                let new_cell_energy = match direction {
                    Up => life_type.consumption() + area.up.life.energy() * 0.5,
                    Down => life_type.consumption() + area.down.life.energy() * 0.5,
                    Left => life_type.consumption() + area.left.life.energy() * 0.5,
                    Right => life_type.consumption() + area.right.life.energy() * 0.5,
                };

                Alive(AliveCell::new(
                    life_type,
                    new_cell_energy,
                    Some(direction.opposite()),
                    new_cell_energy_directions,
                ))
            };

            match direction {
                Up => area.up.life = new_cell,
                Down => area.down.life = new_cell,
                Left => area.left.life = new_cell,
                Right => area.right.life = new_cell,
            };
        }

        area.center.life = Alive(life)
    }

    area
}

/// Kill cell and reroute energy paths
fn kill(mut area: Area<WorldCell>) -> Area<WorldCell> {
    area.center.life = Dead;

    // Reroute energy of neighbors
    {
        if let Alive(mut life) = area.up.life {
            life.energy_to.down = false;

            if matches!(life.parent, Some(Down)) {
                life.parent = None;
            }

            area.up.life = Alive(life)
        }

        if let Alive(mut life) = area.down.life {
            life.energy_to.up = false;

            if matches!(life.parent, Some(Up)) {
                life.parent = None;
            }

            area.down.life = Alive(life)
        }

        if let Alive(mut life) = area.left.life {
            life.energy_to.right = false;

            if matches!(life.parent, Some(Right)) {
                life.parent = None;
            }

            area.left.life = Alive(life)
        }

        if let Alive(mut life) = area.right.life {
            life.energy_to.left = false;

            if matches!(life.parent, Some(Left)) {
                life.parent = None;
            }

            area.right.life = Alive(life)
        }
    }

    return area;
}
