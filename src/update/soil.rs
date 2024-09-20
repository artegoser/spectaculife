use crate::{cells::WorldCell, grid::Area};

pub fn update_soil(area: &mut Area<WorldCell>) {
    let mut total: f32 = 0.;

    total += area.up_left.soil.energy;
    total += area.up.soil.energy;
    total += area.up_right.soil.energy;
    total += area.left.soil.energy;
    total += area.center.soil.energy;
    total += area.right.soil.energy;
    total += area.down_left.soil.energy;
    total += area.down.soil.energy;
    total += area.down_right.soil.energy;

    let foreach = total / 9.0025;

    area.up_left.soil.energy = foreach;
    area.up.soil.energy = foreach;
    area.up_right.soil.energy = foreach;
    area.left.soil.energy = foreach;
    area.center.soil.energy = foreach;
    area.right.soil.energy = foreach;
    area.down_left.soil.energy = foreach;
    area.down.soil.energy = foreach;
    area.down_right.soil.energy = foreach;
}
