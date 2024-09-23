use rand::{thread_rng, Rng};

use crate::{
    all_directions, cell_directions, cell_op_directions_enum, cell_op_directions_with_enum,
    cells::{
        life_cell::{
            genome::{
                Gene,
                GeneAction::*,
                GeneCondition::{self, *},
                GeneDirectionAction::*,
                GeneLocation, Genome,
            },
            AliveCell, EnergyDirections,
            LifeCell::{self, *},
            LifeType::*,
        },
        soil_cell::{MAX_ENERGY_LIFE, MAX_ORGANIC_LIFE},
        WorldCell,
    },
    grid::Area,
    types::{
        CellDir::{self, *},
        State,
    },
};

pub fn update_life(state: &mut State, area: &mut Area<WorldCell>) {
    if let Alive(mut life) = area.center.life {
        if life.steps_to_death == 0 {
            return kill(area);
        } else {
            life.steps_to_death -= 1;
        }

        if ((area.center.soil.organics > MAX_ORGANIC_LIFE) && (life.ty != Root))
            || ((area.center.soil.energy > MAX_ENERGY_LIFE) && (life.ty != Reactor))
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

        generate_energy(area, &mut life);

        // Transfer energy
        transfer_energy(area, &mut life);

        // Process genome
        match life.ty {
            Stem(genome) => {
                process_genome(state, area, &mut life, genome);
            }
            _ => {}
        };

        area.center.life = Alive(life);
    }
}

fn process_genome(
    state: &State,
    area: &mut Area<WorldCell>,
    life: &mut AliveCell,
    mut genome: Genome,
) {
    let total_energy = genome.active_gene().energy_capacity();

    if life.energy > total_energy {
        let mut birth_once = false;

        macro_rules! try_birth {
            ($dir: ident, $op_dir: ident, $cell_type: expr, $steps_to_death: expr) => {{
                if let Alive(mut $dir) = area.$dir.life {
                    $dir.steps_to_death = $dir.steps_to_death.saturating_sub(250);
                    area.$dir.life = Alive($dir);
                } else {
                    if $cell_type.is_fertile() {
                        life.energy_to.$dir = true;
                    }

                    area.$dir.life = $cell_type.make_newborn_cell($op_dir, $steps_to_death);

                    birth_once = true;
                }
            }};
        }

        macro_rules! kill_cell {
            ($dir:ident) => {
                if let Alive(mut $dir) = area.$dir.life {
                    life.energy += $dir.energy;

                    $dir.steps_to_death = 0;
                    $dir.energy = 0.;

                    area.$dir.life = Alive($dir);
                }
            };
        }

        macro_rules! direction_action {
            ($dir: ident, $op_dir: ident) => {
                match genome.active_gene().$dir {
                    MakeLeaf(lifespan) => try_birth!($dir, $op_dir, Leaf, lifespan.0),
                    MakeRoot(lifespan) => try_birth!($dir, $op_dir, Root, lifespan.0),
                    MakeReactor(lifespan) => try_birth!($dir, $op_dir, Reactor, lifespan.0),
                    MakeFilter(lifespan) => try_birth!($dir, $op_dir, Filter, lifespan.0),
                    MultiplySelf(lifespan, next_gene) => {
                        genome.mutate();
                        genome.active_gene = next_gene;

                        try_birth!($dir, $op_dir, Stem(genome), lifespan.0);
                    }
                    CreateSeed(lifespan) => {
                        genome.mutate();
                        genome.active_gene = genome.seed_gene;

                        try_birth!($dir, $op_dir, Stem(genome), lifespan.0);
                    }
                    KillCell => kill_cell!($dir),

                    Nothing => {}
                };
            };
        }

        macro_rules! move_organic {
            ($from: ident, $to: ident) => {{
                let to_move = (255 - area.$to.soil.organics).min(area.$from.soil.organics);
                area.$from.soil.organics -= to_move;
                area.$to.soil.organics += to_move;
            }};
        }

        macro_rules! make_action {
            ($action: ident) => {
                match genome.active_gene().$action {
                    MoveOrganicUp => move_organic!(center, up),
                    MoveOrganicDown => move_organic!(center, down),
                    MoveOrganicLeft => move_organic!(center, left),
                    MoveOrganicRight => move_organic!(center, right),

                    MoveOrganicFromUp => move_organic!(up, center),
                    MoveOrganicFromDown => move_organic!(down, center),
                    MoveOrganicFromLeft => move_organic!(left, center),
                    MoveOrganicFromRight => move_organic!(right, center),

                    DoNothing => {}

                    ChangeActiveGene(gene_location) => {
                        genome.active_gene = gene_location;
                        life.ty = Stem(genome);
                    }

                    KillUpLeft => kill_cell!(up_left),
                    KillUpRight => kill_cell!(up_right),
                    KillDownLeft => kill_cell!(down_left),
                    KillDownRight => kill_cell!(down_right),
                }
            };
        }

        if check_gene_condition(
            state,
            area,
            life,
            genome.active_gene().main_action_condition,
            genome.active_gene().main_action_param,
        ) {
            make_action!(main_action);
        }

        {
            let condition_1 = check_gene_condition(
                state,
                area,
                life,
                genome.active_gene().additional_action_condition1,
                genome.active_gene().additional_action_param1,
            );

            let condition_2 = check_gene_condition(
                state,
                area,
                life,
                genome.active_gene().additional_action_condition2,
                genome.active_gene().additional_action_param2,
            );

            match (condition_1, condition_2) {
                (true, true) => make_action!(additional_action1),
                (true, false) => make_action!(additional_action2),
                (false, true) => make_action!(additional_action3),
                (false, false) => {}
            }
        }

        {
            let condition_1 = check_gene_condition(
                state,
                area,
                life,
                genome.active_gene().condition_1,
                genome.active_gene().param_1,
            );

            let condition_2 = check_gene_condition(
                state,
                area,
                life,
                genome.active_gene().condition_2,
                genome.active_gene().param_2,
            );

            match (condition_1, condition_2) {
                (true, true) => {
                    genome.active_gene = genome.active_gene().alt_gene1;
                    life.ty = Stem(genome);
                }
                (true, false) => {
                    genome.active_gene = genome.active_gene().alt_gene2;
                    life.ty = Stem(genome);
                }
                (false, true) => {
                    genome.active_gene = genome.active_gene().alt_gene3;
                    life.ty = Stem(genome);
                }
                (false, false) => {
                    cell_op_directions_enum!(direction_action);
                }
            };
        }

        // Update parent
        if birth_once {
            life.ty = Pipe;
            life.steps_to_death = genome.active_gene().self_lifespan.0;
        }

        life.energy -= total_energy;
    }
}

