use rayon::prelude::*;

use crate::{cells::WorldCell, grid::Grid};

/// Diffuse soil energy and pollution from the same immutable world state.
///
/// Both fields are calculated in one row-parallel pass from the same stable
/// state. The caller commits these buffers together with life maintenance,
/// avoiding a second world-sized write pass. The double buffer also guarantees
/// that neither field propagates farther than one Moore-neighborhood cell per tick.
pub fn compute_environment(
    grid: &Grid<WorldCell>,
    next_soil_energy: &mut [f32],
    next_pollution: &mut [f32],
    soil_diffusion: f32,
    air_diffusion: f32,
) {
    let width = grid.width as usize;
    let height = grid.height as usize;
    debug_assert_eq!(next_soil_energy.len(), grid.len());
    debug_assert_eq!(next_pollution.len(), grid.len());

    let cells = grid.cells();

    next_soil_energy
        .par_chunks_mut(width)
        .zip(next_pollution.par_chunks_mut(width))
        .enumerate()
        .for_each(|(y, (soil_row, pollution_row))| {
            let yu = if y == 0 { height - 1 } else { y - 1 };
            let yd = if y + 1 == height { 0 } else { y + 1 };
            let row = y * width;
            let row_up = yu * width;
            let row_down = yd * width;

            for x in 0..width {
                let xl = if x == 0 { width - 1 } else { x - 1 };
                let xr = if x + 1 == width { 0 } else { x + 1 };
                let center_idx = row + x;

                let neighbor_indices = [
                    row_up + xl,
                    row_up + x,
                    row_up + xr,
                    row + xl,
                    row + xr,
                    row_down + xl,
                    row_down + x,
                    row_down + xr,
                ];

                let mut soil_total = 0.0_f32;
                let mut pollution_total = 0.0_f32;
                for neighbor in neighbor_indices {
                    let cell = &cells[neighbor];
                    soil_total += cell.soil.energy;
                    pollution_total += cell.air.pollution as f32;
                }

                let center = &cells[center_idx];
                let soil_neighbor_avg = soil_total * 0.125;
                soil_row[x] = (center.soil.energy
                    + (soil_neighbor_avg - center.soil.energy) * soil_diffusion)
                    .max(0.0);

                let pollution_center = center.air.pollution as f32;
                let pollution_neighbor_avg = pollution_total * 0.125;
                pollution_row[x] = pollution_center
                    + (pollution_neighbor_avg - pollution_center) * air_diffusion;
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_does_not_skip_cells_in_one_tick() {
        let mut grid = Grid::<WorldCell>::new(7, 7);
        grid.uget_mut(3, 3).soil.energy = 100.0;
        grid.uget_mut(3, 3).air.pollution = 100;
        let mut next_soil = vec![0.0; grid.len()];
        let mut next_pollution = vec![0.0; grid.len()];

        compute_environment(
            &grid,
            &mut next_soil,
            &mut next_pollution,
            1.0,
            1.0,
        );

        let neighbor = 3 * 7 + 4;
        let two_away = 3 * 7 + 5;
        assert!(next_soil[neighbor] > 0.0);
        assert_eq!(next_soil[two_away], 0.0);
        assert!(next_pollution[neighbor] > 0.0);
        assert_eq!(next_pollution[two_away], 0.0);
    }
}
