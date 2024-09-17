use bevy::log::warn;
use rand::random;

use crate::{
    cells::{
        life_cell::{
            AliveCell, BirthDirective, EnergyDirections,
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
            if life.energy_to.branches_amount() == 0 {
                match life.parent {
                    Some(parent_dir) => {
                        if life.is_pipe() {
                            match parent_dir {
                                Up => {
                                    life.energy_to.up = true;

                                    if let Alive(mut up) = area.up.life {
                                        up.energy_to.down = false;
                                        area.up.life = Alive(up);
                                    }
                                }
                                Down => {
                                    life.energy_to.down = true;

                                    if let Alive(mut down) = area.down.life {
                                        down.energy_to.up = false;
                                        area.down.life = Alive(down);
                                    }
                                }
                                Left => {
                                    life.energy_to.left = true;

                                    if let Alive(mut left) = area.left.life {
                                        left.energy_to.right = false;
                                        area.left.life = Alive(left);
                                    }
                                }
                                Right => {
                                    life.energy_to.right = true;

                                    if let Alive(mut right) = area.right.life {
                                        right.energy_to.left = false;
                                        area.right.life = Alive(right);
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        if !life.is_fertile() {
                            return kill(area);
                        };
                    }
                };
            }
        }

        // Generate energy
        {
            match life.ty {
                Leaf => life.energy += 1.,
                _ => {}
            }
        }

        area.center.life = Alive(life);

        // Transfer energy
        if life.energy_to.branches_amount() > 0 {
            area = transfer_energy(area);
        }

        // Process fertile cells
        match life.ty {
            StemCell(genome) => {
                area = try_birth(
                    area,
                    genome.genes[genome.active_gene as usize].to_birth_directive(genome),
                )
            }
            Cancer => {
                let up = match area.up.life {
                    Alive(_) => None,
                    Dead => Some(Cancer),
                };

                let down = match area.down.life {
                    Alive(_) => None,
                    Dead => Some(Cancer),
                };

                let left = match area.left.life {
                    Alive(_) => None,
                    Dead => Some(Cancer),
                };

                let right = match area.right.life {
                    Alive(_) => None,
                    Dead => Some(Cancer),
                };

                area = try_birth(area, BirthDirective::new(up, down, left, right))
            }

            _ => {}
        }
    }

    area
}

/// Transfer energy
fn transfer_energy(mut area: Area<WorldCell>) -> Area<WorldCell> {
    if let Alive(mut life) = area.center.life {
        let flow_each = {
            let to_flow = (life.energy * 0.5 - 2. * life.consumption()).max(0.);

            life.energy -= to_flow;

            if life.energy < life.consumption() {
                return area;
            }

            to_flow / (life.energy_to.branches_amount() as f32)
        };

        if life.energy_to.up {
            if let Alive(mut up) = area.up.life {
                if up.is_pipe() || up.is_fertile() {
                    up.energy += flow_each;
                    area.up.life = Alive(up);
                } else {
                    life.energy_to.up = false;
                }
            }
        }

        if life.energy_to.down {
            if let Alive(mut down) = area.down.life {
                if down.is_pipe() || down.is_fertile() {
                    down.energy += flow_each;
                    area.down.life = Alive(down);
                } else {
                    life.energy_to.down = false;
                }
            }
        }

        if life.energy_to.left {
            if let Alive(mut left) = area.left.life {
                if left.is_pipe() || left.is_fertile() {
                    left.energy += flow_each;
                    area.left.life = Alive(left);
                } else {
                    life.energy_to.left = false;
                }
            }
        }

        if life.energy_to.right {
            if let Alive(mut right) = area.right.life {
                if right.is_pipe() || right.is_fertile() {
                    right.energy += flow_each;
                    area.right.life = Alive(right);
                } else {
                    life.energy_to.right = false;
                }
            }
        }

        area.center.life = Alive(life);
    }

    area
}

/// Try to give birth to cell at given direction
fn try_birth(mut area: Area<WorldCell>, birth_directive: BirthDirective) -> Area<WorldCell> {
    if let Alive(mut life) = area.center.life {
        let energy_capacity = birth_directive.energy_capacity();

        if life.energy > energy_capacity {
            let mut capacity = 0.;

            // Cell rebirth
            life.ty = match life.ty {
                StemCell(_) => Pipe,

                ty => ty,
            };

            if let Some(cell_type) = birth_directive.up {
                if cell_type.is_fertile() {
                    life.energy_to.up = true;
                }
                let (cell, cell_capacity) =
                    cell_type.make_newborn_cell(Down, area.up.life.energy());

                if !area.up.life.is_alive() {
                    area.up.life = cell;
                    capacity += cell_capacity;
                }
            }

            if let Some(cell_type) = birth_directive.down {
                if cell_type.is_fertile() {
                    life.energy_to.down = true;
                }
                let (cell, cell_capacity) =
                    cell_type.make_newborn_cell(Up, area.down.life.energy());

                if !area.down.life.is_alive() {
                    area.down.life = cell;
                    capacity += cell_capacity;
                }
            }

            if let Some(cell_type) = birth_directive.left {
                if cell_type.is_fertile() {
                    life.energy_to.left = true;
                }
                let (cell, cell_capacity) =
                    cell_type.make_newborn_cell(Right, area.left.life.energy());

                if !area.left.life.is_alive() {
                    area.left.life = cell;
                    capacity += cell_capacity;
                }
            }

            if let Some(cell_type) = birth_directive.right {
                if cell_type.is_fertile() {
                    life.energy_to.right = true;
                }

                let (cell, cell_capacity) =
                    cell_type.make_newborn_cell(Left, area.right.life.energy());

                if !area.right.life.is_alive() {
                    area.right.life = cell;
                    capacity += cell_capacity;
                }
            }

            life.energy -= capacity;
        }

        area.center.life = Alive(life)
    }

    area
}

/// Kill cell and reroute energy paths
fn kill(mut area: Area<WorldCell>) -> Area<WorldCell> {
    area.center.soil.organic = area
        .center
        .soil
        .organic
        .saturating_add(area.center.life.organics());

    area.center.soil.energy += area.center.life.energy() * 0.5;
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
