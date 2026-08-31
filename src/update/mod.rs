use crate::{
    cells::{life_cell::genome::GenomePool, WorldCell},
    config::SimulationConfig,
    grid::Grid,
    types::State,
};

mod environment;
mod life;

use environment::compute_environment;
use life::update_life_step;

pub struct SimulationBuffers {
    soil_energy: Vec<f32>,
    pollution: Vec<f32>,
}

impl SimulationBuffers {
    pub fn new(grid: &Grid<WorldCell>) -> Self {
        let len = grid.len();
        Self {
            soil_energy: vec![0.0; len],
            pollution: vec![0.0; len],
        }
    }

    fn ensure_size(&mut self, grid: &Grid<WorldCell>) {
        let len = grid.len();
        if self.soil_energy.len() != len {
            self.soil_energy.resize(len, 0.0);
            self.pollution.resize(len, 0.0);
        }
    }
}

pub fn update_simulation_step(
    state: &mut State,
    grid: &mut Grid<WorldCell>,
    genomes: &mut GenomePool,
    buffers: &mut SimulationBuffers,
    config: &SimulationConfig,
) {
    buffers.ensure_size(grid);

    compute_environment(
        grid,
        &mut buffers.soil_energy,
        &mut buffers.pollution,
        config.environment.soil_diffusion,
        config.environment.air_diffusion,
    );

    update_life_step(
        state,
        grid,
        genomes,
        config,
        &buffers.soil_energy,
        &buffers.pollution,
    );
}
