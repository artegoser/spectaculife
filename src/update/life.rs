use rand::{thread_rng, Rng};

use crate::{
    all_directions, cell_op_directions_enum, cell_op_directions_with_enum,
    cells::{
        life_cell::{
            genome::{
                GeneAction::*,
                GeneCondition::{self, *},
                GeneDirectionAction::*,
                GenomeHandle, GenomePool,
            },
            AliveCell,
            LifeCell::*,
            LifeType::*,
            MAX_ENERGY_TRANSFER,
        },
        soil_cell::{MAX_ENERGY_LIFE, MAX_ORGANIC_LIFE},
        WorldCell,
    },
    config::SimulationConfig,
    grid::Area,
    types::{
        CellDir::{self, *},
        State,
    },
};

pub fn update_life(
    state: &mut State,
    area: &mut Area<WorldCell>,
    genomes: &mut GenomePool,
    config: &SimulationConfig,
) {
    if let Alive(mut life) = area.center.life {
        if life.steps_to_death == 0 {
            return kill(area, genomes);
        } else {
            life.steps_to_death -= 1;
        }

        if ((area.center.soil.organics > MAX_ORGANIC_LIFE) && (life.ty != Root))
            || ((area.center.soil.energy > MAX_ENERGY_LIFE) && (life.ty != Reactor))
        {
            return kill(area, genomes);
        }

        life.energy -= life.consumption();

        if life.energy < 0. {
            return kill(area, genomes);
        }

        if (life.energy_to.branches_amount() == 0) && !life.is_fertile() {
            if life.is_pipe() {
                if let Some(parent_dir) = life.parent_dir {
                    reroute_energy_paths(area, &mut life, parent_dir);
                } else {
                    return kill(area, genomes);
                }
            } else {
                return kill(area, genomes);
            }
        }

        generate_energy(area, &mut life);

        // Transfer energy
        transfer_energy(area, &mut life);

        // Process genome
        match life.ty {
            Stem(handle) => {
                if !process_genome(state, area, &mut life, handle, genomes, config) {
                    return;
                }
            }
            _ => {}
        };

        area.center.life = Alive(life);
    }
}

