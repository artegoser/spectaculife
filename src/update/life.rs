use rand::{thread_rng, Rng};

use crate::{
    all_directions, cell_directions, cell_op_directions_enum, cell_op_directions_with_enum,
    cells::{
        life_cell::{
            genome::{Gene, GeneAction::*, GeneCondition::*, Genome},
            AliveCell, EnergyDirections,
            LifeCell::{self, *},
            LifeType::*,
        },
        soil_cell::{MAX_ENERGY_LIFE, MAX_ORGANIC_LIFE},
        WorldCell,
    },
    grid::Area,
    types::CellDir::{self, *},
};

pub fn update_life(area: &mut Area<WorldCell>) {
    if let Alive(mut life) = area.center.life {
        if life.steps_to_death == 0 {
            return kill(area);
        } else {
            life.steps_to_death -= 1;
        }

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

        // Process genome
        match life.ty {
            Stem(genome) => {
                process_genome(area, &mut life, genome);
            }
            _ => {}
        };

        area.center.life = Alive(life);
    }
}

fn process_genome(area: &mut Area<WorldCell>, life: &mut AliveCell, mut genome: Genome) {
    let gene = genome.genes[genome.active_gene as usize];
    // let energy_capacity = ;

    if life.energy > gene.energy_capacity() {
        let mut action_once = false;
        let mut total_energy = 0.0;

        macro_rules! try_birth {
            ($dir: ident, $op_dir: ident, $cell_type: expr, $steps_to_death: expr) => {
                if area.$dir.life == Dead {
                    if $cell_type.is_fertile() {
                        life.energy_to.$dir = true;
                    }

                    let energy_spent = gene.$dir.energy_capacity();

                    area.$dir.life =
                        $cell_type.make_newborn_cell($op_dir, energy_spent * 0.8, $steps_to_death);

                    total_energy += energy_spent;
                    action_once = true;
                }
            };
        }

        macro_rules! try_action {
            ($dir: ident, $op_dir: ident) => {
                match gene.$dir {
                    MakeLeaf(steps_to_death) => {
                        try_birth!($dir, $op_dir, Leaf, steps_to_death);
                    }
                    MakeRoot(steps_to_death) => {
                        try_birth!($dir, $op_dir, Root, steps_to_death);
                    }
                    MakeReactor(steps_to_death) => {
                        try_birth!($dir, $op_dir, Reactor, steps_to_death);
                    }
                    MakeFilter(steps_to_death) => {
                        try_birth!($dir, $op_dir, Filter, steps_to_death);
                    }
                    MultiplySelf(steps_to_death, next_gene) => {
                        genome.mutate();
                        genome.active_gene = next_gene;

                        try_birth!($dir, $op_dir, Stem(genome), steps_to_death);
                    }
                    KillCell => {
                        if let Alive(mut $dir) = area.$dir.life {
                            life.energy += $dir.energy * 0.8;
                            $dir.steps_to_death = 0;
                            $dir.energy = 0.;

                            area.$dir.life = Alive($dir);
                        }
                    }
                    Nothing => {}
                };
            };
        }

        if check_gene_condition(area, life, gene) {
            cell_op_directions_enum!(try_action);
        } else {
            genome.active_gene = gene.secondary_gene;
            life.ty = Stem(genome);
        }

        // Update parent
        if action_once {
            life.energy -= total_energy;
            life.ty = Pipe;
        }
    }
}

fn check_gene_condition(area: &Area<WorldCell>, life: &AliveCell, gene: Gene) -> bool {
    match gene.condition {
        LifeUp => area.up.life.is_alive(),
        LifeDown => area.down.life.is_alive(),
        LifeLeft => area.left.life.is_alive(),
        LifeRight => area.right.life.is_alive(),

        LethalOrganicUp => area.up.soil.organics > MAX_ORGANIC_LIFE,
        LethalOrganicDown => area.down.soil.organics > MAX_ORGANIC_LIFE,
        LethalOrganicLeft => area.left.soil.organics > MAX_ORGANIC_LIFE,
        LethalOrganicRight => area.right.soil.organics > MAX_ORGANIC_LIFE,

        LethalEnergyUp => area.up.soil.energy > MAX_ENERGY_LIFE,
        LethalEnergyDown => area.down.soil.energy > MAX_ENERGY_LIFE,
        LethalEnergyLeft => area.left.soil.energy > MAX_ENERGY_LIFE,
        LethalEnergyRight => area.right.soil.energy > MAX_ENERGY_LIFE,

        RandomMT => thread_rng().gen::<u8>() > gene.param,
        LifeEnergyMT => life.energy > gene.param as f32,

        OrganicCenterMT => area.center.soil.organics > gene.param,
        OrganicUpMT => area.up.soil.organics > gene.param,
        OrganicDownMT => area.down.soil.organics > gene.param,
        OrganicLeftMT => area.left.soil.organics > gene.param,
        OrganicRightMT => area.right.soil.organics > gene.param,

        SoilEnergyCenterMT => area.center.soil.energy > gene.param as f32,
        SoilEnergyUpMT => area.up.soil.energy > gene.param as f32,
        SoilEnergyDownMT => area.down.soil.energy > gene.param as f32,
        SoilEnergyLeftMT => area.left.soil.energy > gene.param as f32,
        SoilEnergyRightMT => area.right.soil.energy > gene.param as f32,

        AirPollutionCenterMT => area.center.air.pollution > gene.param,
        AirPollutionUpMT => area.up.air.pollution > gene.param,
        AirPollutionDownMT => area.down.air.pollution > gene.param,
        AirPollutionEnergyLeftMT => area.left.air.pollution > gene.param,
        AirPollutionEnergyRightMT => area.right.air.pollution > gene.param,
    }
}

fn generate_energy(area: &mut Area<WorldCell>, life: &mut AliveCell) {
    match life.ty {
        Leaf => {
            let total = 1.8 / (area.center.air.pollution as f32 / 4.).max(1.);
            life.energy += total;
            area.center.soil.energy += total * 0.25;
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

            life.energy += total * 0.1;
        }
        Filter => {
            let mut total = 0.0;

            macro_rules! process_pollution {
                ($dir: ident) => {
                    if area.$dir.air.pollution <= 8 && area.$dir.air.pollution > 0 {
                        area.$dir.air.pollution -= 1;
                        total += 1.;
                    } else {
                        let pollution = (area.$dir.air.pollution as f32 * 0.16) as u8;
                        area.$dir.air.pollution -= pollution;

                        total += pollution as f32;

                        area.$dir.soil.organics = area.$dir.soil.organics.saturating_add(pollution);
                    };
                };
            }

            all_directions!(process_pollution);

            life.energy += total * 0.4;
            area.center.soil.organics = area
                .center
                .soil
                .organics
                .saturating_add((total * 0.3) as u8);
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
            ($dir:ident,$op_dir:ident, $op_dir_enum: ident) => {
                if let Alive(mut $dir) = area.$dir.life {
                    $dir.energy_to.$op_dir = false;

                    if let Some($op_dir_enum) = $dir.parent_dir {
                        $dir.parent_dir = None;
                    }

                    area.$dir.life = Alive($dir);
                }
            };
        }

        cell_op_directions_with_enum!(reroute);
    }
}
