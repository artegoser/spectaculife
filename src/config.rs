use bevy::prelude::Resource;
use rand::Rng;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};

pub const DEFAULT_CONFIG_PATH: &str = "assets/simulation.ron";

#[derive(Debug, Clone, Resource, Deserialize)]
pub struct SimulationConfig {
    pub world: WorldConfig,
    pub environment: EnvironmentConfig,
    pub genetics: GeneticsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldConfig {
    pub width: u32,
    pub height: u32,
    pub organism_spacing: u32,

    pub initial_organics: U8Range,
    pub initial_soil_energy: F32Range,
    pub initial_pollution: U8Range,

    pub initial_stem_energy: f32,
    pub initial_stem_lifespan: u16,
    pub collision_damage: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnvironmentConfig {
    pub soil_diffusion: f32,
    pub air_diffusion: f32,
    pub soil_energy_render_max: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneticsConfig {
    pub lifespan: U16Range,
    pub initial_mutation_rate: U8Range,
    pub mutation_rate_min: u8,
    pub mutation_rate_max: u8,

    pub second_mutation_edit_chance_percent: u8,
    pub mutation_rate_evolution_chance_percent: u8,
    pub mutation_rate_increase_weight: u32,
    pub mutation_rate_decrease_weight: u32,

    pub mutation_gene_targets: Vec<Weighted<MutationGeneTarget>>,
    pub direction_actions: Vec<Weighted<DirectionActionKind>>,
    pub conditions: Vec<Weighted<ConditionKind>>,
    pub gene_actions: Vec<Weighted<GeneActionKind>>,
    pub mutation_edits: Vec<Weighted<MutationEditKind>>,

    pub condition_params: ConditionParamConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConditionParamConfig {
    pub life_energy_max: u8,
    pub organics_max: u8,
    pub soil_energy_max: u8,
    pub pollution_max: u8,
    pub steps_divides_max: u8,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct U8Range {
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct U16Range {
    pub min: u16,
    pub max: u16,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct F32Range {
    pub min: f32,
    pub max: f32,
}

impl U8Range {
    pub fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        rng.gen_range(self.min..=self.max)
    }
}

impl U16Range {
    pub fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u16 {
        rng.gen_range(self.min..=self.max)
    }
}

impl F32Range {
    pub fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f32 {
        rng.gen_range(self.min..=self.max)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Weighted<T> {
    pub kind: T,
    pub weight: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum DirectionActionKind {
    MultiplySelf,
    MakeLeaf,
    MakeRoot,
    MakeReactor,
    MakeFilter,
    CreateSeed,
    Nothing,
    KillCell,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum ConditionKind {
    LifeUp,
    LifeDown,
    LifeLeft,
    LifeRight,

    LethalOrganicUp,
    LethalOrganicDown,
    LethalOrganicLeft,
    LethalOrganicRight,

    LethalEnergyUp,
    LethalEnergyDown,
    LethalEnergyLeft,
    LethalEnergyRight,

    RandomMT,
    LifeEnergyMT,

    OrganicCenterMT,
    OrganicUpMT,
    OrganicDownMT,
    OrganicLeftMT,
    OrganicRightMT,

    SoilEnergyCenterMT,
    SoilEnergyUpMT,
    SoilEnergyDownMT,
    SoilEnergyLeftMT,
    SoilEnergyRightMT,

    AirPollutionCenterMT,
    AirPollutionUpMT,
    AirPollutionDownMT,
    AirPollutionLeftMT,
    AirPollutionRightMT,

    Always,
    Never,
    StepsDividesP,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum GeneActionKind {
    MoveOrganicUp,
    MoveOrganicDown,
    MoveOrganicLeft,
    MoveOrganicRight,

    MoveOrganicFromUp,
    MoveOrganicFromDown,
    MoveOrganicFromLeft,
    MoveOrganicFromRight,

    DoNothing,
    ChangeActiveGene,

    KillUpLeft,
    KillUpRight,
    KillDownLeft,
    KillDownRight,

    WaitStep,
    Die,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum MutationGeneTarget {
    SeedGene,
    ActiveGene,
    RandomGene,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum MutationEditKind {
    DirectionUp,
    DirectionDown,
    DirectionLeft,
    DirectionRight,

    Condition1,
    Condition1Param,
    Condition2,
    Condition2Param,

    AltGene1,
    AltGene2,
    AltGene3,

    AdditionalCondition1,
    AdditionalCondition1Param,
    AdditionalCondition2,
    AdditionalCondition2Param,

    AdditionalAction1,
    AdditionalAction2,
    AdditionalAction3,

    MainActionCondition,
    MainActionParam,
    MainAction,
    SelfLifespan,
    SeedGene,
}

pub fn choose_weighted<T: Copy, R: Rng + ?Sized>(items: &[Weighted<T>], rng: &mut R) -> T {
    let total: u64 = items.iter().map(|item| item.weight as u64).sum();
    assert!(total > 0, "weighted config list must contain a positive weight");

    let mut roll = rng.gen_range(0..total);
    for item in items {
        let weight = item.weight as u64;
        if roll < weight {
            return item.kind;
        }
        roll -= weight;
    }

    unreachable!("weighted selection exhausted a validated weight list")
}

impl SimulationConfig {
    pub fn load() -> Self {
        let path = env::var_os("SPECTACULIFE_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));

        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read simulation config {}: {error}", path.display())
        });
        let config: Self = ron::de::from_str(&source).unwrap_or_else(|error| {
            panic!("failed to parse simulation config {}: {error}", path.display())
        });
        config.validate().unwrap_or_else(|error| {
            panic!("invalid simulation config {}: {error}", path.display())
        });
        config
    }

    fn validate(&self) -> Result<(), String> {
        if self.world.width == 0 || self.world.height == 0 {
            return Err("world width/height must be greater than zero".into());
        }
        if self.world.organism_spacing == 0 {
            return Err("world.organism_spacing must be greater than zero".into());
        }
        validate_u8_range("world.initial_organics", self.world.initial_organics)?;
        validate_f32_range(
            "world.initial_soil_energy",
            self.world.initial_soil_energy,
        )?;
        validate_u8_range("world.initial_pollution", self.world.initial_pollution)?;

        if !(0.0..=1.0).contains(&self.environment.soil_diffusion) {
            return Err("environment.soil_diffusion must be in 0..=1".into());
        }
        if !(0.0..=1.0).contains(&self.environment.air_diffusion) {
            return Err("environment.air_diffusion must be in 0..=1".into());
        }
        if self.environment.soil_energy_render_max <= 0.0 {
            return Err("environment.soil_energy_render_max must be > 0".into());
        }

        validate_u16_range("genetics.lifespan", self.genetics.lifespan)?;
        validate_u8_range(
            "genetics.initial_mutation_rate",
            self.genetics.initial_mutation_rate,
        )?;
        if self.genetics.mutation_rate_min == 0
            || self.genetics.mutation_rate_min > self.genetics.mutation_rate_max
            || self.genetics.mutation_rate_max > 100
        {
            return Err("genetics mutation-rate limits must satisfy 1 <= min <= max <= 100".into());
        }
        if self.genetics.initial_mutation_rate.min < self.genetics.mutation_rate_min
            || self.genetics.initial_mutation_rate.max > self.genetics.mutation_rate_max
        {
            return Err("initial_mutation_rate must fit inside mutation-rate limits".into());
        }
        if self.genetics.second_mutation_edit_chance_percent > 100
            || self.genetics.mutation_rate_evolution_chance_percent > 100
        {
            return Err("genetics percentage values must be in 0..=100".into());
        }
        if self.genetics.condition_params.steps_divides_max == 0 {
            return Err("condition_params.steps_divides_max must be at least 1".into());
        }

        validate_weighted("genetics.mutation_gene_targets", &self.genetics.mutation_gene_targets)?;
        validate_weighted("genetics.direction_actions", &self.genetics.direction_actions)?;
        validate_weighted("genetics.conditions", &self.genetics.conditions)?;
        validate_weighted("genetics.gene_actions", &self.genetics.gene_actions)?;
        validate_weighted("genetics.mutation_edits", &self.genetics.mutation_edits)?;
        if self.genetics.mutation_rate_increase_weight as u64
            + self.genetics.mutation_rate_decrease_weight as u64
            == 0
        {
            return Err("mutation-rate increase/decrease weights cannot both be zero".into());
        }

        Ok(())
    }
}

fn validate_u8_range(name: &str, range: U8Range) -> Result<(), String> {
    if range.min > range.max {
        Err(format!("{name}: min must be <= max"))
    } else {
        Ok(())
    }
}

fn validate_u16_range(name: &str, range: U16Range) -> Result<(), String> {
    if range.min > range.max {
        Err(format!("{name}: min must be <= max"))
    } else {
        Ok(())
    }
}

fn validate_f32_range(name: &str, range: F32Range) -> Result<(), String> {
    if !range.min.is_finite() || !range.max.is_finite() || range.min > range.max {
        Err(format!("{name}: values must be finite and min <= max"))
    } else {
        Ok(())
    }
}

fn validate_weighted<T>(name: &str, items: &[Weighted<T>]) -> Result<(), String> {
    if items.is_empty() {
        return Err(format!("{name} cannot be empty"));
    }
    if items.iter().all(|item| item.weight == 0) {
        return Err(format!("{name} must contain at least one positive weight"));
    }
    Ok(())
}
