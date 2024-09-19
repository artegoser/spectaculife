use crate::{
    all_directions, cell_directions, cell_op_directions,
    cells::{
        life_cell::{BirthDirective, LifeCell::*, LifeType::*},
        soil_cell::{MAX_ENERGY_LIFE, MAX_ORGANIC_LIFE},
        WorldCell,
    },
    grid::Area,
    types::CellDir::*,
};

pub fn update_area(mut area: Area<WorldCell>) {
    process_air(&mut area);
    process_soil(&mut area);

    if let Alive(mut life) = area.center.life {
        life.energy -= life.consumption();

        if life.energy < 0. {
            return kill(&mut area);
        }

        if (life.energy_to.branches_amount() == 0) && !life.is_fertile() {
            if life.is_pipe() {
                if let Some(dir) = life.parent_dir {
                    macro_rules! reroute {
                        ($dir: ident, $op_dir: ident) => {
                            if let Alive(mut up) = area.$dir.life {
                                life.energy_to.$dir = true;

                                up.energy_to.$op_dir = false;
                                area.$dir.life = Alive(up);
                            }
                        };
                    }

                    life.parent_dir = None;
                    match dir {
                        Up => reroute!(up, down),
                        Down => reroute!(down, up),
                        Left => reroute!(left, right),
                        Right => reroute!(right, left),
                    }
                } else {
                    return kill(&mut area);
                }
            } else {
                return kill(&mut area);
            }
        }

        if area.center.soil.organics > MAX_ORGANIC_LIFE || area.center.soil.energy > MAX_ENERGY_LIFE
        {
            return kill(&mut area);
        }

        // Generate energy
        {
            match life.ty {
                Leaf => {
                    let total = 1.8 / (area.center.air.pollution as f32 / 4.).max(1.);
                    life.energy += total;
                    area.center.soil.energy += total * 0.5;
                }
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

                    all_directions!(process_organic);

                    life.energy += total * 0.4;
                    area.center.soil.energy += total * 0.5;
                }
                Reactor => {
                    let mut total = 0.0;

                    macro_rules! process_energy {
                        ($dir: ident) => {
                            let energy = area.$dir.soil.energy * 0.16;
                            area.$dir.soil.energy -= energy;

                            total += energy as f32;
                        };
                    }

                    all_directions!(process_energy);

                    life.energy += total * 0.6;
                    area.center.soil.organics = area
                        .center
                        .soil
                        .organics
                        .saturating_add((total * 0.2) as u8)
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
                try_birth(
                    &mut area,
                    BirthDirective::new(Some(Cancer), Some(Cancer), Some(Cancer), Some(Cancer)),
                );
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

fn process_soil(area: &mut Area<WorldCell>) {
    let mut total: f32 = 0.;

    total += area.up_left.soil.energy;
    total += area.up.soil.energy;
    total += area.up_right.soil.energy;
    total += area.left.soil.energy;
    total += area.center.soil.energy;
    total += area.right.soil.energy;
    total += area.down_left.soil.energy;
    total += area.down.soil.energy;
    total += area.down_right.soil.energy;

    let foreach = total / 9.0025;

    area.up_left.soil.energy = foreach;
    area.up.soil.energy = foreach;
    area.up_right.soil.energy = foreach;
    area.left.soil.energy = foreach;
    area.center.soil.energy = foreach;
    area.right.soil.energy = foreach;
    area.down_left.soil.energy = foreach;
    area.down.soil.energy = foreach;
    area.down_right.soil.energy = foreach;
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

        macro_rules! transfer {
            ($dir: ident) => {
                if life.energy_to.$dir {
                    if let Alive(mut $dir) = area.$dir.life {
                        if $dir.is_pipe_recipient() {
                            $dir.energy += flow_each;
                            area.$dir.life = Alive($dir);
                        } else {
                            life.energy_to.$dir = false;
                        }
                    } else {
                        life.energy_to.$dir = false;
                    }
                }
            };
        }

        cell_directions!(transfer);

        area.center.life = Alive(life);
    }
}

/// Try to give birth to cell at given direction
fn try_birth(area: &mut Area<WorldCell>, birth_directive: BirthDirective) {
    if let Alive(mut life) = area.center.life {
        let energy_capacity = birth_directive.energy_capacity();

        if life.energy > energy_capacity {
            life.energy -= energy_capacity;

            let mut born_once = false;

            macro_rules! try_newborn {
                ($dir: ident, $op_dir: ident) => {
                    if let Some(cell_type) = birth_directive.$dir {
                        if area.$dir.life == Dead {
                            if cell_type.is_fertile() {
                                life.energy_to.$dir = true;
                            }

                            area.$dir.life = cell_type.make_newborn_cell($op_dir);

                            born_once = true;
                        }
                    }
                };
            }

            try_newborn!(up, Down);
            try_newborn!(down, Up);
            try_newborn!(left, Right);
            try_newborn!(right, Left);

            // Cell rebirth
            if born_once {
                life.ty = match life.ty {
                    Stem(_) => Pipe,

                    ty => ty,
                };
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
        macro_rules! reroute {
            ($dir:ident,$op_dir:ident) => {
                if let Alive(mut $dir) = area.$dir.life {
                    $dir.energy_to.$op_dir = false;

                    if let Some(Down) = $dir.parent_dir {
                        $dir.parent_dir = None;
                    }

                    area.$dir.life = Alive($dir);
                }
            };
        }

        cell_op_directions!(reroute);
    }
}
