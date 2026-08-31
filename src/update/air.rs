use crate::{cells::WorldCell, grid::Grid};

const AIR_DIFFUSION: f32 = 0.22;

pub fn update_air(grid: &mut Grid<WorldCell>, next: &mut [f32]) {
    let width = grid.width;
    let height = grid.height;

    for y in 0..height {
        for x in 0..width {
            let x = x as i64;
            let y = y as i64;
            let center = grid.get(x, y).air.pollution as f32;
            let neighbor_avg = (
                grid.get(x - 1, y - 1).air.pollution as f32
                    + grid.get(x, y - 1).air.pollution as f32
                    + grid.get(x + 1, y - 1).air.pollution as f32
                    + grid.get(x - 1, y).air.pollution as f32
                    + grid.get(x + 1, y).air.pollution as f32
                    + grid.get(x - 1, y + 1).air.pollution as f32
                    + grid.get(x, y + 1).air.pollution as f32
                    + grid.get(x + 1, y + 1).air.pollution as f32
            ) / 8.0;

            let idx = y as usize * width as usize + x as usize;
            next[idx] = center + (neighbor_avg - center) * AIR_DIFFUSION;
        }
    }

    for y in 0..height {
        for x in 0..width {
            let idx = y as usize * width as usize + x as usize;
            grid.uget_mut(x, y).air.pollution = next[idx].round().clamp(0.0, 255.0) as u8;
        }
    }
}
