use rand::{thread_rng, Rng};

use crate::{
    cells::{
        life_cell::{
            genome::{
                GeneCondition::{self, *},
                Genome,
            },
            LifeCell::*,
            LifeType::{self, *},
        },
        soil_cell::{MAX_ENERGY_LIFE, MAX_ORGANIC_LIFE},
        WorldCell,
    },
    grid::Area,
    types::State,
};

pub fn update_life(state: &mut State, area: &mut Area<WorldCell>) {
    if let Alive(mut life) = area.center.life {
        // Process genome
        match life {
            Stem { genome, energy } => {
                process_genome(state, area, &mut life, genome);
            }
            _ => {}
        };

        area.center.life = Alive(life);
    }
}

fn process_genome(
    state: &State,
    area: &mut Area<WorldCell>,
    life: &mut LifeType,
    mut genome: Genome,
) {
}

fn check_gene_condition(
    state: &State,
    area: &Area<WorldCell>,
    energy: f32,
    condition: GeneCondition,
    param: u8,
) -> bool {
    match condition {
        LifeUp => area.up.life.is_alive(),
        LifeDown => area.down.life.is_alive(),
        LifeLeft => area.left.life.is_alive(),
        LifeRight => area.right.life.is_alive(),

        LethalOrganicUp => area.up.soil.organics > MAX_ORGANIC_LIFE,
        LethalOrganicDown => area.down.soil.organics > MAX_ORGANIC_LIFE,
        LethalOrganicLeft => area.left.soil.organics > MAX_ORGANIC_LIFE,
        LethalOrganicRight => area.right.soil.organics > MAX_ORGANIC_LIFE,

        LethalEnergyUp => area.up.soil.energy > MAX_ENERGY_LIFE,
        LethalEnergyDown => area.down.soil.energy > MAX_ENERGY_LIFE,
        LethalEnergyLeft => area.left.soil.energy > MAX_ENERGY_LIFE,
        LethalEnergyRight => area.right.soil.energy > MAX_ENERGY_LIFE,

        RandomMT => thread_rng().gen::<u8>() > param,
        LifeEnergyMT => energy > param as f32,

        OrganicCenterMT => area.center.soil.organics > param,
        OrganicUpMT => area.up.soil.organics > param,
        OrganicDownMT => area.down.soil.organics > param,
        OrganicLeftMT => area.left.soil.organics > param,
        OrganicRightMT => area.right.soil.organics > param,

        SoilEnergyCenterMT => area.center.soil.energy > param as f32,
        SoilEnergyUpMT => area.up.soil.energy > param as f32,
        SoilEnergyDownMT => area.down.soil.energy > param as f32,
        SoilEnergyLeftMT => area.left.soil.energy > param as f32,
        SoilEnergyRightMT => area.right.soil.energy > param as f32,

        AirPollutionCenterMT => area.center.air.pollution > param,
        AirPollutionUpMT => area.up.air.pollution > param,
        AirPollutionDownMT => area.down.air.pollution > param,
        AirPollutionLeftMT => area.left.air.pollution > param,
        AirPollutionRightMT => area.right.air.pollution > param,

        Always => true,
        Never => false,
    }
}
