use crate::{cells::WorldCell, grid::Grid};

pub fn update_soil(grid: &mut Grid<WorldCell>, next: &mut [f32], diffusion: f32) {
    let width = grid.width;
    let height = grid.height;

    for y in 0..height {
        for x in 0..width {
            let x = x as i64;
            let y = y as i64;
            let center = grid.get(x, y).soil.energy;
            let neighbor_avg = (
                grid.get(x - 1, y - 1).soil.energy
                    + grid.get(x, y - 1).soil.energy
                    + grid.get(x + 1, y - 1).soil.energy
                    + grid.get(x - 1, y).soil.energy
                    + grid.get(x + 1, y).soil.energy
                    + grid.get(x - 1, y + 1).soil.energy
                    + grid.get(x, y + 1).soil.energy
                    + grid.get(x + 1, y + 1).soil.energy
            ) / 8.0;

            let idx = y as usize * width as usize + x as usize;
            next[idx] = center + (neighbor_avg - center) * diffusion;
        }
    }

    for y in 0..height {
        for x in 0..width {
            let idx = y as usize * width as usize + x as usize;
            grid.uget_mut(x, y).soil.energy = next[idx].max(0.0);
        }
    }
}
