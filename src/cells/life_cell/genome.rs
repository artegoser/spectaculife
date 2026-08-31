use rand::{thread_rng, Rng};

use crate::config::{
    choose_weighted, ConditionKind, ConditionParamConfig, DirectionActionKind, GeneActionKind,
    GeneticsConfig, MutationEditKind, MutationGeneTarget,
};

pub const MAX_GENES: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GenomeHandle(pub u32);

#[derive(Debug, Clone)]
pub struct GenomePool {
    genomes: Vec<Genome>,
    free_list: Vec<u32>,
    allocated: Vec<bool>,
}

impl GenomePool {
    pub fn new() -> Self {
        Self {
            genomes: Vec::new(),
            free_list: Vec::new(),
            allocated: Vec::new(),
        }
    }

    pub fn alloc(&mut self, genome: Genome) -> GenomeHandle {
        if let Some(idx) = self.free_list.pop() {
            self.genomes[idx as usize] = genome;
            self.allocated[idx as usize] = true;
            GenomeHandle(idx)
        } else {
            let idx = self.genomes.len() as u32;
            self.genomes.push(genome);
            self.allocated.push(true);
            GenomeHandle(idx)
        }
    }

    pub fn free(&mut self, handle: GenomeHandle) {
        let idx = handle.0 as usize;
        assert!(idx < self.genomes.len(), "invalid GenomeHandle {}", handle.0);

        if !self.allocated[idx] {
            return;
        }

        self.allocated[idx] = false;
        self.free_list.push(handle.0);
    }

    pub fn get(&self, handle: GenomeHandle) -> &Genome {
        let idx = handle.0 as usize;
        assert!(
            self.allocated.get(idx).copied().unwrap_or(false),
            "use of freed GenomeHandle {}",
            handle.0
        );
        &self.genomes[idx]
    }

