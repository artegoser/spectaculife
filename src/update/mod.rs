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
