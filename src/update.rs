use crate::{
    cells::{
        life_cell::{BirthDirective, LifeCell::*, LifeType::*},
        soil_cell::MAX_ORGANIC_LIFE,
        WorldCell,
    },
    grid::Area,
    types::CellDir::{self, *},
};

pub fn update_area(mut area: Area<WorldCell>) {
    process_air(&mut area);

    if let Alive(mut life) = area.center.life {
        life.energy -= life.consumption();

        if life.energy < 0. {
            return kill(&mut area);
        }

        if (life.energy_to.branches_amount() == 0) && !life.is_fertile() {
            return kill(&mut area);
        }

        if area.center.soil.organics > MAX_ORGANIC_LIFE {
            return kill(&mut area);
        }

        // Generate energy
        {
            match life.ty {
                Leaf => life.energy += 1.8 / (area.center.air.pollution as f32 / 4.).max(1.),
                Root => {
                    let mut total = 0.0;

                    macro_rules! process_organic {
                        ($dir: ident) => {
                            if area.$dir.soil.organics <= 8 && area.$dir.soil.organics > 0 {
                                area.$dir.soil.organics -= 1;
                                total += 1.;
                            } else {
                                let organic = (area.$dir.soil.organics as f32 * 0.16) as u8;
                                area.$dir.soil.organics -= organic;

                                total += organic as f32;

                                area.$dir.air.pollution =
                                    area.$dir.air.pollution.saturating_add(organic);
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
            transfer_energy(&mut area);
        }

        // Process fertile cells
        match life.ty {
            Stem(genome) => try_birth(
                &mut area,
                genome.genes[genome.active_gene as usize].to_birth_directive(genome),
            ),
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

                try_birth(&mut area, BirthDirective::new(up, down, left, right));
            }

            _ => {}
        }
    }
}

fn process_air(area: &mut Area<WorldCell>) {
    let mut total: u16 = 0;

    total += area.up_left.air.pollution as u16;
    total += area.up.air.pollution as u16;
    total += area.up_right.air.pollution as u16;
    total += area.left.air.pollution as u16;
    total += area.center.air.pollution as u16;
    total += area.right.air.pollution as u16;
    total += area.down_left.air.pollution as u16;
    total += area.down.air.pollution as u16;
    total += area.down_right.air.pollution as u16;

    let foreach: u8 = (total / 9) as u8;
    let left = (total - (foreach as u16 * 9)) as u8;

    if left == 0 && foreach != 0 && foreach != 255 {
        area.up_left.air.pollution = foreach - 1;
        area.up.air.pollution = foreach - 1;
        area.up_right.air.pollution = foreach;
        area.left.air.pollution = foreach - 1;
        area.center.air.pollution = foreach;
        area.right.air.pollution = foreach + 1;
        area.down_left.air.pollution = foreach;
        area.down.air.pollution = foreach + 1;
        area.down_right.air.pollution = foreach + 1;
    }
}

/// Transfer energy
fn transfer_energy(area: &mut Area<WorldCell>) {
    if let Alive(mut life) = area.center.life {
        if !life.can_transfer() {
            return;
        }

        let flow_each = {
            let to_flow = (life.energy * 0.5 - 2. * life.consumption()).max(0.);

            life.energy -= to_flow;

            if life.energy < life.consumption() {
                return;
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
}

/// Try to give birth to cell at given direction
fn try_birth(area: &mut Area<WorldCell>, birth_directive: BirthDirective) {
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
                area.up.soil.organics = area
                    .up
                    .soil
                    .organics
                    .saturating_add(area.up.life.organics());
                area.up.life = cell_type.make_newborn_cell(Down, area.up.life.energy());
            }

            if let Some(cell_type) = birth_directive.down {
                if cell_type.is_fertile() {
                    life.energy_to.down = true;
                }

                area.down.soil.organics = area
                    .down
                    .soil
                    .organics
                    .saturating_add(area.down.life.organics());

                area.down.life = cell_type.make_newborn_cell(Up, area.down.life.energy());
            }

            if let Some(cell_type) = birth_directive.left {
                if cell_type.is_fertile() {
                    life.energy_to.left = true;
                }

                area.left.soil.organics = area
                    .left
                    .soil
                    .organics
                    .saturating_add(area.left.life.organics());

                area.left.life = cell_type.make_newborn_cell(Right, area.left.life.energy());
            }

            if let Some(cell_type) = birth_directive.right {
                if cell_type.is_fertile() {
                    life.energy_to.right = true;
                }

                area.right.soil.organics = area
                    .right
                    .soil
                    .organics
                    .saturating_add(area.right.life.organics());

                area.right.life = cell_type.make_newborn_cell(Left, area.right.life.energy());
            }
        }

        area.center.life = Alive(life)
    }
}

/// Kill cell and reroute energy paths
fn kill(area: &mut Area<WorldCell>) {
    area.center.soil.organics = area
        .center
        .soil
        .organics
        .saturating_add(area.center.life.organics());

    area.center.soil.energy += area.center.life.energy() * 0.5;

    area.center.air.pollution = area
        .center
        .air
        .pollution
        .saturating_add((area.center.life.organics() / 2).max(1));

    area.center.life = Dead;

    // Reroute energy of neighbors
    {
        if let Alive(mut life) = area.up.life {
            life.energy_to.down = false;
            area.up.life = Alive(life)
        }

        if let Alive(mut life) = area.down.life {
            life.energy_to.up = false;
            area.down.life = Alive(life)
        }

        if let Alive(mut life) = area.left.life {
            life.energy_to.right = false;
            area.left.life = Alive(life)
        }

        if let Alive(mut life) = area.right.life {
            life.energy_to.left = false;
            area.right.life = Alive(life)
        }
    }
}
