use crate::{
    cells::{
        life_cell::{BirthDirective, LifeCell::*, LifeType::*},
        soil_cell::MAX_ORGANIC_LIFE,
        WorldCell,
    },
    grid::Area,
    types::CellDir::*,
};

pub fn update_area(mut area: Area<WorldCell>) -> Area<WorldCell> {
    if let Alive(mut life) = area.center.life {
        life.energy -= life.consumption();

        if life.energy < 0. {
            return kill(area);
        }

        if (life.energy_to.branches_amount() == 0) && !life.is_fertile() {
            return kill(area);
        }

        if area.center.soil.organic > MAX_ORGANIC_LIFE {
            return kill(area);
        }

        // Generate energy
        {
            match life.ty {
                Leaf => life.energy += 1.8,
                Root => {
                    let mut total = 0.0;

                    macro_rules! process_organic {
                        ($dir: ident) => {
                            if area.$dir.soil.organic <= 4 && area.$dir.soil.organic > 0 {
                                area.$dir.soil.organic -= 1;
                                total += 1.;
                            } else {
                                let organic = (area.$dir.soil.organic as f32 * 0.25) as u8;
                                area.$dir.soil.organic -= organic;

                                total += organic as f32;
                            };
                        };
                    }

                    process_organic!(center);
                    process_organic!(up);
                    process_organic!(down);
                    process_organic!(left);
                    process_organic!(right);

                    process_organic!(up_left);
                    process_organic!(up_right);

                    process_organic!(down_left);
                    process_organic!(down_right);

                    life.energy += total * 0.4;
                }
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
            Stem(genome) => {
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
fn transfer_energy(area: Area<WorldCell>) -> Area<WorldCell> {
    if let Alive(mut life) = area.center.life {
        if !life.can_transfer() {
            return area;
        }

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
                if up.is_pipe_recipient() {
                    up.energy += flow_each;
                    area.up.life = Alive(up);
                } else {
                    life.energy_to.up = false;
                }
            }
        }

        if life.energy_to.down {
            if let Alive(mut down) = area.down.life {
                if down.is_pipe_recipient() {
                    down.energy += flow_each;
                    area.down.life = Alive(down);
                } else {
                    life.energy_to.down = false;
                }
            }
        }

        if life.energy_to.left {
            if let Alive(mut left) = area.left.life {
                if left.is_pipe_recipient() {
                    left.energy += flow_each;
                    area.left.life = Alive(left);
                } else {
                    life.energy_to.left = false;
                }
            }
        }

        if life.energy_to.right {
            if let Alive(mut right) = area.right.life {
                if right.is_pipe_recipient() {
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
fn try_birth(area: Area<WorldCell>, birth_directive: BirthDirective) -> Area<WorldCell> {
    if let Alive(mut life) = area.center.life {
        let energy_capacity = birth_directive.energy_capacity();

        if life.energy > energy_capacity {
            life.energy -= energy_capacity;

            // Cell rebirth
            life.ty = match life.ty {
                Stem(_) => Pipe,

                ty => ty,
            };

            if let Some(cell_type) = birth_directive.up {
                if cell_type.is_fertile() {
                    life.energy_to.up = true;
                }
                area.up.soil.organic = area.up.soil.organic.saturating_add(area.up.life.organics());
                area.up.life = cell_type.make_newborn_cell(Down, area.up.life.energy());
            }

            if let Some(cell_type) = birth_directive.down {
                if cell_type.is_fertile() {
                    life.energy_to.down = true;
                }

                area.down.soil.organic = area
                    .down
                    .soil
                    .organic
                    .saturating_add(area.down.life.organics());

                area.down.life = cell_type.make_newborn_cell(Up, area.down.life.energy());
            }

            if let Some(cell_type) = birth_directive.left {
                if cell_type.is_fertile() {
                    life.energy_to.left = true;
                }

                area.left.soil.organic = area
                    .left
                    .soil
                    .organic
                    .saturating_add(area.left.life.organics());

                area.left.life = cell_type.make_newborn_cell(Right, area.left.life.energy());
            }

            if let Some(cell_type) = birth_directive.right {
                if cell_type.is_fertile() {
                    life.energy_to.right = true;
                }

                area.right.soil.organic = area
                    .right
                    .soil
                    .organic
                    .saturating_add(area.right.life.organics());

                area.right.life = cell_type.make_newborn_cell(Left, area.right.life.energy());
            }
        }

        area.center.life = Alive(life)
    }

    area
}

/// Kill cell and reroute energy paths
fn kill(area: Area<WorldCell>) -> Area<WorldCell> {
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
