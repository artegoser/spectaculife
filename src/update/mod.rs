use crate::{
    cells::WorldCell,
    config::SimulationConfig,
    cells::life_cell::genome::GenomePool,
    grid::{Area, Grid},
    types::State,
};

mod air;
mod life;
mod soil;

use air::update_air;
use life::update_life;
use soil::update_soil;

pub struct EnvironmentBuffers {
    soil_energy: Vec<f32>,
    pollution: Vec<f32>,
}

impl EnvironmentBuffers {
    pub fn new(grid: &Grid<WorldCell>) -> Self {
        let len = grid.width as usize * grid.height as usize;
        Self {
            soil_energy: vec![0.0; len],
            pollution: vec![0.0; len],
        }
    }

    fn ensure_size(&mut self, grid: &Grid<WorldCell>) {
        let len = grid.width as usize * grid.height as usize;
        if self.soil_energy.len() != len {
            self.soil_energy.resize(len, 0.0);
            self.pollution.resize(len, 0.0);
        }
    }
}

/// Diffuse the environment exactly once per simulation step using a read/write
/// buffer. This removes order dependence and directional bias from the old
/// in-place 3x3 averaging.
pub fn update_environment(
    grid: &mut Grid<WorldCell>,
    buffers: &mut EnvironmentBuffers,
    config: &SimulationConfig,
) {
    buffers.ensure_size(grid);
    update_soil(
        grid,
        &mut buffers.soil_energy,
        config.environment.soil_diffusion,
    );
    update_air(
        grid,
        &mut buffers.pollution,
        config.environment.air_diffusion,
    );
}

pub fn update_world(
    state: &mut State,
    area: &mut Area<WorldCell>,
    genomes: &mut GenomePool,
    config: &SimulationConfig,
) {
    update_life(state, area, genomes, config);
}
