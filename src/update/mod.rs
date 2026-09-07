use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
    cells::{life_cell::genome::GenomePool, WorldCell},
    config::SimulationConfig,
    grid::Grid,
    types::State,
};

mod environment;
mod life;

use environment::{compute_environment, refresh_wind_targets};
use life::update_life_step;

pub struct SimulationBuffers {
    soil_energy: Vec<f32>,
    pollution: Vec<f32>,
    wind_x: Vec<f32>,
    wind_y: Vec<f32>,
    target_wind_x: Vec<f32>,
    target_wind_y: Vec<f32>,
    wind_epoch: usize,
}

impl SimulationBuffers {
    pub fn new(grid: &Grid<WorldCell>) -> Self {
        let len = grid.len();
        Self {
            soil_energy: vec![0.0; len],
            pollution: vec![0.0; len],
            wind_x: vec![0.0; len],
            wind_y: vec![0.0; len],
            target_wind_x: vec![0.0; len],
            target_wind_y: vec![0.0; len],
            wind_epoch: usize::MAX,
        }
    }

    fn ensure_size(&mut self, grid: &Grid<WorldCell>) {
        let len = grid.len();
        if self.soil_energy.len() != len {
            self.soil_energy.resize(len, 0.0);
            self.pollution.resize(len, 0.0);
            self.wind_x.resize(len, 0.0);
            self.wind_y.resize(len, 0.0);
            self.target_wind_x.resize(len, 0.0);
            self.target_wind_y.resize(len, 0.0);
            self.wind_epoch = usize::MAX;
        }
    }
}

pub fn update_simulation_step(
    state: &mut State,
    grid: &mut Grid<WorldCell>,
    genomes: &mut GenomePool,
    buffers: &mut SimulationBuffers,
    config: &SimulationConfig,
    debug_phase: &AtomicU8,
) {
    buffers.ensure_size(grid);

    let gust_period = config.environment.wind.gust_period_ticks.max(1);
    let wind_epoch = state.simulation_step / gust_period;
    if buffers.wind_epoch != wind_epoch {
        debug_phase.store(1, Ordering::Relaxed);
        refresh_wind_targets(
            grid,
            &mut buffers.target_wind_x,
            &mut buffers.target_wind_y,
            wind_epoch as u64,
            &config.environment.wind,
        );

        // Start directly in the first coherent field; later weather epochs are
        // approached gradually by `response_per_tick` inside compute_environment.
        if buffers.wind_epoch == usize::MAX {
            buffers.wind_x.copy_from_slice(&buffers.target_wind_x);
            buffers.wind_y.copy_from_slice(&buffers.target_wind_y);
        }
        buffers.wind_epoch = wind_epoch;
    }

    debug_phase.store(2, Ordering::Relaxed);
    compute_environment(
        grid,
        &mut buffers.soil_energy,
        &mut buffers.pollution,
        &mut buffers.wind_x,
        &mut buffers.wind_y,
        &buffers.target_wind_x,
        &buffers.target_wind_y,
        &config.environment,
    );

    debug_phase.store(3, Ordering::Relaxed);
    update_life_step(
        state,
        grid,
        genomes,
        config,
        &buffers.soil_energy,
        &buffers.pollution,
    );
}

#[cfg(test)]
mod ecology_tests {
    use super::*;
    use crate::cells::life_cell::{
        genome::Genome, AliveCell, EnergyDirections, LifeCell, LifeType,
    };
    use rand::{rngs::StdRng, Rng, SeedableRng};

