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
            if life.energy_to.branches_amount() == 0 && matches!(life.ty, Pipe) {
                match life.parent {
                    Some(dir) => match dir {
                        Up => life.energy_to.up = true,
                        Down => life.energy_to.down = true,
                        Left => life.energy_to.left = true,
                        Right => life.energy_to.right = true,
                    },
                    None => {
                        return kill(area);
                    }
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
                up.energy += flow_each;
                area.up.life = Alive(up);
            }
        }

        if life.energy_to.down {
            if let Alive(mut down) = area.down.life {
                down.energy += flow_each;
                area.down.life = Alive(down);
            }
        }

        if life.energy_to.left {
            if let Alive(mut left) = area.left.life {
                left.energy += flow_each;
                area.left.life = Alive(left);
            }
        }

        if life.energy_to.right {
            if let Alive(mut right) = area.right.life {
                right.energy += flow_each;
                area.right.life = Alive(right);
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
            life.energy -= energy_capacity;

            // Cell rebirth
            life.ty = match life.ty {
                Cancer => Pipe,

                ty => ty,
            };

            if let Some(cell_type) = birth_directive.up {
                life.energy_to.up = true;
                area.up.life = cell_type.make_newborn_cell(Down, area.up.life.energy());
            }

            if let Some(cell_type) = birth_directive.down {
                life.energy_to.down = true;
                area.down.life = cell_type.make_newborn_cell(Up, area.down.life.energy());
            }

            if let Some(cell_type) = birth_directive.left {
                life.energy_to.left = true;
                area.left.life = cell_type.make_newborn_cell(Right, area.left.life.energy());
            }

            if let Some(cell_type) = birth_directive.right {
                life.energy_to.right = true;
                area.right.life = cell_type.make_newborn_cell(Left, area.right.life.energy());
            }
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
