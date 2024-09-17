use bevy::log::warn;
use rand::random;

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
    if let Alive(mut life) = area.center.life {
        life.energy -= life.consumption();

        if life.energy < 0. {
            return kill(area);
        }

        // Reroute energy
        {
            if life.energy_to.branches_amount() == 0 && !matches!(life.ty, Cancer) {
                match life.parent {
                    Some(dir) => match dir {
                        Up => life.energy_to.up = true,
                        Down => life.energy_to.down = true,
                        Left => life.energy_to.left = true,
                        Right => life.energy_to.right = true,
                    },
                    None => {}
                };
            };
        }

        area.center.life = Alive(life);

        // Transfer energy
        if life.energy_to.branches_amount() > 0 {
            area = transfer_energy(area);
        }

        // Process fertile cells
        match life.ty {
            Cancer => {
                let dir = random::<CellDir>();

                if matches!(area.cell_from_dir(&dir).life, Dead) {
                    area = try_born(area, dir, Cancer)
                }
            }
        }
    }

    area
}

/// Transfer energy
fn transfer_energy(mut area: Area<WorldCell>) -> Area<WorldCell> {
    if let Alive(mut life) = area.center.life {
        let flow_each = {
            let to_flow = life.energy_flow();

            life.energy -= to_flow;

            if life.energy < life.consumption() {
                return area;
            }

            to_flow / (life.energy_to.branches_amount() as f32)
        };

        if life.energy_to.up {
            if let Alive(mut life) = area.up.life {
                life.energy += flow_each;
                area.up.life = Alive(life);
            }
        }

        if life.energy_to.down {
            if let Alive(mut life) = area.down.life {
                life.energy += flow_each;
                area.down.life = Alive(life);
            }
        }

        if life.energy_to.left {
            if let Alive(mut life) = area.left.life {
                life.energy += flow_each;
                area.left.life = Alive(life);
            }
        }

        if life.energy_to.right {
            if let Alive(mut life) = area.right.life {
                life.energy += flow_each;
                area.right.life = Alive(life);
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

            match direction {
                Up => life.energy_to.up = true,
                Down => life.energy_to.down = true,
                Left => life.energy_to.left = true,
                Right => life.energy_to.right = true,
            };

            let new_cell = {
                let new_cell_energy_directions = match life_type {
                    Cancer => EnergyDirections::default(),
                };

                let new_cell_energy = match direction {
                    Up => to_produce_energy * 0.8 + area.up.life.energy() * 0.5,
                    Down => to_produce_energy * 0.8 + area.down.life.energy() * 0.5,
                    Left => to_produce_energy * 0.8 + area.left.life.energy() * 0.5,
                    Right => to_produce_energy * 0.8 + area.right.life.energy() * 0.5,
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
