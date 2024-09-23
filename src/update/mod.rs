use crate::{cells::WorldCell, grid::Area, types::State};

mod air;
mod life;
mod soil;

use air::*;
use life::*;
use soil::*;

pub fn update_world(state: &mut State, area: &mut Area<WorldCell>) {
    update_soil(area);
    update_air(area);
    update_life(state, area);
}