    pub fn get_mut(&mut self, handle: GenomeHandle) -> &mut Genome {
        let idx = handle.0 as usize;
        assert!(
            self.allocated.get(idx).copied().unwrap_or(false),
            "use of freed GenomeHandle {}",
            handle.0
        );
        &mut self.genomes[idx]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MutationRate(pub u8);

impl MutationRate {
    fn random<R: Rng + ?Sized>(rng: &mut R, config: &GeneticsConfig) -> Self {
        Self(config.initial_mutation_rate.sample(rng))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneLocation(pub u8);

impl GeneLocation {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        Self(rng.gen_range(0..MAX_GENES))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifeSpan(pub u16);

impl LifeSpan {
    fn random<R: Rng + ?Sized>(rng: &mut R, config: &GeneticsConfig) -> Self {
        Self(config.lifespan.sample(rng))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Genome {
    pub genes: [Gene; MAX_GENES as usize],
    pub active_gene: GeneLocation,
    pub seed_gene: GeneLocation,
    pub mutation_rate: MutationRate,
}

impl Genome {
    pub fn random<R: Rng + ?Sized>(rng: &mut R, config: &GeneticsConfig) -> Self {
        let seed_gene = GeneLocation::random(rng);
        Self {
            active_gene: seed_gene,
            seed_gene,
            genes: std::array::from_fn(|_| Gene::random(rng, config)),
            mutation_rate: MutationRate::random(rng, config),
        }
    }

    pub const fn active_gene(&self) -> Gene {
        self.get_gene(self.active_gene)
    }

    pub const fn get_gene(&self, loc: GeneLocation) -> Gene {
        self.genes[loc.0 as usize]
    }

    pub fn mutate(&mut self, config: &GeneticsConfig) {
        let mut rng = thread_rng();
        let rate = self
            .mutation_rate
            .0
            .clamp(config.mutation_rate_min, config.mutation_rate_max) as u32;

        if !rng.gen_ratio(rate, 100) {
            return;
        }

        self.mutate_one(&mut rng, config);
        if rng.gen_ratio(
            config.second_mutation_edit_chance_percent as u32,
            100,
        ) {
            self.mutate_one(&mut rng, config);
        }

        if rng.gen_ratio(
            config.mutation_rate_evolution_chance_percent as u32,
            100,
        ) {
            let total = config.mutation_rate_increase_weight as u64
                + config.mutation_rate_decrease_weight as u64;
            let increase = rng.gen_range(0..total) < config.mutation_rate_increase_weight as u64;

            if increase {
                self.mutation_rate.0 = self
                    .mutation_rate
                    .0
                    .saturating_add(1)
                    .min(config.mutation_rate_max);
            } else {
                self.mutation_rate.0 = self
                    .mutation_rate
                    .0
                    .saturating_sub(1)
                    .max(config.mutation_rate_min);
            }
        }
    }

    fn mutate_one<R: Rng + ?Sized>(&mut self, rng: &mut R, config: &GeneticsConfig) {
        use MutationEditKind::*;

        let mutation = choose_weighted(&config.mutation_edits, rng);
        if matches!(mutation, SeedGene) {
            self.seed_gene = GeneLocation::random(rng);
            return;
        }

        let gene_idx = match choose_weighted(&config.mutation_gene_targets, rng) {
            MutationGeneTarget::SeedGene => self.seed_gene.0 as usize,
            MutationGeneTarget::ActiveGene => self.active_gene.0 as usize,
            MutationGeneTarget::RandomGene => rng.gen_range(0..MAX_GENES as usize),
        };
        let gene = &mut self.genes[gene_idx];

        match mutation {
            DirectionUp => gene.up = GeneDirectionAction::random(rng, config),
            DirectionDown => gene.down = GeneDirectionAction::random(rng, config),
            DirectionLeft => gene.left = GeneDirectionAction::random(rng, config),
            DirectionRight => gene.right = GeneDirectionAction::random(rng, config),

            Condition1 => {
                gene.condition_1 = GeneCondition::random(rng, config);
                gene.param_1 = gene
                    .condition_1
                    .random_param(rng, &config.condition_params);
            }
            Condition1Param => {
                gene.param_1 = gene
                    .condition_1
                    .random_param(rng, &config.condition_params)
            }
            Condition2 => {
                gene.condition_2 = GeneCondition::random(rng, config);
                gene.param_2 = gene
                    .condition_2
                    .random_param(rng, &config.condition_params);
            }
            Condition2Param => {
                gene.param_2 = gene
                    .condition_2
                    .random_param(rng, &config.condition_params)
            }

            AltGene1 => gene.alt_gene1 = GeneLocation::random(rng),
            AltGene2 => gene.alt_gene2 = GeneLocation::random(rng),
            AltGene3 => gene.alt_gene3 = GeneLocation::random(rng),

            AdditionalCondition1 => {
                gene.additional_action_condition1 = GeneCondition::random(rng, config);
                gene.additional_action_param1 = gene
                    .additional_action_condition1
                    .random_param(rng, &config.condition_params);
            }
            AdditionalCondition1Param => {
                gene.additional_action_param1 = gene
                    .additional_action_condition1
                    .random_param(rng, &config.condition_params)
            }
            AdditionalCondition2 => {
                gene.additional_action_condition2 = GeneCondition::random(rng, config);
                gene.additional_action_param2 = gene
                    .additional_action_condition2
                    .random_param(rng, &config.condition_params);
            }
            AdditionalCondition2Param => {
                gene.additional_action_param2 = gene
                    .additional_action_condition2
                    .random_param(rng, &config.condition_params)
            }

            AdditionalAction1 => gene.additional_action1 = GeneAction::random(rng, config),
            AdditionalAction2 => gene.additional_action2 = GeneAction::random(rng, config),
            AdditionalAction3 => gene.additional_action3 = GeneAction::random(rng, config),

            MainActionCondition => {
                gene.main_action_condition = GeneCondition::random(rng, config);
                gene.main_action_param = gene
                    .main_action_condition
                    .random_param(rng, &config.condition_params);
            }
            MainActionParam => {
                gene.main_action_param = gene
                    .main_action_condition
                    .random_param(rng, &config.condition_params)
            }
            MainAction => gene.main_action = GeneAction::random(rng, config),
            SelfLifespan => gene.self_lifespan = LifeSpan::random(rng, config),
            SeedGene => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gene {
    pub up: GeneDirectionAction,
    pub down: GeneDirectionAction,
    pub left: GeneDirectionAction,
    pub right: GeneDirectionAction,

    pub main_action_condition: GeneCondition,
    pub main_action_param: u8,
    pub main_action: GeneAction,

    pub additional_action_condition1: GeneCondition,
    pub additional_action_param1: u8,

    pub additional_action_condition2: GeneCondition,
    pub additional_action_param2: u8,

    pub additional_action1: GeneAction,
    pub additional_action2: GeneAction,
    pub additional_action3: GeneAction,

    pub condition_1: GeneCondition,
    pub param_1: u8,

    pub condition_2: GeneCondition,
    pub param_2: u8,

    pub alt_gene1: GeneLocation,
    pub alt_gene2: GeneLocation,
    pub alt_gene3: GeneLocation,

    pub self_lifespan: LifeSpan,
}

impl Gene {
    fn random<R: Rng + ?Sized>(rng: &mut R, config: &GeneticsConfig) -> Self {
        let main_action_condition = GeneCondition::random(rng, config);
        let additional_action_condition1 = GeneCondition::random(rng, config);
        let additional_action_condition2 = GeneCondition::random(rng, config);
        let condition_1 = GeneCondition::random(rng, config);
        let condition_2 = GeneCondition::random(rng, config);

        Self {
            up: GeneDirectionAction::random(rng, config),
            down: GeneDirectionAction::random(rng, config),
            left: GeneDirectionAction::random(rng, config),
            right: GeneDirectionAction::random(rng, config),

            main_action_condition,
            main_action_param: main_action_condition.random_param(rng, &config.condition_params),
            main_action: GeneAction::random(rng, config),

            additional_action_condition1,
            additional_action_param1: additional_action_condition1
                .random_param(rng, &config.condition_params),

            additional_action_condition2,
            additional_action_param2: additional_action_condition2
                .random_param(rng, &config.condition_params),

            additional_action1: GeneAction::random(rng, config),
            additional_action2: GeneAction::random(rng, config),
            additional_action3: GeneAction::random(rng, config),

            condition_1,
            param_1: condition_1.random_param(rng, &config.condition_params),

            condition_2,
            param_2: condition_2.random_param(rng, &config.condition_params),

            alt_gene1: GeneLocation::random(rng),
            alt_gene2: GeneLocation::random(rng),
            alt_gene3: GeneLocation::random(rng),

            self_lifespan: LifeSpan::random(rng, config),
        }
    }

    pub fn energy_capacity(&self) -> f32 {
        self.up.energy_capacity()
            + self.down.energy_capacity()
            + self.left.energy_capacity()
            + self.right.energy_capacity()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneDirectionAction {
    MakeLeaf(LifeSpan),
    MakeRoot(LifeSpan),
    MakeReactor(LifeSpan),
    MakeFilter(LifeSpan),
    MultiplySelf(LifeSpan, GeneLocation),
    KillCell,
    CreateSeed(LifeSpan),
    Nothing,
}

impl GeneDirectionAction {
    fn random<R: Rng + ?Sized>(rng: &mut R, config: &GeneticsConfig) -> Self {
        match choose_weighted(&config.direction_actions, rng) {
            DirectionActionKind::MultiplySelf => {
                Self::MultiplySelf(LifeSpan::random(rng, config), GeneLocation::random(rng))
            }
            DirectionActionKind::MakeLeaf => Self::MakeLeaf(LifeSpan::random(rng, config)),
            DirectionActionKind::MakeRoot => Self::MakeRoot(LifeSpan::random(rng, config)),
            DirectionActionKind::MakeReactor => Self::MakeReactor(LifeSpan::random(rng, config)),
            DirectionActionKind::MakeFilter => Self::MakeFilter(LifeSpan::random(rng, config)),
            DirectionActionKind::CreateSeed => Self::CreateSeed(LifeSpan::random(rng, config)),
            DirectionActionKind::Nothing => Self::Nothing,
            DirectionActionKind::KillCell => Self::KillCell,
        }
    }

    pub fn energy_capacity(&self) -> f32 {
        use GeneDirectionAction::*;
        match self {
            MakeLeaf(_) => 1.2,
            MakeRoot(_) => 0.4,
            MakeReactor(_) => 0.8,
            MultiplySelf(_, _) => 0.8,
            CreateSeed(_) => 0.8,
            MakeFilter(_) => 0.6,
            Nothing => 0.,
            KillCell => 0.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneCondition {
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

impl GeneCondition {
    fn random<R: Rng + ?Sized>(rng: &mut R, config: &GeneticsConfig) -> Self {
        use ConditionKind::*;
        match choose_weighted(&config.conditions, rng) {
            LifeUp => Self::LifeUp,
            LifeDown => Self::LifeDown,
            LifeLeft => Self::LifeLeft,
            LifeRight => Self::LifeRight,
            LethalOrganicUp => Self::LethalOrganicUp,
            LethalOrganicDown => Self::LethalOrganicDown,
            LethalOrganicLeft => Self::LethalOrganicLeft,
            LethalOrganicRight => Self::LethalOrganicRight,
            LethalEnergyUp => Self::LethalEnergyUp,
            LethalEnergyDown => Self::LethalEnergyDown,
            LethalEnergyLeft => Self::LethalEnergyLeft,
            LethalEnergyRight => Self::LethalEnergyRight,
            RandomMT => Self::RandomMT,
            LifeEnergyMT => Self::LifeEnergyMT,
            OrganicCenterMT => Self::OrganicCenterMT,
            OrganicUpMT => Self::OrganicUpMT,
            OrganicDownMT => Self::OrganicDownMT,
            OrganicLeftMT => Self::OrganicLeftMT,
            OrganicRightMT => Self::OrganicRightMT,
            SoilEnergyCenterMT => Self::SoilEnergyCenterMT,
            SoilEnergyUpMT => Self::SoilEnergyUpMT,
            SoilEnergyDownMT => Self::SoilEnergyDownMT,
            SoilEnergyLeftMT => Self::SoilEnergyLeftMT,
            SoilEnergyRightMT => Self::SoilEnergyRightMT,
            AirPollutionCenterMT => Self::AirPollutionCenterMT,
            AirPollutionUpMT => Self::AirPollutionUpMT,
            AirPollutionDownMT => Self::AirPollutionDownMT,
            AirPollutionLeftMT => Self::AirPollutionLeftMT,
            AirPollutionRightMT => Self::AirPollutionRightMT,
            Always => Self::Always,
            Never => Self::Never,
            StepsDividesP => Self::StepsDividesP,
        }
    }

    fn random_param<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        config: &ConditionParamConfig,
    ) -> u8 {
        use GeneCondition::*;

        match self {
            LifeUp | LifeDown | LifeLeft | LifeRight
            | LethalOrganicUp | LethalOrganicDown | LethalOrganicLeft | LethalOrganicRight
            | LethalEnergyUp | LethalEnergyDown | LethalEnergyLeft | LethalEnergyRight
            | Always | Never => 0,
            RandomMT => rng.gen(),
            LifeEnergyMT => rng.gen_range(0..=config.life_energy_max),
            OrganicCenterMT | OrganicUpMT | OrganicDownMT | OrganicLeftMT | OrganicRightMT => {
                rng.gen_range(0..=config.organics_max)
            }
            SoilEnergyCenterMT | SoilEnergyUpMT | SoilEnergyDownMT | SoilEnergyLeftMT
            | SoilEnergyRightMT => rng.gen_range(0..=config.soil_energy_max),
            AirPollutionCenterMT | AirPollutionUpMT | AirPollutionDownMT | AirPollutionLeftMT
            | AirPollutionRightMT => rng.gen_range(0..=config.pollution_max),
            StepsDividesP => rng.gen_range(1..=config.steps_divides_max),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneAction {
    MoveOrganicUp,
    MoveOrganicDown,
    MoveOrganicLeft,
    MoveOrganicRight,

    MoveOrganicFromUp,
    MoveOrganicFromDown,
    MoveOrganicFromLeft,
    MoveOrganicFromRight,

    DoNothing,
    ChangeActiveGene(GeneLocation),

    KillUpLeft,
    KillUpRight,
    KillDownLeft,
    KillDownRight,

    WaitStep,
    Die,
}

impl GeneAction {
    fn random<R: Rng + ?Sized>(rng: &mut R, config: &GeneticsConfig) -> Self {
        use GeneActionKind::*;
        match choose_weighted(&config.gene_actions, rng) {
            MoveOrganicUp => Self::MoveOrganicUp,
            MoveOrganicDown => Self::MoveOrganicDown,
            MoveOrganicLeft => Self::MoveOrganicLeft,
            MoveOrganicRight => Self::MoveOrganicRight,
            MoveOrganicFromUp => Self::MoveOrganicFromUp,
            MoveOrganicFromDown => Self::MoveOrganicFromDown,
            MoveOrganicFromLeft => Self::MoveOrganicFromLeft,
            MoveOrganicFromRight => Self::MoveOrganicFromRight,
            DoNothing => Self::DoNothing,
            ChangeActiveGene => Self::ChangeActiveGene(GeneLocation::random(rng)),
            KillUpLeft => Self::KillUpLeft,
            KillUpRight => Self::KillUpRight,
            KillDownLeft => Self::KillDownLeft,
            KillDownRight => Self::KillDownRight,
            WaitStep => Self::WaitStep,
            Die => Self::Die,
        }
    }
}
