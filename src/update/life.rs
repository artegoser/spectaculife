use crate::{
    all_directions, cell_directions, cell_op_directions,
    cells::{
        life_cell::{AliveCell, BirthDirective, LifeCell::*, LifeType::*},
        soil_cell::{MAX_ENERGY_LIFE, MAX_ORGANIC_LIFE},
        WorldCell,
    },
    grid::Area,
    types::CellDir::{self, *},
};

pub fn update_life(area: &mut Area<WorldCell>) {
    if let Alive(mut life) = area.center.life {
        if area.center.soil.organics > MAX_ORGANIC_LIFE || area.center.soil.energy > MAX_ENERGY_LIFE
        {
            return kill(area);
        }

        life.energy -= life.consumption();

        if life.energy < 0. {
            return kill(area);
        }

        if (life.energy_to.branches_amount() == 0) && !life.is_fertile() {
            if life.is_pipe() {
                if let Some(parent_dir) = life.parent_dir {
                    reroute_energy_paths(area, &mut life, parent_dir);
                } else {
                    return kill(area);
                }
            } else {
                return kill(area);
            }
        }

        // Generate energy
        generate_energy(area, &mut life);

        // Transfer energy
        if life.energy_to.branches_amount() > 0 {
            transfer_energy(area, &mut life);
        }

        // Process fertile cells
        match life.ty {
            Stem(genome) => try_birth(
                area,
                &mut life,
                genome.genes[genome.active_gene as usize].to_birth_directive(genome),
            ),
            Cancer => {
                try_birth(
                    area,
                    &mut life,
                    BirthDirective::new(Some(Cancer), Some(Cancer), Some(Cancer), Some(Cancer)),
                );
            }

            _ => {}
        }

        area.center.life = Alive(life);
    }
}

fn generate_energy(area: &mut Area<WorldCell>, life: &mut AliveCell) {
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

                        area.$dir.air.pollution = area.$dir.air.pollution.saturating_add(organic);
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

fn reroute_energy_paths(area: &mut Area<WorldCell>, life: &mut AliveCell, parent_dir: CellDir) {
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
    match parent_dir {
        Up => reroute!(up, down),
        Down => reroute!(down, up),
        Left => reroute!(left, right),
        Right => reroute!(right, left),
    }
}

/// Transfer energy
fn transfer_energy(area: &mut Area<WorldCell>, life: &mut AliveCell) {
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
}

/// Try to give birth to cell at given direction
fn try_birth(area: &mut Area<WorldCell>, life: &mut AliveCell, birth_directive: BirthDirective) {
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