fn check_gene_condition(
    state: &State,
    area: &Area<WorldCell>,
    life: &AliveCell,
    condition: GeneCondition,
    param: u8,
) -> bool {
    match condition {
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

        RandomMT => thread_rng().gen::<u8>() > param,
        LifeEnergyMT => life.energy > param as f32,

        OrganicCenterMT => area.center.soil.organics > param,
        OrganicUpMT => area.up.soil.organics > param,
        OrganicDownMT => area.down.soil.organics > param,
        OrganicLeftMT => area.left.soil.organics > param,
        OrganicRightMT => area.right.soil.organics > param,

        SoilEnergyCenterMT => area.center.soil.energy > param as f32,
        SoilEnergyUpMT => area.up.soil.energy > param as f32,
        SoilEnergyDownMT => area.down.soil.energy > param as f32,
        SoilEnergyLeftMT => area.left.soil.energy > param as f32,
        SoilEnergyRightMT => area.right.soil.energy > param as f32,

        AirPollutionCenterMT => area.center.air.pollution > param,
        AirPollutionUpMT => area.up.air.pollution > param,
        AirPollutionDownMT => area.down.air.pollution > param,
        AirPollutionLeftMT => area.left.air.pollution > param,
        AirPollutionRightMT => area.right.air.pollution > param,

        Always => true,
        Never => false,

        StepsDividesP => state.simulation_step % param.max(1) as usize == 0,
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
                    };
                };
            }

            all_directions!(process_organic);

            life.energy += total * 0.6;
            area.center.soil.energy += total * 0.2;
            area.center.air.pollution = area
                .center
                .air
                .pollution
                .saturating_add((total * 0.5) as u8);
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

            life.energy += total * 0.4;
        }
        Filter => {
            let mut total = 0.0;

            macro_rules! process_pollution {
                ($dir: ident) => {
                    if area.$dir.air.pollution <= 8 && area.$dir.air.pollution > 0 {
                        area.$dir.air.pollution -= 1;
                        total += 1.;
                    } else {
                        let pollution = (area.$dir.air.pollution as f32 * 0.16);
                        area.$dir.air.pollution -= pollution as u8;

                        total += pollution;
                    };
                };
            }

            all_directions!(process_pollution);

            life.energy += total * 0.5;
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
    if !life.can_transfer() || life.energy_to.branches_amount() == 0 {
        return;
    }

    let flow_each = {
        let to_flow = if life.steps_to_death == 1 {
            life.energy
        } else {
            (life.energy - 1.1 * life.consumption()).max(0.)
        };

        life.energy -= to_flow;

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