    #[test]
    fn connected_root_leaf_body_grows_after_bootstrap_runs_out() {
        use crate::cells::life_cell::genome::{GeneCondition, GeneDirectionAction, LifeSpan};
        fn run(with_roots: bool) -> usize {
            let mut config = SimulationConfig::load();
            config.genetics.somatic_mutation.chance_per_million = 0;
            let mut rng = StdRng::seed_from_u64(7);
            let mut genome = Genome::random(&mut rng, &config.genetics);
            let active = genome.active_gene;
            let gene = &mut genome.genes[active.0 as usize];
            gene.up = GeneDirectionAction::MultiplySelf(LifeSpan(1000), active);
            gene.down = GeneDirectionAction::Nothing;
            gene.left = if with_roots {
                GeneDirectionAction::MakeRoot(LifeSpan(1000))
            } else {
                GeneDirectionAction::Nothing
            };
            gene.right = GeneDirectionAction::MakeLeaf(LifeSpan(1000));
            gene.self_lifespan = LifeSpan(1000);
            gene.main_action_condition = GeneCondition::Never;
            gene.additional_action_condition1 = GeneCondition::Never;
            gene.additional_action_condition2 = GeneCondition::Never;
            gene.condition_1 = GeneCondition::Never;
            gene.condition_2 = GeneCondition::Never;
            let mut genomes = GenomePool::new();
            let handle = genomes.alloc(genome);
            let mut grid = Grid::<WorldCell>::new(32, 512);
            grid.uget_mut(16, 400).life = LifeCell::Alive(AliveCell::new(
                LifeType::Stem(handle),
                1,
                config.world.initial_stem_energy,
                EnergyDirections::default(),
                None,
                1000,
            ));
            let mut state = State::default();
            let mut buffers = SimulationBuffers::new(&grid);
            let phase = AtomicU8::new(0);
            for tick in 0..600 {
                state.simulation_step = tick;
                update_simulation_step(
                    &mut state,
                    &mut grid,
                    &mut genomes,
                    &mut buffers,
                    &config,
                    &phase,
                );
            }
            grid.cells().iter().filter(|c| c.life.is_alive()).count()
        }
        let supported = run(true);
        let rootless = run(false);
        eprintln!("supported={supported}, rootless={rootless}");
        assert!(
            supported >= 25,
            "functional body must grow beyond its initial energy budget"
        );
        assert_eq!(
            rootless, 0,
            "leaves cannot live indefinitely without water-supplying roots"
        );
    }

    // Run explicitly when tuning ecology; includes weather, metabolism and reproduction.
    #[test]
    #[ignore]
    fn population_survival_probe() {
        let config = SimulationConfig::load();
        let seed = std::env::var("SPECTACULIFE_PROBE_SEED")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(42);
        let mut rng = StdRng::seed_from_u64(seed);
        let mut grid = Grid::<WorldCell>::new(768, 256);
        let mut genomes = GenomePool::new();
        let mut state = State::default();
        for y in 0..grid.height {
            for x in 0..grid.width {
                let cell = grid.uget_mut(x, y);
                cell.soil.organics = config.world.initial_organics.sample(&mut rng);
                cell.soil.energy = config.world.initial_soil_energy.sample(&mut rng);
                cell.air.pollution = config.world.initial_pollution.sample(&mut rng);
                if x % config.world.organism_spacing == 0 && y % config.world.organism_spacing == 0
                {
                    let handle = genomes.alloc(Genome::random(&mut rng, &config.genetics));
                    let mut founder = AliveCell::new(
                        LifeType::Stem(handle),
                        state.allocate_organism_id(),
                        config.world.initial_stem_energy,
                        EnergyDirections::default(),
                        None,
                        config.world.initial_stem_lifespan,
                    );
                    founder.heading = rng.gen();
                    cell.life = LifeCell::Alive(founder);
                }
            }
        }
        let founders = state.next_organism_id;
        let mut buffers = SimulationBuffers::new(&grid);
        let phase = AtomicU8::new(0);
        let mut offspring_at_2500 = founders;
        for tick in 0..3000 {
            if tick == 2500 {
                offspring_at_2500 = state.next_organism_id;
            }
            state.simulation_step = tick;
            update_simulation_step(
                &mut state,
                &mut grid,
                &mut genomes,
                &mut buffers,
                &config,
                &phase,
            );
            if tick % 250 == 249 {
                let mut counts = [0; 7];
                let mut bodies = std::collections::BTreeMap::<u64, usize>::new();
                for cell in grid.cells() {
                    if let LifeCell::Alive(life) = cell.life {
                        *bodies.entry(life.organism_id).or_default() += 1;
                        counts[match life.ty {
                            LifeType::Pipe => 0,
                            LifeType::Stem(_) => 1,
                            LifeType::Seed(_) => 2,
                            LifeType::Leaf => 3,
                            LifeType::Root => 4,
                            LifeType::Reactor => 5,
                            LifeType::Filter => 6,
                        }] += 1;
                        assert!(life.energy.is_finite() && life.water >= -0.0001);
                    }
                }
                eprintln!(
                    "tick={} cells={counts:?} offspring={} largest={}",
                    tick + 1,
                    state.next_organism_id - founders,
                    bodies.values().max().unwrap_or(&0)
                );
            }
        }
        assert!(
            grid.cells()
                .iter()
                .any(|c| matches!(c.life, LifeCell::Alive(life)
            if life.is_fertile() || life.is_seed())),
            "no surviving growth fronts or embryos"
        );
        assert!(
            state.next_organism_id > offspring_at_2500,
            "reproduction stopped during the final 500 ticks"
        );
    }
}
