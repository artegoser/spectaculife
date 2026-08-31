use rand::Rng;
use rayon::prelude::*;

use crate::{
    cells::{
        life_cell::{
            genome::{
                GeneAction, GeneCondition, GeneDirectionAction, GeneLocation, Genome, GenomeHandle,
                GenomePool,
            },
            AliveCell, EnergyDirections, LifeCell, LifeType, SeedState,
        },
        WorldCell,
    },
    config::{LifeConfig, SimulationConfig},
    grid::Grid,
    types::{CellDir, State},
};

const MOORE_OFFSETS: [(i64, i64); 9] = [
    (0, 0),
    (0, -1),
    (0, 1),
    (-1, 0),
    (1, 0),
    (-1, -1),
    (1, -1),
    (-1, 1),
    (1, 1),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum GeneratorKind {
    Root,
    Reactor,
    Filter,
}

#[derive(Debug, Clone)]
struct GenerationPlan {
    source: usize,
    kind: GeneratorKind,
    targets: [usize; 9],
    requests: [f32; 9],
}

#[derive(Debug, Clone, Copy)]
struct DemandEvent {
    kind: GeneratorKind,
    target: usize,
    amount: f32,
}

#[derive(Debug, Clone, Copy)]
struct DemandTotal {
    kind: GeneratorKind,
    target: usize,
    amount: f32,
}

#[derive(Debug, Clone, Copy)]
struct GeneratorEffect {
    source: usize,
    energy_gain: f32,
    soil_output: f32,
    pollution_output: f32,
}

#[derive(Debug, Clone)]
struct EnergyPlan {
    source: usize,
    energy_after: f32,
    directions_after: EnergyDirections,
    outgoing: [(usize, f32); 4],
    outgoing_len: usize,
}

#[derive(Debug, Clone, Copy)]
struct OrganicMove {
    from: usize,
    to: usize,
    amount: u8,
}

#[derive(Debug, Clone, Copy)]
enum OrganicPos {
    Center,
    Direction(CellDir),
}

#[derive(Debug, Clone, Copy)]
struct LocalOrganics {
    center: u8,
    up: u8,
    down: u8,
    left: u8,
    right: u8,
}

impl LocalOrganics {
    fn from_grid(grid: &Grid<WorldCell>, source: usize) -> Self {
        Self {
            center: grid.cells()[source].soil.organics,
            up: grid.cells()[neighbor_index(grid, source, CellDir::Up)]
                .soil
                .organics,
            down: grid.cells()[neighbor_index(grid, source, CellDir::Down)]
                .soil
                .organics,
            left: grid.cells()[neighbor_index(grid, source, CellDir::Left)]
                .soil
                .organics,
            right: grid.cells()[neighbor_index(grid, source, CellDir::Right)]
                .soil
                .organics,
        }
    }

    fn get(&self, pos: OrganicPos) -> u8 {
        match pos {
            OrganicPos::Center => self.center,
            OrganicPos::Direction(CellDir::Up) => self.up,
            OrganicPos::Direction(CellDir::Down) => self.down,
            OrganicPos::Direction(CellDir::Left) => self.left,
            OrganicPos::Direction(CellDir::Right) => self.right,
        }
    }

    fn set(&mut self, pos: OrganicPos, value: u8) {
        match pos {
            OrganicPos::Center => self.center = value,
            OrganicPos::Direction(CellDir::Up) => self.up = value,
            OrganicPos::Direction(CellDir::Down) => self.down = value,
            OrganicPos::Direction(CellDir::Left) => self.left = value,
            OrganicPos::Direction(CellDir::Right) => self.right = value,
        }
    }

    fn move_all(&mut self, from: OrganicPos, to: OrganicPos) -> u8 {
        let source = self.get(from);
        let target = self.get(to);
        let amount = (u8::MAX - target).min(source);
        self.set(from, source - amount);
        self.set(to, target + amount);
        amount
    }
}

#[derive(Debug, Clone)]
enum BirthKind {
    Leaf,
    Root,
    Reactor,
    Filter,
    Stem { genome: Genome },
    Seed { genome: Genome },
}

#[derive(Debug, Clone)]
struct BirthRequest {
    target: usize,
    parent_dir: CellDir,
    lifespan: u16,
    kind: BirthKind,
    won: bool,
}

#[derive(Debug, Clone)]
struct GenomePlan {
    source: usize,
    handle: GenomeHandle,
    organism_id: u64,
    energy_after: f32,
    active_gene_update: Option<GeneLocation>,
    parent_lifespan: Option<u16>,
    organic_moves: Vec<OrganicMove>,
    kill_targets: Vec<usize>,
    collision_targets: Vec<(usize, u16)>,
    births: Vec<BirthRequest>,
    die: bool,
}

impl GenomePlan {
    fn passive(source: usize, handle: GenomeHandle, organism_id: u64, energy: f32) -> Self {
        Self {
            source,
            handle,
            organism_id,
            energy_after: energy,
            active_gene_update: None,
            parent_lifespan: None,
            organic_moves: Vec::new(),
            kill_targets: Vec::new(),
            collision_targets: Vec::new(),
            births: Vec::new(),
            die: false,
        }
    }
}

#[derive(Default)]
struct LifeCensus {
    alive: Vec<usize>,
    generators: Vec<usize>,
    transferable: Vec<usize>,
    stems: Vec<usize>,
    seeds: Vec<usize>,
}

impl LifeCensus {
    fn push(&mut self, index: usize, life: AliveCell) {
        self.alive.push(index);
        if life.is_energy_generator() {
            self.generators.push(index);
        }
        if life.can_transfer() {
            self.transferable.push(index);
        }
        if life.is_fertile() {
            self.stems.push(index);
        }
        if life.is_seed() {
            self.seeds.push(index);
        }
    }

    fn append(&mut self, mut other: Self) {
        self.alive.append(&mut other.alive);
        self.generators.append(&mut other.generators);
        self.transferable.append(&mut other.transferable);
        self.stems.append(&mut other.stems);
        self.seeds.append(&mut other.seeds);
    }
}

/// Execute one synchronous life tick. Each phase first computes from a stable
/// world state and only then commits neighbor effects, so scan order cannot
/// create same-tick propagation chains.
///
/// 1. Deliver energy queued by the previous tick.
/// 2. Apply local maintenance/deaths.
/// 3. Repair broken energy topology.
/// 4. Resolve generator resource claims.
/// 5. Queue one-hop energy transfers.
/// 6. Evaluate genomes in parallel and commit growth/actions.
pub fn update_life_step(
    state: &mut State,
    grid: &mut Grid<WorldCell>,
    genomes: &mut GenomePool,
    config: &SimulationConfig,
    next_soil_energy: &[f32],
    next_pollution: &[f32],
) {
    // Delivery + maintenance + the sparse life census share one full-grid pass.
    // All later life phases operate only on the cells that can actually act.
    let census = update_maintenance_and_census(
        grid,
        genomes,
        next_soil_energy,
        next_pollution,
        &config.life,
    );
    mature_seeds(state, grid, &census.seeds, &config.life);
    repair_energy_paths(grid, genomes, &census.alive, &config.life);
    generate_energy(grid, &census.generators, &config.life);
    transfer_energy_one_hop(grid, &census.transferable, &config.life);
    process_genomes(state, grid, genomes, config, &census.stems);
}

/// Deliver tick N-1 energy, perform maintenance, and build sparse work lists in
/// a single parallel scan. New energy is only written to `incoming_energy`
/// later in the tick, preserving the strict one-cell-per-tick propagation rule.
fn update_maintenance_and_census(
    grid: &mut Grid<WorldCell>,
    genomes: &mut GenomePool,
    next_soil_energy: &[f32],
    next_pollution: &[f32],
    config: &LifeConfig,
) -> LifeCensus {
    debug_assert_eq!(next_soil_energy.len(), grid.len());
    debug_assert_eq!(next_pollution.len(), grid.len());
    let (census, deaths) = grid
        .cells_mut()
        .par_iter_mut()
        .enumerate()
        .fold(
            || (LifeCensus::default(), Vec::<usize>::new()),
            |(mut census, mut deaths), (index, cell)| {
                cell.soil.energy = next_soil_energy[index];
                cell.air.pollution = next_pollution[index].round().clamp(0.0, 255.0) as u8;

                let LifeCell::Alive(mut life) = cell.life else {
                    return (census, deaths);
                };

                life.energy += life.incoming_energy;
                life.incoming_energy = 0.0;

                let mut dead = life.steps_to_death == 0;
                if !dead {
                    life.steps_to_death -= 1;
                    dead = (cell.soil.organics > config.lethal_organics && life.ty != LifeType::Root)
                        || (cell.soil.energy > config.lethal_soil_energy && life.ty != LifeType::Reactor);
                }
                if !dead {
                    life.energy -= life.consumption(config);
                    dead = life.energy < 0.0;
                }

                cell.life = LifeCell::Alive(life);
                if dead {
                    deaths.push(index);
                } else {
                    census.push(index, life);
                }
                (census, deaths)
            },
        )
        .reduce(
            || (LifeCensus::default(), Vec::<usize>::new()),
            |(mut left_census, mut left_deaths), (right_census, mut right_deaths)| {
                left_census.append(right_census);
                left_deaths.append(&mut right_deaths);
                (left_census, left_deaths)
            },
        );

    for index in deaths {
        kill_index(grid, index, genomes, config);
    }

    census
}

/// Germinate fully charged seeds.  Until this point the seed belongs to the
/// parent's body and is fed through the parent's energy edge.  Maturation is
/// the exact boundary where it becomes a new organism.
fn mature_seeds(
    state: &mut State,
    grid: &mut Grid<WorldCell>,
    seed_indices: &[usize],
    config: &LifeConfig,
) {
    let mut ordered = seed_indices.to_vec();
    ordered.sort_unstable();

    for index in ordered {
        let LifeCell::Alive(mut seed_life) = grid.cells()[index].life else {
            continue;
        };
        let LifeType::Seed(seed) = seed_life.ty else {
            continue;
        };
        if seed_life.energy < config.reproduction.seed_maturation_energy {
            continue;
        }

        // Remove the parent's outgoing edge before the child starts living as
        // an independent organism.  If the parent has already died, kill_index
        // has cleared parent_dir and there is simply no edge left to remove.
        if let Some(parent_dir) = seed_life.parent_dir {
            let parent = neighbor_index(grid, index, parent_dir);
            if let LifeCell::Alive(mut parent_life) = grid.cells()[parent].life {
                parent_life.energy_to.set(parent_dir.opposite(), false);
                grid.cells_mut()[parent].life = LifeCell::Alive(parent_life);
            }
        }

        seed_life.ty = LifeType::Stem(seed.genome);
        seed_life.organism_id = state.allocate_organism_id();
        seed_life.parent_dir = None;
        seed_life.energy_to = EnergyDirections::default();
        seed_life.steps_to_death = seed.stem_lifespan;
        grid.cells_mut()[index].life = LifeCell::Alive(seed_life);
    }
}

fn repair_energy_paths(
    grid: &mut Grid<WorldCell>,
    genomes: &mut GenomePool,
    alive_indices: &[usize],
    config: &LifeConfig,
) {
    #[derive(Clone, Copy)]
    struct PathPlan {
        source: usize,
        die: bool,
        connect: Option<(CellDir, usize)>,
    }

    let plans: Vec<PathPlan> = alive_indices
        .par_iter()
        .filter_map(|&index| {
            let LifeCell::Alive(life) = grid.cells()[index].life else {
                return None;
            };
            if life.energy_to.branches_amount() != 0 || life.is_fertile() || life.is_seed() {
                return None;
            }

            if !life.is_pipe() {
                return Some(PathPlan { source: index, die: true, connect: None });
            }

            let Some(parent_dir) = life.parent_dir else {
                return Some(PathPlan { source: index, die: true, connect: None });
            };

            let target = neighbor_index(grid, index, parent_dir);
            let connect = grid.cells()[target]
                .life
                .is_alive()
                .then_some((parent_dir, target));
            Some(PathPlan { source: index, die: false, connect })
        })
        .collect();

    // Sparse edge edits replace two world-sized set/clear masks. Edits are
    // sorted by target cell so contradictory requests remain deterministic;
    // clear wins over set just like the previous staged implementation.
    let mut edits: Vec<(usize, u8, u8)> = Vec::with_capacity(plans.len() * 2);
    for plan in &plans {
        if let Some((dir, target)) = plan.connect {
            edits.push((plan.source, direction_bit(dir), 0));
            edits.push((target, 0, direction_bit(dir.opposite())));
        }
    }
    edits.sort_unstable_by_key(|edit| edit.0);

    let mut i = 0;
    while i < edits.len() {
        let index = edits[i].0;
        let mut set_mask = 0_u8;
        let mut clear_mask = 0_u8;
        while i < edits.len() && edits[i].0 == index {
            set_mask |= edits[i].1;
            clear_mask |= edits[i].2;
            i += 1;
        }

        if let LifeCell::Alive(mut life) = grid.cells()[index].life {
            for dir in CellDir::ALL {
                let bit = direction_bit(dir);
                if set_mask & bit != 0 {
                    life.energy_to.set(dir, true);
                }
                if clear_mask & bit != 0 {
                    life.energy_to.set(dir, false);
                }
            }
            grid.cells_mut()[index].life = LifeCell::Alive(life);
        }
    }

    for plan in plans {
        if plan.die {
            kill_index(grid, plan.source, genomes, config);
        }
    }
}

#[inline(always)]
const fn direction_bit(dir: CellDir) -> u8 {
    match dir {
        CellDir::Up => 1 << 0,
        CellDir::Down => 1 << 1,
        CellDir::Left => 1 << 2,
        CellDir::Right => 1 << 3,
    }
}

fn generate_energy(grid: &mut Grid<WorldCell>, generator_indices: &[usize], config: &LifeConfig) {
    // Leaves need no arbitration and are kept as tiny sparse effects.
    let leaf_effects: Vec<GeneratorEffect> = generator_indices
        .par_iter()
        .filter_map(|&source| {
            let LifeCell::Alive(life) = grid.cells()[source].life else {
                return None;
            };
            (life.ty == LifeType::Leaf).then(|| GeneratorEffect {
                source,
                energy_gain: config.generators.leaf.energy_per_tick
                    / (grid.cells()[source].air.pollution as f32
                        / config.generators.leaf.pollution_divisor)
                        .max(1.0),
                soil_output: 0.0,
                pollution_output: 0.0,
            })
        })
        .collect();

    let plans: Vec<GenerationPlan> = generator_indices
        .par_iter()
        .filter_map(|&source| {
            let LifeCell::Alive(life) = grid.cells()[source].life else {
                return None;
            };

            let kind = match life.ty {
                LifeType::Root => GeneratorKind::Root,
                LifeType::Reactor => GeneratorKind::Reactor,
                LifeType::Filter => GeneratorKind::Filter,
                _ => return None,
            };

            let targets = std::array::from_fn(|i| {
                let (dx, dy) = MOORE_OFFSETS[i];
                grid.offset_index(source, dx, dy)
            });
            let requests = std::array::from_fn(|i| {
                let target = &grid.cells()[targets[i]];
                match kind {
                    GeneratorKind::Root => {
                        let value = target.soil.organics;
                        if value == 0 {
                            0.0
                        } else if value <= config.generators.root.low_resource_threshold {
                            config.generators.root.low_resource_take.min(value) as f32
                        } else {
                            ((value as f32) * config.generators.root.extraction_fraction)
                                .floor()
                                .max(config.generators.root.low_resource_take as f32)
                                .min(value as f32)
                        }
                    }
                    GeneratorKind::Reactor => {
                        target.soil.energy * config.generators.reactor.extraction_fraction
                    }
                    GeneratorKind::Filter => {
                        let value = target.air.pollution;
                        if value == 0 {
                            0.0
                        } else if value <= config.generators.filter.low_resource_threshold {
                            config.generators.filter.low_resource_take.min(value) as f32
                        } else {
                            ((value as f32) * config.generators.filter.extraction_fraction)
                                .floor()
                                .max(config.generators.filter.low_resource_take as f32)
                                .min(value as f32)
                        }
                    }
                }
            });

            Some(GenerationPlan { source, kind, targets, requests })
        })
        .collect();

    // Resource contention is sparse: at most nine demand records per actual
    // Root/Reactor/Filter instead of six+ arrays the size of the whole world.
    let mut demands = Vec::<DemandEvent>::with_capacity(plans.len() * 9);
    for plan in &plans {
        for i in 0..9 {
            let amount = plan.requests[i];
            if amount != 0.0 {
                demands.push(DemandEvent {
                    kind: plan.kind,
                    target: plan.targets[i],
                    amount,
                });
            }
        }
    }
    demands.sort_unstable_by_key(|event| (event.kind, event.target));

    let mut totals = Vec::<DemandTotal>::with_capacity(demands.len());
    for event in demands {
        if let Some(last) = totals.last_mut() {
            if last.kind == event.kind && last.target == event.target {
                last.amount += event.amount;
                continue;
            }
        }
        totals.push(DemandTotal {
            kind: event.kind,
            target: event.target,
            amount: event.amount,
        });
    }

    let effects: Vec<GeneratorEffect> = plans
        .par_iter()
        .map(|plan| {
            let mut allocated = 0.0_f32;
            for i in 0..9 {
                let request = plan.requests[i];
                if request == 0.0 {
                    continue;
                }
                let target = plan.targets[i];
                let demand = total_demand(&totals, plan.kind, target);
                let available = match plan.kind {
                    GeneratorKind::Root => grid.cells()[target].soil.organics as f32,
                    GeneratorKind::Reactor => grid.cells()[target].soil.energy,
                    GeneratorKind::Filter => grid.cells()[target].air.pollution as f32,
                };
                allocated += request * demand_scale(available, demand);
            }

            match plan.kind {
                GeneratorKind::Root => GeneratorEffect {
                    source: plan.source,
                    energy_gain: allocated * config.generators.root.energy_efficiency,
                    soil_output: allocated * config.generators.root.soil_energy_output,
                    pollution_output: allocated * config.generators.root.pollution_output,
                },
                GeneratorKind::Reactor => GeneratorEffect {
                    source: plan.source,
                    energy_gain: allocated * config.generators.reactor.energy_efficiency,
                    soil_output: 0.0,
                    pollution_output: 0.0,
                },
                GeneratorKind::Filter => GeneratorEffect {
                    source: plan.source,
                    energy_gain: allocated * config.generators.filter.energy_efficiency,
                    soil_output: 0.0,
                    pollution_output: 0.0,
                },
            }
        })
        .collect();

    // Consume only resources that were touched by a generator. Root outputs are
    // applied afterwards, so newly produced soil/pollution cannot be consumed
    // again in the same tick.
    for total in &totals {
        let cell = &mut grid.cells_mut()[total.target];
        match total.kind {
            GeneratorKind::Root => {
                let used = total.amount.min(cell.soil.organics as f32).round() as u8;
                cell.soil.organics = cell.soil.organics.saturating_sub(used);
            }
            GeneratorKind::Reactor => {
                let used = total.amount.min(cell.soil.energy);
                cell.soil.energy = (cell.soil.energy - used).max(0.0);
            }
            GeneratorKind::Filter => {
                let used = total.amount.min(cell.air.pollution as f32).round() as u8;
                cell.air.pollution = cell.air.pollution.saturating_sub(used);
            }
        }
    }

    for effect in leaf_effects.into_iter().chain(effects) {
        let cell = &mut grid.cells_mut()[effect.source];
        if let LifeCell::Alive(mut life) = cell.life {
            life.energy += effect.energy_gain;
            cell.life = LifeCell::Alive(life);
        }
        cell.soil.energy += effect.soil_output;
        cell.air.pollution = cell.air.pollution.saturating_add(
            effect.pollution_output.round().clamp(0.0, 255.0) as u8,
        );
    }
}

#[inline]
fn total_demand(totals: &[DemandTotal], kind: GeneratorKind, target: usize) -> f32 {
    totals
        .binary_search_by_key(&(kind, target), |total| (total.kind, total.target))
        .ok()
        .map(|index| totals[index].amount)
        .unwrap_or(0.0)
}

#[inline]
fn demand_scale(available: f32, demand: f32) -> f32 {
    if demand <= 0.0 {
        0.0
    } else {
        (available / demand).clamp(0.0, 1.0)
    }
}

fn transfer_energy_one_hop(grid: &mut Grid<WorldCell>, transferable_indices: &[usize], config: &LifeConfig) {
    let plans: Vec<EnergyPlan> = transferable_indices
        .par_iter()
        .filter_map(|&source| {
            let LifeCell::Alive(life) = grid.cells()[source].life else {
                return None;
            };

            if !life.can_transfer() || life.energy_to.branches_amount() == 0 {
                return None;
            }

            let mut directions = life.energy_to;
            let mut targets = [0_usize; 4];
            let mut target_caps = [f32::INFINITY; 4];
            let mut target_count = 0_usize;
            for dir in CellDir::ALL {
                if !life.energy_to.get(dir) {
                    continue;
                }
                let target = neighbor_index(grid, source, dir);
                match grid.cells()[target].life {
                    LifeCell::Alive(target_life) if target_life.is_pipe_recipient() => {
                        targets[target_count] = target;
                        if target_life.is_seed() {
                            // Incoming energy is delivered before the next maintenance
                            // charge. Aim one maintenance above the maturation threshold
                            // so the seed can actually cross it instead of asymptotically
                            // topping up to threshold and immediately falling below it.
                            target_caps[target_count] = config
                                .reproduction
                                .seed_max_charge_per_tick
                                .max(0.0)
                                .min(
                                    (config.reproduction.seed_maturation_energy
                                        + config.consumption.seed
                                        - target_life.energy)
                                        .max(0.0),
                                );
                        }
                        target_count += 1;
                    }
                    _ => directions.set(dir, false),
                }
            }

            if target_count == 0 {
                return Some(EnergyPlan {
                    source,
                    energy_after: life.energy,
                    directions_after: directions,
                    outgoing: [(0, 0.0); 4],
                    outgoing_len: 0,
                });
            }

            let to_flow = if life.steps_to_death == 1 {
                life.energy.max(0.0)
            } else {
                let available = (life.energy
                    - config.transfer.reserve_consumption_multiplier * life.consumption(config))
                    .max(0.0);
                match config.transfer.max_energy_per_tick {
                    Some(cap) => available.min(cap),
                    None => available,
                }
            };
            let flow_each = to_flow / target_count as f32;
            let mut outgoing = [(0_usize, 0.0_f32); 4];
            let mut actual_flow = 0.0;
            for i in 0..target_count {
                let amount = flow_each.min(target_caps[i]);
                outgoing[i] = (targets[i], amount);
                actual_flow += amount;
            }

            Some(EnergyPlan {
                source,
                // Energy a capped seed cannot accept stays in the source cell.
                energy_after: life.energy - actual_flow,
                directions_after: directions,
                outgoing,
                outgoing_len: target_count,
            })
        })
        .collect();

    for plan in plans {
        if let LifeCell::Alive(mut life) = grid.cells()[plan.source].life {
            life.energy = plan.energy_after;
            life.energy_to = plan.directions_after;
            grid.cells_mut()[plan.source].life = LifeCell::Alive(life);
        }
        for &(target, amount) in &plan.outgoing[..plan.outgoing_len] {
            if amount == 0.0 {
                continue;
            }
            if let LifeCell::Alive(mut life) = grid.cells()[target].life {
                life.incoming_energy += amount;
                grid.cells_mut()[target].life = LifeCell::Alive(life);
            }
        }
    }
}

fn process_genomes(
    state: &mut State,
    grid: &mut Grid<WorldCell>,
    genomes: &mut GenomePool,
    config: &SimulationConfig,
    stem_indices: &[usize],
) {
    let step = state.simulation_step;

    let mut plans: Vec<GenomePlan> = stem_indices
        .par_iter()
        .filter_map(|&source| {
            let LifeCell::Alive(life) = grid.cells()[source].life else {
                return None;
            };
            let LifeType::Stem(handle) = life.ty else {
                return None;
            };
            Some(build_genome_plan(
                source, life, handle, grid, genomes, step, config,
            ))
        })
        .collect();

    // Birth arbitration is sparse and proportional to attempted births, not to
    // world area. The first candidate after sorting by (target, random key) wins.
    let mut birth_candidates = Vec::<(usize, u64, usize, usize)>::new();
    for (plan_idx, plan) in plans.iter().enumerate() {
        for (birth_idx, birth) in plan.births.iter().enumerate() {
            birth_candidates.push((
                birth.target,
                arbitration_key(
                    step as u64,
                    plan.source as u64,
                    birth.target as u64,
                    birth_idx as u64,
                ),
                plan_idx,
                birth_idx,
            ));
        }
    }
    birth_candidates.sort_unstable_by_key(|candidate| (candidate.0, candidate.1, candidate.2, candidate.3));
    let mut last_target = None;
    for (target, _, plan_idx, birth_idx) in birth_candidates {
        if last_target == Some(target) {
            continue;
        }
        plans[plan_idx].births[birth_idx].won = true;
        last_target = Some(target);
    }

    // Organic groups remain ordered by deterministic per-tick arbitration, but
    // only plan indices are sorted; the movement vectors themselves are not cloned.
    let mut organic_groups = Vec::<(u64, usize)>::new();
    for (plan_idx, plan) in plans.iter().enumerate() {
        if let Some(first) = plan.organic_moves.first() {
            organic_groups.push((
                arbitration_key(step as u64, plan.source as u64, first.to as u64, 0),
                plan_idx,
            ));
        }
    }
    organic_groups.sort_unstable_by_key(|group| (group.0, group.1));
    for (_, plan_idx) in organic_groups {
        for &movement in &plans[plan_idx].organic_moves {
            move_organic(grid, movement);
        }
    }

    // External effects are also sparse. Sorting groups all attacks on the same
    // target and preserves the rule that KillCell overrides collision damage.
    let mut external_effects = Vec::<(usize, bool, u32)>::new();
    for plan in &plans {
        external_effects.extend(
            plan.collision_targets
                .iter()
                .copied()
                .map(|(target, damage)| (target, false, damage as u32)),
        );
        external_effects.extend(
            plan.kill_targets
                .iter()
                .copied()
                .map(|target| (target, true, 0)),
        );
    }
    external_effects.sort_unstable_by_key(|effect| effect.0);

    // Commit every Stem's own state first. External effects are intentionally
    // delayed until all births/state changes are done so they cannot be overwritten.
    for plan in plans {
        if plan.die {
            kill_index(grid, plan.source, genomes, &config.life);
            continue;
        }

        let LifeCell::Alive(mut center) = grid.cells()[plan.source].life else {
            continue;
        };
        let LifeType::Stem(current_handle) = center.ty else {
            continue;
        };
        if current_handle != plan.handle {
            continue;
        }

        center.energy = plan.energy_after;
        let mut birthed = false;

        for birth in plan.births {
            if !birth.won || grid.cells()[birth.target].life.is_alive() {
                continue;
            }

            let (organism_id, ty, parent_dir, initial_energy, connect_to_parent, lifespan) =
                match birth.kind {
                    BirthKind::Leaf => (
                        plan.organism_id,
                        LifeType::Leaf,
                        Some(birth.parent_dir),
                        None,
                        false,
                        birth.lifespan,
                    ),
                    BirthKind::Root => (
                        plan.organism_id,
                        LifeType::Root,
                        Some(birth.parent_dir),
                        None,
                        false,
                        birth.lifespan,
                    ),
                    BirthKind::Reactor => (
                        plan.organism_id,
                        LifeType::Reactor,
                        Some(birth.parent_dir),
                        None,
                        false,
                        birth.lifespan,
                    ),
                    BirthKind::Filter => (
                        plan.organism_id,
                        LifeType::Filter,
                        Some(birth.parent_dir),
                        None,
                        false,
                        birth.lifespan,
                    ),
                    BirthKind::Stem { genome } => (
                        plan.organism_id,
                        LifeType::Stem(genomes.alloc(genome)),
                        Some(birth.parent_dir),
                        None,
                        true,
                        birth.lifespan,
                    ),
                    BirthKind::Seed { genome } => (
                        // A developing seed is still physically part of the
                        // parent's body.  Its new organism id is allocated only
                        // on maturation/detachment.
                        plan.organism_id,
                        LifeType::Seed(SeedState {
                            genome: genomes.alloc(genome),
                            stem_lifespan: birth.lifespan,
                        }),
                        Some(birth.parent_dir),
                        Some(config.life.reproduction.seed_initial_energy),
                        true,
                        config.life.reproduction.seed_lifespan,
                    ),
                };

            let mut child = ty.make_newborn_cell(
                organism_id,
                parent_dir,
                lifespan,
                &config.life,
            );
            if let Some(initial_energy) = initial_energy {
                if let LifeCell::Alive(child_life) = &mut child {
                    child_life.energy = initial_energy;
                }
            }
            grid.cells_mut()[birth.target].life = child;

            // Somatic growth and an immature seed both extend the parent's
            // energy graph.  The seed edge is removed exactly at maturation.
            if connect_to_parent {
                center.energy_to.set(birth.parent_dir.opposite(), true);
            }
            birthed = true;
        }

        if birthed {
            genomes.free(plan.handle);
            center.ty = LifeType::Pipe;
            if let Some(lifespan) = plan.parent_lifespan {
                center.steps_to_death = lifespan;
            }
        } else if let Some(next_gene) = plan.active_gene_update {
            genomes.get_mut(plan.handle).active_gene = next_gene;
        }

        grid.cells_mut()[plan.source].life = LifeCell::Alive(center);
    }

    let mut i = 0;
    while i < external_effects.len() {
        let target = external_effects[i].0;
        let mut kill = false;
        let mut damage = 0_u32;
        while i < external_effects.len() && external_effects[i].0 == target {
            kill |= external_effects[i].1;
            damage = damage.saturating_add(external_effects[i].2);
            i += 1;
        }

        if let LifeCell::Alive(mut life) = grid.cells()[target].life {
            if kill {
                life.steps_to_death = 0;
            } else {
                life.steps_to_death = life.steps_to_death.saturating_sub(
                    damage.min(u16::MAX as u32) as u16,
                );
            }
            grid.cells_mut()[target].life = LifeCell::Alive(life);
        }
    }
}

fn build_genome_plan(
    source: usize,
    life: AliveCell,
    handle: GenomeHandle,
    grid: &Grid<WorldCell>,
    genomes: &GenomePool,
    step: usize,
    config: &SimulationConfig,
) -> GenomePlan {
    let gene = genomes.get(handle).active_gene();
    let mut plan = GenomePlan::passive(source, handle, life.organism_id, life.energy);

    let mut next_active_gene = genomes.get(handle).active_gene;
    let mut local_energy = life.energy;
    let mut local_organics = LocalOrganics::from_grid(grid, source);

    if check_gene_condition(
        grid,
        source,
        local_energy,
        &local_organics,
        gene.main_action_condition,
        gene.main_action_param,
        step,
        life.heading,
        config.genetics.relative_directions,
        &config.life,
    ) {
        match collect_gene_action(
            gene.main_action,
            source,
            grid,
            &mut local_energy,
            &mut local_organics,
            &mut next_active_gene,
            &mut plan,
            life.heading,
            config.genetics.relative_directions,
            &config.life,
        ) {
            ActionFlow::Continue => {}
            ActionFlow::Wait => {
                plan.energy_after = local_energy;
                plan.active_gene_update = Some(next_active_gene);
                return plan;
            }
            ActionFlow::Die => {
                plan.energy_after = local_energy;
                plan.die = true;
                return plan;
            }
        }
    }

    let condition_1 = check_gene_condition(
        grid,
        source,
        local_energy,
        &local_organics,
        gene.additional_action_condition1,
        gene.additional_action_param1,
        step,
        life.heading,
        config.genetics.relative_directions,
        &config.life,
    );
    let condition_2 = check_gene_condition(
        grid,
        source,
        local_energy,
        &local_organics,
        gene.additional_action_condition2,
        gene.additional_action_param2,
        step,
        life.heading,
        config.genetics.relative_directions,
        &config.life,
    );
    let additional = match (condition_1, condition_2) {
        (true, true) => Some(gene.additional_action1),
        (true, false) => Some(gene.additional_action2),
        (false, true) => Some(gene.additional_action3),
        (false, false) => None,
    };
    if let Some(action) = additional {
        match collect_gene_action(
            action,
            source,
            grid,
            &mut local_energy,
            &mut local_organics,
            &mut next_active_gene,
            &mut plan,
            life.heading,
            config.genetics.relative_directions,
            &config.life,
        ) {
            ActionFlow::Continue => {}
            ActionFlow::Wait => {
                plan.energy_after = local_energy;
                plan.active_gene_update = Some(next_active_gene);
                return plan;
            }
            ActionFlow::Die => {
                plan.energy_after = local_energy;
                plan.die = true;
                return plan;
            }
        }
    }

    let condition_1 = check_gene_condition(
        grid,
        source,
        local_energy,
        &local_organics,
        gene.condition_1,
        gene.param_1,
        step,
        life.heading,
        config.genetics.relative_directions,
        &config.life,
    );
    let condition_2 = check_gene_condition(
        grid,
        source,
        local_energy,
        &local_organics,
        gene.condition_2,
        gene.param_2,
        step,
        life.heading,
        config.genetics.relative_directions,
        &config.life,
    );
    next_active_gene = match (condition_1, condition_2) {
        (true, true) => gene.alt_gene1,
        (true, false) => gene.alt_gene2,
        (false, true) => gene.alt_gene3,
        (false, false) => next_active_gene,
    };

    let growth_gene = genomes.get(handle).get_gene(next_active_gene);
    let total_energy = growth_gene.energy_capacity(&config.life.growth_energy);
    plan.active_gene_update = Some(next_active_gene);

    if local_energy <= total_energy {
        plan.energy_after = local_energy;
        return plan;
    }

    for local_dir in CellDir::ALL {
        let action = direction_action(&growth_gene, local_dir);
        let dir = resolve_genome_dir(
            local_dir,
            life.heading,
            config.genetics.relative_directions,
        );
        let target = neighbor_index(grid, source, dir);
        match action {
            GeneDirectionAction::MakeLeaf(lifespan) => collect_birth_or_collision(
                &mut plan,
                grid,
                target,
                dir,
                lifespan.0,
                BirthKind::Leaf,
                &config.life,
            ),
            GeneDirectionAction::MakeRoot(lifespan) => collect_birth_or_collision(
                &mut plan,
                grid,
                target,
                dir,
                lifespan.0,
                BirthKind::Root,
                &config.life,
            ),
            GeneDirectionAction::MakeReactor(lifespan) => collect_birth_or_collision(
                &mut plan,
                grid,
                target,
                dir,
                lifespan.0,
                BirthKind::Reactor,
                &config.life,
            ),
            GeneDirectionAction::MakeFilter(lifespan) => collect_birth_or_collision(
                &mut plan,
                grid,
                target,
                dir,
                lifespan.0,
                BirthKind::Filter,
                &config.life,
            ),
            GeneDirectionAction::MultiplySelf(lifespan, child_gene) => {
                let mut child_genome = *genomes.get(handle);
                child_genome.active_gene = child_gene;
                // A somatic copy is almost exact, but not mathematically perfect.
                // Only the daughter is edited; the parent's genome is untouched.
                child_genome.mutate_somatic(&config.genetics);
                collect_birth_or_collision(
                    &mut plan,
                    grid,
                    target,
                    dir,
                    lifespan.0,
                    BirthKind::Stem { genome: child_genome },
                    &config.life,
                );
            }
            GeneDirectionAction::CreateSeed(lifespan) => {
                let mut child_genome = *genomes.get(handle);
                // Seed mutation is the strong inherited mutation path.  The
                // resulting genome is stored dormant until the seed matures.
                child_genome.mutate_seed(&config.genetics);
                collect_birth_or_collision(
                    &mut plan,
                    grid,
                    target,
                    dir,
                    lifespan.0,
                    BirthKind::Seed { genome: child_genome },
                    &config.life,
                );
            }
            GeneDirectionAction::KillCell => {
                if let LifeCell::Alive(target_life) = grid.cells()[target].life {
                    local_energy += predation_energy_gain(target_life.energy, &config.life);
                    plan.kill_targets.push(target);
                }
            }
            GeneDirectionAction::Nothing => {}
        }
    }

    plan.energy_after = local_energy - total_energy;
    plan.parent_lifespan = Some(growth_gene.self_lifespan.0);
    plan
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionFlow {
    Continue,
    Wait,
    Die,
}

fn collect_gene_action(
    action: GeneAction,
    source: usize,
    grid: &Grid<WorldCell>,
    local_energy: &mut f32,
    local_organics: &mut LocalOrganics,
    next_active_gene: &mut GeneLocation,
    plan: &mut GenomePlan,
    heading: CellDir,
    relative_directions: bool,
    config: &LifeConfig,
) -> ActionFlow {
    use GeneAction::*;

    let center = source;
    match action {
        MoveOrganicUp => collect_organic_move(
            plan,
            grid,
            source,
            local_organics,
            OrganicPos::Center,
            OrganicPos::Direction(resolve_genome_dir(CellDir::Up, heading, relative_directions)),
        ),
        MoveOrganicDown => collect_organic_move(
            plan,
            grid,
            source,
            local_organics,
            OrganicPos::Center,
            OrganicPos::Direction(resolve_genome_dir(CellDir::Down, heading, relative_directions)),
        ),
        MoveOrganicLeft => collect_organic_move(
            plan,
            grid,
            source,
            local_organics,
            OrganicPos::Center,
            OrganicPos::Direction(resolve_genome_dir(CellDir::Left, heading, relative_directions)),
        ),
        MoveOrganicRight => collect_organic_move(
            plan,
            grid,
            source,
            local_organics,
            OrganicPos::Center,
            OrganicPos::Direction(resolve_genome_dir(CellDir::Right, heading, relative_directions)),
        ),
        MoveOrganicFromUp => collect_organic_move(
            plan,
            grid,
            source,
            local_organics,
            OrganicPos::Direction(resolve_genome_dir(CellDir::Up, heading, relative_directions)),
            OrganicPos::Center,
        ),
        MoveOrganicFromDown => collect_organic_move(
            plan,
            grid,
            source,
            local_organics,
            OrganicPos::Direction(resolve_genome_dir(CellDir::Down, heading, relative_directions)),
            OrganicPos::Center,
        ),
        MoveOrganicFromLeft => collect_organic_move(
            plan,
            grid,
            source,
            local_organics,
            OrganicPos::Direction(resolve_genome_dir(CellDir::Left, heading, relative_directions)),
            OrganicPos::Center,
        ),
        MoveOrganicFromRight => collect_organic_move(
            plan,
            grid,
            source,
            local_organics,
            OrganicPos::Direction(resolve_genome_dir(CellDir::Right, heading, relative_directions)),
            OrganicPos::Center,
        ),
        DoNothing => {}
        ChangeActiveGene(gene) => *next_active_gene = gene,
        KillUpLeft => {
            let (dx, dy) = resolve_relative_offset(-1, -1, heading, relative_directions);
            collect_kill(grid.offset_index(center, dx, dy), grid, local_energy, plan, config);
        }
        KillUpRight => {
            let (dx, dy) = resolve_relative_offset(1, -1, heading, relative_directions);
            collect_kill(grid.offset_index(center, dx, dy), grid, local_energy, plan, config);
        }
        KillDownLeft => {
            let (dx, dy) = resolve_relative_offset(-1, 1, heading, relative_directions);
            collect_kill(grid.offset_index(center, dx, dy), grid, local_energy, plan, config);
        }
        KillDownRight => {
            let (dx, dy) = resolve_relative_offset(1, 1, heading, relative_directions);
            collect_kill(grid.offset_index(center, dx, dy), grid, local_energy, plan, config);
        }
        WaitStep => return ActionFlow::Wait,
        Die => return ActionFlow::Die,
    }
    ActionFlow::Continue
}

fn collect_kill(
    target: usize,
    grid: &Grid<WorldCell>,
    energy: &mut f32,
    plan: &mut GenomePlan,
    config: &LifeConfig,
) {
    if let LifeCell::Alive(target_life) = grid.cells()[target].life {
        *energy += predation_energy_gain(target_life.energy, config);
        plan.kill_targets.push(target);
    }
}

#[inline]
fn predation_energy_gain(stored_energy: f32, config: &LifeConfig) -> f32 {
    let stored_energy = stored_energy.max(0.0);
    match config.predation.max_energy_gain {
        Some(cap) => stored_energy.min(cap),
        None => stored_energy,
    }
}

fn collect_birth_or_collision(
    plan: &mut GenomePlan,
    grid: &Grid<WorldCell>,
    target: usize,
    dir: CellDir,
    lifespan: u16,
    kind: BirthKind,
    config: &LifeConfig,
) {
    match grid.cells()[target].life {
        LifeCell::Alive(target_life) => {
            let damage = if target_life.organism_id == plan.organism_id {
                config.collision.self_damage
            } else {
                config.collision.foreign_damage
            };
            if damage != 0 {
                plan.collision_targets.push((target, damage));
            }
        }
        LifeCell::Dead => plan.births.push(BirthRequest {
            target,
            parent_dir: dir.opposite(),
            lifespan,
            kind,
            won: false,
        }),
    }
}

#[inline(always)]
fn resolve_genome_dir(local: CellDir, heading: CellDir, relative: bool) -> CellDir {
    if relative {
        heading.resolve_relative(local)
    } else {
        local
    }
}

#[inline(always)]
fn resolve_relative_offset(
    dx: i64,
    dy: i64,
    heading: CellDir,
    relative: bool,
) -> (i64, i64) {
    if !relative {
        return (dx, dy);
    }
    match heading {
        CellDir::Up => (dx, dy),
        CellDir::Right => (-dy, dx),
        CellDir::Down => (-dx, -dy),
        CellDir::Left => (dy, -dx),
    }
}

fn direction_action(gene: &crate::cells::life_cell::genome::Gene, dir: CellDir) -> GeneDirectionAction {
    match dir {
        CellDir::Up => gene.up,
        CellDir::Down => gene.down,
        CellDir::Left => gene.left,
        CellDir::Right => gene.right,
    }
}

fn check_gene_condition(
    grid: &Grid<WorldCell>,
    source: usize,
    life_energy: f32,
    local_organics: &LocalOrganics,
    condition: GeneCondition,
    param: u8,
    step: usize,
    heading: CellDir,
    relative_directions: bool,
    config: &LifeConfig,
) -> bool {
    use GeneCondition::*;

    let up_dir = resolve_genome_dir(CellDir::Up, heading, relative_directions);
    let down_dir = resolve_genome_dir(CellDir::Down, heading, relative_directions);
    let left_dir = resolve_genome_dir(CellDir::Left, heading, relative_directions);
    let right_dir = resolve_genome_dir(CellDir::Right, heading, relative_directions);
    let up = neighbor_index(grid, source, up_dir);
    let down = neighbor_index(grid, source, down_dir);
    let left = neighbor_index(grid, source, left_dir);
    let right = neighbor_index(grid, source, right_dir);
    let cells = grid.cells();

    match condition {
        LifeUp => cells[up].life.is_alive(),
        LifeDown => cells[down].life.is_alive(),
        LifeLeft => cells[left].life.is_alive(),
        LifeRight => cells[right].life.is_alive(),
        LethalOrganicUp => local_organics.get(OrganicPos::Direction(up_dir)) > config.lethal_organics,
        LethalOrganicDown => local_organics.get(OrganicPos::Direction(down_dir)) > config.lethal_organics,
        LethalOrganicLeft => local_organics.get(OrganicPos::Direction(left_dir)) > config.lethal_organics,
        LethalOrganicRight => local_organics.get(OrganicPos::Direction(right_dir)) > config.lethal_organics,
        LethalEnergyUp => cells[up].soil.energy > config.lethal_soil_energy,
        LethalEnergyDown => cells[down].soil.energy > config.lethal_soil_energy,
        LethalEnergyLeft => cells[left].soil.energy > config.lethal_soil_energy,
        LethalEnergyRight => cells[right].soil.energy > config.lethal_soil_energy,
        RandomMT => rand::thread_rng().gen::<u8>() > param,
        LifeEnergyMT => life_energy > param as f32,
        OrganicCenterMT => local_organics.center > param,
        OrganicUpMT => local_organics.get(OrganicPos::Direction(up_dir)) > param,
        OrganicDownMT => local_organics.get(OrganicPos::Direction(down_dir)) > param,
        OrganicLeftMT => local_organics.get(OrganicPos::Direction(left_dir)) > param,
        OrganicRightMT => local_organics.get(OrganicPos::Direction(right_dir)) > param,
        SoilEnergyCenterMT => cells[source].soil.energy > param as f32,
        SoilEnergyUpMT => cells[up].soil.energy > param as f32,
        SoilEnergyDownMT => cells[down].soil.energy > param as f32,
        SoilEnergyLeftMT => cells[left].soil.energy > param as f32,
        SoilEnergyRightMT => cells[right].soil.energy > param as f32,
        AirPollutionCenterMT => cells[source].air.pollution > param,
        AirPollutionUpMT => cells[up].air.pollution > param,
        AirPollutionDownMT => cells[down].air.pollution > param,
        AirPollutionLeftMT => cells[left].air.pollution > param,
        AirPollutionRightMT => cells[right].air.pollution > param,
        Always => true,
        Never => false,
        StepsDividesP => step % param.max(1) as usize == 0,
    }
}

fn neighbor_index(grid: &Grid<WorldCell>, index: usize, dir: CellDir) -> usize {
    let (dx, dy) = dir.offset();
    grid.offset_index(index, dx, dy)
}

fn collect_organic_move(
    plan: &mut GenomePlan,
    grid: &Grid<WorldCell>,
    source: usize,
    local: &mut LocalOrganics,
    from: OrganicPos,
    to: OrganicPos,
) {
    let amount = local.move_all(from, to);
    if amount == 0 {
        return;
    }
    plan.organic_moves.push(OrganicMove {
        from: organic_pos_index(grid, source, from),
        to: organic_pos_index(grid, source, to),
        amount,
    });
}

fn organic_pos_index(grid: &Grid<WorldCell>, source: usize, pos: OrganicPos) -> usize {
    match pos {
        OrganicPos::Center => source,
        OrganicPos::Direction(dir) => neighbor_index(grid, source, dir),
    }
}

fn move_organic(grid: &mut Grid<WorldCell>, movement: OrganicMove) {
    if movement.from == movement.to || movement.amount == 0 {
        return;
    }
    let source = grid.cells()[movement.from].soil.organics;
    let target = grid.cells()[movement.to].soil.organics;
    let amount = movement
        .amount
        .min(source)
        .min(u8::MAX - target);
    grid.cells_mut()[movement.from].soil.organics -= amount;
    grid.cells_mut()[movement.to].soil.organics += amount;
}

fn kill_index(grid: &mut Grid<WorldCell>, index: usize, genomes: &mut GenomePool, config: &LifeConfig) {
    let LifeCell::Alive(life) = grid.cells()[index].life else {
        return;
    };

    match life.ty {
        LifeType::Stem(handle) => genomes.free(handle),
        LifeType::Seed(seed) => genomes.free(seed.genome),
        _ => {}
    }

    {
        let cell = &mut grid.cells_mut()[index];
        cell.soil.organics = cell.soil.organics.saturating_add(life.organics(config));
        cell.soil.energy += life.energy.max(0.0) * config.death.energy_to_soil_fraction;
        cell.air.pollution = cell
            .air
            .pollution
            .saturating_add(
                (life.organics(config) / config.death.pollution_per_organic_divisor)
                    .max(config.death.minimum_pollution),
            );
        cell.life = LifeCell::Dead;
    }

    for dir in CellDir::ALL {
        let target = neighbor_index(grid, index, dir);
        if let LifeCell::Alive(mut neighbor) = grid.cells()[target].life {
            let toward_dead = dir.opposite();
            neighbor.energy_to.set(toward_dead, false);
            if neighbor.parent_dir == Some(toward_dead) {
                neighbor.parent_dir = None;
            }
            grid.cells_mut()[target].life = LifeCell::Alive(neighbor);
        }
    }
}

fn arbitration_key(step: u64, source: u64, target: u64, slot: u64) -> u64 {
    splitmix64(step ^ source.rotate_left(17) ^ target.rotate_left(31) ^ slot.rotate_left(47))
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cells::life_cell::{AliveCell, EnergyDirections};

    fn pipe(energy: f32, energy_to: EnergyDirections) -> WorldCell {
        let mut cell = WorldCell::default();
        cell.life = LifeCell::Alive(AliveCell::new(
            LifeType::Pipe,
            1,
            energy,
            energy_to,
            None,
            100,
        ));
        cell
    }


    #[test]
    fn relative_direction_frame_rotates_with_the_growth_heading() {
        let mut heading = CellDir::Up;
        let expected = [CellDir::Left, CellDir::Down, CellDir::Right, CellDir::Up];
        for next in expected {
            heading = resolve_genome_dir(CellDir::Left, heading, true);
            assert_eq!(heading, next);
        }

        // Absolute mode remains available for reproducibility/debugging.
        assert_eq!(
            resolve_genome_dir(CellDir::Left, CellDir::Down, false),
            CellDir::Left
        );
    }

    #[test]
    fn self_loop_on_local_left_turns_the_growth_front() {
        use crate::cells::life_cell::genome::{
            GeneAction, GeneCondition, GeneDirectionAction, LifeSpan,
        };

        let mut config = SimulationConfig::load();
        config.genetics.relative_directions = true;
        config.genetics.somatic_mutation.chance_per_million = 0;

        let mut rng = rand::thread_rng();
        let mut genome = Genome::random(&mut rng, &config.genetics);
        let active = genome.active_gene;
        let gene = &mut genome.genes[active.0 as usize];
        gene.up = GeneDirectionAction::Nothing;
        gene.down = GeneDirectionAction::Nothing;
        gene.left = GeneDirectionAction::MultiplySelf(LifeSpan(100), active);
        gene.right = GeneDirectionAction::Nothing;
        gene.main_action_condition = GeneCondition::Never;
        gene.additional_action_condition1 = GeneCondition::Never;
        gene.additional_action_condition2 = GeneCondition::Never;
        gene.condition_1 = GeneCondition::Never;
        gene.condition_2 = GeneCondition::Never;
        gene.main_action = GeneAction::DoNothing;

        let mut genomes = GenomePool::new();
        let handle = genomes.alloc(genome);
        let mut grid = Grid::<WorldCell>::new(5, 5);
        let source = 12; // (2,2)
        let mut life = AliveCell::new(
            LifeType::Stem(handle),
            1,
            100.0,
            EnergyDirections::default(),
            None,
            100,
        );
        life.heading = CellDir::Up;
        grid.cells_mut()[source].life = LifeCell::Alive(life);

        let first = build_genome_plan(source, life, handle, &grid, &genomes, 1, &config);
        let first_birth = first.births.first().expect("local-left self-loop should grow");
        assert_eq!(first_birth.target, grid.offset_index(source, -1, 0));
        assert_eq!(first_birth.parent_dir, CellDir::Right);

        let BirthKind::Stem { genome: child_genome } = &first_birth.kind else {
            panic!("expected a Stem child");
        };
        let child_handle = genomes.alloc(*child_genome);
        let child_source = first_birth.target;
        let child = AliveCell::new(
            LifeType::Stem(child_handle),
            1,
            100.0,
            EnergyDirections::default(),
            Some(first_birth.parent_dir),
            100,
        );
        assert_eq!(child.heading, CellDir::Left);
        grid.cells_mut()[source].life = LifeCell::Dead;
        grid.cells_mut()[child_source].life = LifeCell::Alive(child);

        let second = build_genome_plan(
            child_source,
            child,
            child_handle,
            &grid,
            &genomes,
            2,
            &config,
        );
        let second_birth = second.births.first().expect("loop should turn again");
        assert_eq!(second_birth.target, grid.offset_index(child_source, 0, 1));
    }

    #[test]
    fn newborn_heading_faces_away_from_its_parent() {
        let child = AliveCell::new(
            LifeType::Pipe,
            1,
            1.0,
            EnergyDirections::default(),
            Some(CellDir::Right),
            100,
        );
        // Parent is to the child's right, so the child grew leftwards.
        assert_eq!(child.heading, CellDir::Left);
    }

    #[test]
    fn multiply_self_copies_genome_without_mutating_it() {
        use crate::cells::life_cell::genome::{
            GeneAction, GeneCondition, GeneDirectionAction, GeneLocation, LifeSpan,
        };

        let mut config = SimulationConfig::load();
        config.genetics.somatic_mutation.chance_per_million = 0;
        let mut rng = rand::thread_rng();
        let mut genome = Genome::random(&mut rng, &config.genetics);
        let active = genome.active_gene;
        let next_gene = GeneLocation((active.0 + 1) % crate::cells::life_cell::genome::MAX_GENES);
        let gene = &mut genome.genes[active.0 as usize];
        gene.up = GeneDirectionAction::Nothing;
        gene.down = GeneDirectionAction::Nothing;
        gene.left = GeneDirectionAction::Nothing;
        gene.right = GeneDirectionAction::MultiplySelf(LifeSpan(100), next_gene);
        gene.main_action_condition = GeneCondition::Never;
        gene.additional_action_condition1 = GeneCondition::Never;
        gene.additional_action_condition2 = GeneCondition::Never;
        gene.condition_1 = GeneCondition::Never;
        gene.condition_2 = GeneCondition::Never;
        gene.main_action = GeneAction::DoNothing;

        let mut genomes = GenomePool::new();
        let handle = genomes.alloc(genome);
        let mut grid = Grid::<WorldCell>::new(3, 3);
        let source = 4;
        let life = AliveCell::new(
            LifeType::Stem(handle),
            1,
            100.0,
            EnergyDirections::default(),
            None,
            100,
        );
        grid.cells_mut()[source].life = LifeCell::Alive(life);

        let plan = build_genome_plan(source, life, handle, &grid, &genomes, 1, &config);
        let child = plan
            .births
            .iter()
            .find_map(|birth| match &birth.kind {
                BirthKind::Stem { genome } => Some(*genome),
                _ => None,
            })
            .expect("MultiplySelf should produce a Stem birth request");

        let mut expected = genome;
        expected.active_gene = next_gene;
        assert_eq!(child, expected);
    }

    #[test]
    fn change_gene_then_wait_preserves_the_state_transition_even_without_growth_energy() {
        use crate::cells::life_cell::genome::{
            GeneAction, GeneCondition, GeneDirectionAction, GeneLocation, LifeSpan,
        };

        let config = SimulationConfig::load();
        let mut rng = rand::thread_rng();
        let mut genome = Genome::random(&mut rng, &config.genetics);
        let active = genome.active_gene;
        let next_gene = GeneLocation((active.0 + 1) % crate::cells::life_cell::genome::MAX_GENES);
        let gene = &mut genome.genes[active.0 as usize];

        // Deliberately make the current growth program expensive. State-machine
        // actions must still execute before the selected growth gene is costed.
        gene.up = GeneDirectionAction::MakeLeaf(LifeSpan(100));
        gene.down = GeneDirectionAction::MakeLeaf(LifeSpan(100));
        gene.left = GeneDirectionAction::MakeLeaf(LifeSpan(100));
        gene.right = GeneDirectionAction::MakeLeaf(LifeSpan(100));
        gene.main_action_condition = GeneCondition::Always;
        gene.main_action = GeneAction::ChangeActiveGene(next_gene);
        gene.additional_action_condition1 = GeneCondition::Always;
        gene.additional_action_condition2 = GeneCondition::Never;
        gene.additional_action2 = GeneAction::WaitStep;

        let mut genomes = GenomePool::new();
        let handle = genomes.alloc(genome);
        let mut grid = Grid::<WorldCell>::new(3, 3);
        let source = 4;
        let life = AliveCell::new(
            LifeType::Stem(handle),
            1,
            0.2,
            EnergyDirections::default(),
            None,
            100,
        );
        grid.cells_mut()[source].life = LifeCell::Alive(life);

        let plan = build_genome_plan(source, life, handle, &grid, &genomes, 1, &config);
        assert_eq!(plan.active_gene_update, Some(next_gene));
        assert!(plan.births.is_empty());
    }


    #[test]
    fn create_seed_stays_attached_until_it_is_charged() {
        use crate::cells::life_cell::genome::{
            GeneAction, GeneCondition, GeneDirectionAction, LifeSpan,
        };

        let mut config = SimulationConfig::load();
        // Make this test deterministic; mutation semantics are tested separately.
        config.genetics.initial_mutation_rate.min = config.genetics.mutation_rate_min;
        config.genetics.initial_mutation_rate.max = config.genetics.mutation_rate_min;

        let mut rng = rand::thread_rng();
        let mut genome = Genome::random(&mut rng, &config.genetics);
        genome.mutation_rate.0 = config.genetics.mutation_rate_min;
        let active = genome.active_gene;
        let gene = &mut genome.genes[active.0 as usize];
        gene.up = GeneDirectionAction::Nothing;
        gene.down = GeneDirectionAction::Nothing;
        gene.left = GeneDirectionAction::Nothing;
        gene.right = GeneDirectionAction::CreateSeed(LifeSpan(100));
        gene.main_action_condition = GeneCondition::Never;
        gene.additional_action_condition1 = GeneCondition::Never;
        gene.additional_action_condition2 = GeneCondition::Never;
        gene.condition_1 = GeneCondition::Never;
        gene.condition_2 = GeneCondition::Never;
        gene.main_action = GeneAction::DoNothing;

        let mut genomes = GenomePool::new();
        let handle = genomes.alloc(genome);
        let mut grid = Grid::<WorldCell>::new(3, 3);
        let source = 4;
        let target = 5;
        grid.cells_mut()[source].life = LifeCell::Alive(AliveCell::new(
            LifeType::Stem(handle),
            7,
            100.0,
            EnergyDirections::default(),
            None,
            100,
        ));

        let mut state = State::default();
        state.next_organism_id = 100;
        process_genomes(&mut state, &mut grid, &mut genomes, &config, &[source]);

        let LifeCell::Alive(parent) = grid.cells()[source].life else {
            panic!("seed parent should remain as a Pipe");
        };
        assert_eq!(parent.ty, LifeType::Pipe);
        assert!(parent.energy_to.right);

        let LifeCell::Alive(seed) = grid.cells()[target].life else {
            panic!("CreateSeed should create a seed");
        };
        assert!(matches!(seed.ty, LifeType::Seed(_)));
        assert_eq!(seed.organism_id, 7);
        assert_eq!(seed.parent_dir, Some(CellDir::Left));
        assert_eq!(seed.steps_to_death, config.life.reproduction.seed_lifespan);
        assert!((seed.energy - config.life.reproduction.seed_initial_energy).abs() < f32::EPSILON);
        // No independent organism exists before maturation.
        assert_eq!(state.next_organism_id, 100);
    }

    #[test]
    fn charged_seed_detaches_and_becomes_a_new_organism() {
        let config = SimulationConfig::load();
        let mut rng = rand::thread_rng();
        let genome = Genome::random(&mut rng, &config.genetics);
        let mut genomes = GenomePool::new();
        let handle = genomes.alloc(genome);

        let mut grid = Grid::<WorldCell>::new(3, 1);
        let parent = 1;
        let seed_index = 2;
        let mut parent_dirs = EnergyDirections::default();
        parent_dirs.right = true;
        grid.cells_mut()[parent].life = LifeCell::Alive(AliveCell::new(
            LifeType::Pipe,
            7,
            10.0,
            parent_dirs,
            None,
            100,
        ));
        grid.cells_mut()[seed_index].life = LifeCell::Alive(AliveCell::new(
            LifeType::Seed(SeedState {
                genome: handle,
                stem_lifespan: 123,
            }),
            7,
            config.life.reproduction.seed_maturation_energy,
            EnergyDirections::default(),
            Some(CellDir::Left),
            config.life.reproduction.seed_lifespan,
        ));

        let mut state = State::default();
        state.next_organism_id = 100;
        mature_seeds(&mut state, &mut grid, &[seed_index], &config.life);

        let LifeCell::Alive(parent) = grid.cells()[parent].life else {
            panic!("parent unexpectedly died");
        };
        assert!(!parent.energy_to.right);

        let LifeCell::Alive(child) = grid.cells()[seed_index].life else {
            panic!("mature seed unexpectedly died");
        };
        assert_eq!(child.ty, LifeType::Stem(handle));
        assert_eq!(child.organism_id, 100);
        assert_eq!(child.parent_dir, None);
        assert_eq!(child.steps_to_death, 123);
        assert_eq!(state.next_organism_id, 101);
    }

    #[test]
    fn seed_charge_is_capped_without_destroying_parent_energy() {
        let config = SimulationConfig::load();
        let mut rng = rand::thread_rng();
        let genome = Genome::random(&mut rng, &config.genetics);
        let mut genomes = GenomePool::new();
        let handle = genomes.alloc(genome);

        let mut grid = Grid::<WorldCell>::new(2, 1);
        let mut right = EnergyDirections::default();
        right.right = true;
        grid.cells_mut()[0].life = LifeCell::Alive(AliveCell::new(
            LifeType::Pipe,
            1,
            10.0,
            right,
            None,
            100,
        ));
        grid.cells_mut()[1].life = LifeCell::Alive(AliveCell::new(
            LifeType::Seed(SeedState {
                genome: handle,
                stem_lifespan: 100,
            }),
            1,
            config.life.reproduction.seed_maturation_energy - 0.01,
            EnergyDirections::default(),
            Some(CellDir::Left),
            config.life.reproduction.seed_lifespan,
        ));

        transfer_energy_one_hop(&mut grid, &[0], &config.life);

        let LifeCell::Alive(parent) = grid.cells()[0].life else {
            panic!("parent unexpectedly died");
        };
        let LifeCell::Alive(seed) = grid.cells()[1].life else {
            panic!("seed unexpectedly died");
        };
        let charged = seed.incoming_energy;
        assert!(charged > 0.0);
        assert!(charged <= config.life.reproduction.seed_max_charge_per_tick + f32::EPSILON);
        assert!((parent.energy - (10.0 - charged)).abs() < 1e-5);
        let after_next_maintenance = seed.energy + charged - config.life.consumption.seed;
        assert!(after_next_maintenance >= config.life.reproduction.seed_maturation_energy);
    }

    #[test]
    fn energy_moves_at_most_one_cell_per_transfer_phase() {
        let mut grid = Grid::<WorldCell>::new(5, 1);
        let right = EnergyDirections {
            right: true,
            ..EnergyDirections::default()
        };

        grid.uset(0, 0, pipe(2.0, right));
        grid.uset(1, 0, pipe(0.0, right));
        grid.uset(2, 0, pipe(0.0, EnergyDirections::default()));

        let config = SimulationConfig::load();
        transfer_energy_one_hop(&mut grid, &[0, 1], &config.life);

        let LifeCell::Alive(b) = grid.uget(1, 0).life else {
            panic!("middle cell unexpectedly died");
        };
        let LifeCell::Alive(c) = grid.uget(2, 0).life else {
            panic!("last cell unexpectedly died");
        };
        assert!(b.incoming_energy > 0.0);
        assert_eq!(c.incoming_energy, 0.0);

        if let LifeCell::Alive(mut b) = grid.uget(1, 0).life {
            b.energy += b.incoming_energy;
            b.incoming_energy = 0.0;
            grid.uget_mut(1, 0).life = LifeCell::Alive(b);
        }
        transfer_energy_one_hop(&mut grid, &[0, 1], &config.life);

        let LifeCell::Alive(c) = grid.uget(2, 0).life else {
            panic!("last cell unexpectedly died");
        };
        assert!(c.incoming_energy > 0.0);
    }
}