fn process_genome(
    state: &mut State,
    area: &mut Area<WorldCell>,
    life: &mut AliveCell,
    handle: GenomeHandle,
    genomes: &mut GenomePool,
    config: &SimulationConfig,
) -> bool {
    let gene_snapshot = genomes.get(handle).active_gene();

    // Preserve the old energy gate for actions of the currently active gene.
    if life.energy <= gene_snapshot.energy_capacity() {
        return true;
    }

    let mut birth_once = false;
    let mut next_active_gene = genomes.get(handle).active_gene;

    macro_rules! collide_or_birth {
        ($dir: ident, $op_dir: ident, $cell_type: expr, $steps_to_death: expr, $organism_id: expr) => {{
            if let Alive(mut target) = area.$dir.life {
                // Failed growth used to damage the organism's own tissue. Only
                // competition with another organism causes collision damage.
                if target.organism_id != life.organism_id {
                    target.steps_to_death = target
                                .steps_to_death
                                .saturating_sub(config.world.collision_damage);
                    area.$dir.life = Alive(target);
                }
                false
            } else {
                if $cell_type.is_fertile() {
                    life.energy_to.$dir = true;
                }

                area.$dir.life = $cell_type.make_newborn_cell(
                    $organism_id,
                    $op_dir,
                    $steps_to_death,
                );
                birth_once = true;
                true
            }
        }};
    }

    macro_rules! kill_cell {
        ($dir:ident) => {
            if let Alive(mut target) = area.$dir.life {
                life.energy += target.energy.min(MAX_ENERGY_TRANSFER);
                target.steps_to_death = 0;
                area.$dir.life = Alive(target);
            }
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
            match gene_snapshot.$action {
                MoveOrganicUp => move_organic!(center, up),
                MoveOrganicDown => move_organic!(center, down),
                MoveOrganicLeft => move_organic!(center, left),
                MoveOrganicRight => move_organic!(center, right),

                MoveOrganicFromUp => move_organic!(up, center),
                MoveOrganicFromDown => move_organic!(down, center),
                MoveOrganicFromLeft => move_organic!(left, center),
                MoveOrganicFromRight => move_organic!(right, center),

                DoNothing => {}

                ChangeActiveGene(gene_location) => next_active_gene = gene_location,

                KillUpLeft => kill_cell!(up_left),
                KillUpRight => kill_cell!(up_right),
                KillDownLeft => kill_cell!(down_left),
                KillDownRight => kill_cell!(down_right),

                WaitStep => return true,
                Die => {
                    // update_life works on a local copy. Synchronize it before
                    // kill() so the cell stays dead and the genome is freed once.
                    area.center.life = Alive(*life);
                    kill(area, genomes);
                    return false;
                }
            }
        };
    }

    if check_gene_condition(
        state,
        area,
        life,
        gene_snapshot.main_action_condition,
        gene_snapshot.main_action_param,
    ) {
        make_action!(main_action);
    }

    {
        let condition_1 = check_gene_condition(
            state,
            area,
            life,
            gene_snapshot.additional_action_condition1,
            gene_snapshot.additional_action_param1,
        );

        let condition_2 = check_gene_condition(
            state,
            area,
            life,
            gene_snapshot.additional_action_condition2,
            gene_snapshot.additional_action_param2,
        );

        match (condition_1, condition_2) {
            (true, true) => make_action!(additional_action1),
            (true, false) => make_action!(additional_action2),
            (false, true) => make_action!(additional_action3),
            (false, false) => {}
        }
    }

    // Select the gene that is expressed *this* growth step. Previously the
    // alternate gene was assigned to the parent only after it had already
    // spawned children, and the parent then became Pipe, so conditions almost
    // never influenced morphology.
    {
        let condition_1 = check_gene_condition(
            state,
            area,
            life,
            gene_snapshot.condition_1,
            gene_snapshot.param_1,
        );

        let condition_2 = check_gene_condition(
            state,
            area,
            life,
            gene_snapshot.condition_2,
            gene_snapshot.param_2,
        );

        match (condition_1, condition_2) {
            (true, true) => next_active_gene = gene_snapshot.alt_gene1,
            (true, false) => next_active_gene = gene_snapshot.alt_gene2,
            (false, true) => next_active_gene = gene_snapshot.alt_gene3,
            (false, false) => {}
        }
    }

    let growth_gene_snapshot = genomes.get(handle).get_gene(next_active_gene);
    let total_energy = growth_gene_snapshot.energy_capacity();

    if life.energy <= total_energy {
        genomes.get_mut(handle).active_gene = next_active_gene;
        return true;
    }

    macro_rules! direction_action {
        ($dir: ident, $op_dir: ident) => {
            match growth_gene_snapshot.$dir {
                MakeLeaf(lifespan) => {
                    collide_or_birth!($dir, $op_dir, Leaf, lifespan.0, life.organism_id);
                }
                MakeRoot(lifespan) => {
                    collide_or_birth!($dir, $op_dir, Root, lifespan.0, life.organism_id);
                }
                MakeReactor(lifespan) => {
                    collide_or_birth!($dir, $op_dir, Reactor, lifespan.0, life.organism_id);
                }
                MakeFilter(lifespan) => {
                    collide_or_birth!($dir, $op_dir, Filter, lifespan.0, life.organism_id);
                }
                MultiplySelf(lifespan, next_gene) => {
                    if let Alive(mut target) = area.$dir.life {
                        if target.organism_id != life.organism_id {
                            target.steps_to_death = target
                                .steps_to_death
                                .saturating_sub(config.world.collision_damage);
                            area.$dir.life = Alive(target);
                        }
                    } else {
                        // Somatic growth inherits the genome exactly. Mutation is
                        // reserved for CreateSeed, i.e. a new organism/generation.
                        let mut child_genome = *genomes.get(handle);
                        child_genome.active_gene = next_gene;
                        let child_handle = genomes.alloc(child_genome);
                        collide_or_birth!(
                            $dir,
                            $op_dir,
                            Stem(child_handle),
                            lifespan.0,
                            life.organism_id
                        );
                    }
                }
                CreateSeed(lifespan) => {
                    if let Alive(mut target) = area.$dir.life {
                        if target.organism_id != life.organism_id {
                            target.steps_to_death = target
                                .steps_to_death
                                .saturating_sub(config.world.collision_damage);
                            area.$dir.life = Alive(target);
                        }
                    } else {
                        let mut child_genome = *genomes.get(handle);
                        child_genome.mutate(&config.genetics);
                        child_genome.active_gene = child_genome.seed_gene;
                        let child_handle = genomes.alloc(child_genome);
                        let child_organism_id = state.allocate_organism_id();
                        collide_or_birth!(
                            $dir,
                            $op_dir,
                            Stem(child_handle),
                            lifespan.0,
                            child_organism_id
                        );
                    }
                }
                KillCell => kill_cell!($dir),
                Nothing => {}
            };
        };
    }

    cell_op_directions_enum!(direction_action);

    let parent_lifespan = growth_gene_snapshot.self_lifespan.0;
    if birth_once {
        genomes.free(handle);
        life.ty = Pipe;
        life.steps_to_death = parent_lifespan;
    } else {
        genomes.get_mut(handle).active_gene = next_active_gene;
    }

    life.energy -= total_energy;
    true
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
            let total = 1.2 / (area.center.air.pollution as f32 / 4.).max(1.);
            life.energy += total;
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
                        let pollution = area.$dir.air.pollution as f32 * 0.16;
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
            (life.energy - 1.1 * life.consumption())
                .min(MAX_ENERGY_TRANSFER)
                .max(0.)
        };

        life.energy -= to_flow;
        to_flow / (life.energy_to.branches_amount() as f32)
    };

    macro_rules! transfer {
        ($dir: ident) => {
            if life.energy_to.$dir {
                if let Alive(mut $dir) = area.$dir.life {
                    if $dir.is_pipe_recipient() {
                        $dir.incoming_energy += flow_each;
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

    transfer!(up);
    transfer!(down);
    transfer!(left);
    transfer!(right);
}

/// Kill cell and reroute energy paths
fn kill(area: &mut Area<WorldCell>, genomes: &mut GenomePool) {
    // Free genome if this was a Stem cell
    if let Alive(life) = area.center.life {
        if let Stem(handle) = life.ty {
            genomes.free(handle);
        }
    }

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
