use bevy::prelude::Resource;
use rand::Rng;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};

pub const DEFAULT_CONFIG_PATH: &str = "assets/simulation.ron";
pub const DEFAULT_RENDER_CONFIG_PATH: &str = "assets/render.ron";

#[derive(Debug, Clone, Resource, Deserialize)]
pub struct SimulationConfig {
    pub world: WorldConfig,
    pub environment: EnvironmentConfig,
    pub life: LifeConfig,
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnvironmentConfig {
    pub soil_diffusion: f32,
    pub air_diffusion: f32,
}

#[derive(Debug, Clone, Resource, Deserialize)]
pub struct RenderConfig {
    pub soil_energy_render_max: f32,
    /// Start fading the detailed tile renderer into the trilinear mipmapped
    /// overview at this camera scale.
    pub mip_lod_fade_start: f32,
    /// At this scale the overview is fully active and detailed tiles are hidden.
    pub mip_lod_fade_end: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LifeConfig {
    pub lethal_organics: u8,
    pub lethal_soil_energy: f32,
    pub newborn_energy_consumption_multiplier: f32,
    pub reproduction: ReproductionConfig,
    pub transfer: TransferConfig,
    pub collision: CollisionConfig,
    pub predation: PredationConfig,
    pub death: DeathConfig,
    pub consumption: CellConsumptionConfig,
    pub organics: CellOrganicsConfig,
    pub growth_energy: GrowthEnergyConfig,
    pub generators: GeneratorConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReproductionConfig {
    /// Energy placed into a newly constructed, still-attached seed.
    pub seed_initial_energy: f32,
    /// Stored energy required before an attached seed becomes an independent Stem.
    pub seed_maturation_energy: f32,
    /// Maximum energy an attached seed can accept from its parent per tick.
    pub seed_max_charge_per_tick: f32,
    /// Maximum time an attached seed can wait for maturation.
    pub seed_lifespan: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransferConfig {
    pub reserve_consumption_multiplier: f32,
    /// None = no throughput cap. Some(x) = at most x energy leaves a cell per tick.
    pub max_energy_per_tick: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollisionConfig {
    pub self_damage: u16,
    pub foreign_damage: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PredationConfig {
    /// None = take all stored energy from a killed cell.
    pub max_energy_gain: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeathConfig {
    pub energy_to_soil_fraction: f32,
    pub pollution_per_organic_divisor: u8,
    pub minimum_pollution: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CellConsumptionConfig {
    pub pipe: f32,
    pub leaf: f32,
    pub stem: f32,
    pub seed: f32,
    pub root: f32,
    pub reactor: f32,
    pub filter: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CellOrganicsConfig {
    pub pipe: u8,
    pub leaf: u8,
    pub stem: u8,
    pub seed: u8,
    pub root: u8,
    pub reactor: u8,
    pub filter: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrowthEnergyConfig {
    pub leaf: f32,
    pub root: f32,
    pub reactor: f32,
    pub filter: f32,
    pub multiply_self: f32,
    pub create_seed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratorConfig {
    pub leaf: LeafGeneratorConfig,
    pub root: RootGeneratorConfig,
    pub reactor: ReactorGeneratorConfig,
    pub filter: FilterGeneratorConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LeafGeneratorConfig {
    pub energy_per_tick: f32,
    pub pollution_divisor: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RootGeneratorConfig {
    pub low_resource_threshold: u8,
    pub low_resource_take: u8,
    pub extraction_fraction: f32,
    pub energy_efficiency: f32,
    pub soil_energy_output: f32,
    pub pollution_output: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReactorGeneratorConfig {
    pub extraction_fraction: f32,
    pub energy_efficiency: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilterGeneratorConfig {
    pub low_resource_threshold: u8,
    pub low_resource_take: u8,
    pub extraction_fraction: f32,
    pub energy_efficiency: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneticsConfig {
    /// Interpret genome Up/Down/Left/Right in the cell's body frame:
    /// forward/back/left/right. This makes programs rotationally invariant.
    pub relative_directions: bool,

    pub lifespan: U16Range,
    pub initial_mutation_rate: U8Range,
    pub mutation_rate_min: u8,
    pub mutation_rate_max: u8,

    /// Strong inherited mutation performed only when CreateSeed constructs a seed.
    pub seed_mutation: SeedMutationConfig,
    /// Rare local copy errors on a MultiplySelf daughter.
    pub somatic_mutation: SomaticMutationConfig,
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
pub struct SeedMutationConfig {
    /// Number of point edits made to each gene selected by the genome mutation rate.
    pub edits_per_affected_gene: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SomaticMutationConfig {
    /// Base chance per million MultiplySelf copies at mutation_rate=100.
    /// The actual chance is scaled by the genome's mutation_rate.
    pub chance_per_million: u32,
    pub edits: u16,
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
    // Replace a complete directional instruction.
    DirectionUp,
    DirectionDown,
    DirectionLeft,
    DirectionRight,

    // Smaller directional edits preserve the rest of a useful instruction.
    DirectionUpLifespan,
    DirectionDownLifespan,
    DirectionLeftLifespan,
    DirectionRightLifespan,
    DirectionUpNextGene,
    DirectionDownNextGene,
    DirectionLeftNextGene,
    DirectionRightNextGene,

    // Copying an already useful direction is a cheap path to symmetry/branching.
    CopyDownToUp,
    CopyLeftToUp,
    CopyRightToUp,
    CopyUpToDown,
    CopyLeftToDown,
    CopyRightToDown,
    CopyUpToLeft,
    CopyDownToLeft,
    CopyRightToLeft,
    CopyUpToRight,
    CopyDownToRight,
    CopyLeftToRight,

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
    WholeGene,
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

impl RenderConfig {
    pub fn load() -> Self {
        let path = env::var_os("SPECTACULIFE_RENDER_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_RENDER_CONFIG_PATH));
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read render config {}: {error}", path.display()));
        let config: Self = ron::de::from_str(&source)
            .unwrap_or_else(|error| panic!("failed to parse render config {}: {error}", path.display()));
        config.validate().unwrap_or_else(|error| {
            panic!("invalid render config {}: {error}", path.display())
        });
        config
    }

    fn validate(&self) -> Result<(), String> {
        if self.soil_energy_render_max <= 0.0 || !self.soil_energy_render_max.is_finite() {
            return Err("soil_energy_render_max must be finite and > 0".into());
        }
        if !self.mip_lod_fade_start.is_finite()
            || !self.mip_lod_fade_end.is_finite()
            || self.mip_lod_fade_start <= 0.0
            || self.mip_lod_fade_end <= self.mip_lod_fade_start
        {
            return Err("mip LOD must satisfy 0 < mip_lod_fade_start < mip_lod_fade_end".into());
        }
        Ok(())
    }
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
        validate_nonnegative("life.lethal_soil_energy", self.life.lethal_soil_energy)?;
        validate_nonnegative(
            "life.newborn_energy_consumption_multiplier",
            self.life.newborn_energy_consumption_multiplier,
        )?;
        validate_nonnegative(
            "life.reproduction.seed_initial_energy",
            self.life.reproduction.seed_initial_energy,
        )?;
        validate_nonnegative(
            "life.reproduction.seed_maturation_energy",
            self.life.reproduction.seed_maturation_energy,
        )?;
        validate_nonnegative(
            "life.reproduction.seed_max_charge_per_tick",
            self.life.reproduction.seed_max_charge_per_tick,
        )?;
        if self.life.reproduction.seed_maturation_energy <= self.life.reproduction.seed_initial_energy {
            return Err("life.reproduction.seed_maturation_energy must exceed seed_initial_energy".into());
        }
        if self.life.reproduction.seed_max_charge_per_tick <= 0.0 {
            return Err("life.reproduction.seed_max_charge_per_tick must be > 0".into());
        }
        if self.life.reproduction.seed_lifespan == 0 {
            return Err("life.reproduction.seed_lifespan must be > 0".into());
        }
        validate_nonnegative(
            "life.transfer.reserve_consumption_multiplier",
            self.life.transfer.reserve_consumption_multiplier,
        )?;
        validate_optional_nonnegative(
            "life.transfer.max_energy_per_tick",
            self.life.transfer.max_energy_per_tick,
        )?;
        validate_optional_nonnegative(
            "life.predation.max_energy_gain",
            self.life.predation.max_energy_gain,
        )?;
        if self.life.death.pollution_per_organic_divisor == 0 {
            return Err("life.death.pollution_per_organic_divisor must be > 0".into());
        }
        validate_fraction(
            "life.death.energy_to_soil_fraction",
            self.life.death.energy_to_soil_fraction,
        )?;
        for (name, value) in [
            ("life.consumption.pipe", self.life.consumption.pipe),
            ("life.consumption.leaf", self.life.consumption.leaf),
            ("life.consumption.stem", self.life.consumption.stem),
            ("life.consumption.seed", self.life.consumption.seed),
            ("life.consumption.root", self.life.consumption.root),
            ("life.consumption.reactor", self.life.consumption.reactor),
            ("life.consumption.filter", self.life.consumption.filter),
            ("life.growth_energy.leaf", self.life.growth_energy.leaf),
            ("life.growth_energy.root", self.life.growth_energy.root),
            ("life.growth_energy.reactor", self.life.growth_energy.reactor),
            ("life.growth_energy.filter", self.life.growth_energy.filter),
            ("life.growth_energy.multiply_self", self.life.growth_energy.multiply_self),
            ("life.growth_energy.create_seed", self.life.growth_energy.create_seed),
            ("life.generators.leaf.energy_per_tick", self.life.generators.leaf.energy_per_tick),
            ("life.generators.leaf.pollution_divisor", self.life.generators.leaf.pollution_divisor),
            ("life.generators.root.extraction_fraction", self.life.generators.root.extraction_fraction),
            ("life.generators.root.energy_efficiency", self.life.generators.root.energy_efficiency),
            ("life.generators.root.soil_energy_output", self.life.generators.root.soil_energy_output),
            ("life.generators.root.pollution_output", self.life.generators.root.pollution_output),
            ("life.generators.reactor.extraction_fraction", self.life.generators.reactor.extraction_fraction),
            ("life.generators.reactor.energy_efficiency", self.life.generators.reactor.energy_efficiency),
            ("life.generators.filter.extraction_fraction", self.life.generators.filter.extraction_fraction),
            ("life.generators.filter.energy_efficiency", self.life.generators.filter.energy_efficiency),
        ] {
            validate_nonnegative(name, value)?;
        }
        if self.life.generators.leaf.pollution_divisor == 0.0 {
            return Err("life.generators.leaf.pollution_divisor must be > 0".into());
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
        if self.genetics.seed_mutation.edits_per_affected_gene == 0 {
            return Err("genetics.seed_mutation.edits_per_affected_gene must be at least 1".into());
        }
        if self.genetics.somatic_mutation.chance_per_million > 1_000_000 {
            return Err("genetics.somatic_mutation.chance_per_million must be <= 1000000".into());
        }
        if self.genetics.somatic_mutation.chance_per_million != 0
            && self.genetics.somatic_mutation.edits == 0
        {
            return Err("genetics.somatic_mutation.edits must be at least 1 when enabled".into());
        }
        if self.genetics.mutation_rate_evolution_chance_percent > 100 {
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

fn validate_nonnegative(name: &str, value: f32) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 {
        Err(format!("{name} must be finite and >= 0"))
    } else {
        Ok(())
    }
}

fn validate_optional_nonnegative(name: &str, value: Option<f32>) -> Result<(), String> {
    if let Some(value) = value {
        validate_nonnegative(name, value)?;
    }
    Ok(())
}

fn validate_fraction(name: &str, value: f32) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        Err(format!("{name} must be in 0..=1"))
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
