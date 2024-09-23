use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
};

pub const MAX_GENES: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MutationRate(pub u8);

impl Distribution<MutationRate> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> MutationRate {
        MutationRate(rng.gen_range(5..=100))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneLocation(pub u8);

impl Distribution<GeneLocation> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneLocation {
        GeneLocation(rng.gen_range(0..MAX_GENES))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifeSpan(pub u16);

impl Distribution<LifeSpan> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LifeSpan {
        LifeSpan(rng.gen_range(0..=1000))
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
    pub const fn active_gene(&self) -> Gene {
        self.get_gene(self.active_gene)
    }

    const fn get_gene(&self, loc: GeneLocation) -> Gene {
        self.genes[loc.0 as usize]
    }

    pub fn mutate(&mut self) {
        let mut rng = thread_rng();

        if rng.gen_ratio(self.mutation_rate.0 as u32, 100) {
            match rng.gen_range(0..=11) {
                0 => self.mutation_rate = rng.gen(),
                1 => self.active_gene = rng.gen(),
                2 => self.seed_gene = rng.gen(),
                _ => {
                    for i in 0..MAX_GENES {
                        if rng.gen_ratio(self.mutation_rate.0 as u32, 100) {
                            let gene = self.genes.get_mut(i as usize).unwrap();

                            match rng.gen_range(0..=22) {
                                0 => gene.up = rng.gen(),
                                1 => gene.down = rng.gen(),
                                2 => gene.left = rng.gen(),
                                3 => gene.right = rng.gen(),

                                4 => gene.condition_1 = rng.gen(),
                                5 => gene.param_1 = rng.gen(),

                                6 => gene.condition_2 = rng.gen(),
                                7 => gene.param_2 = rng.gen(),

                                8 => gene.alt_gene1 = rng.gen(),
                                9 => gene.alt_gene2 = rng.gen(),
                                10 => gene.alt_gene3 = rng.gen(),

                                11 => gene.additional_action_condition1 = rng.gen(),
                                12 => gene.additional_action_param1 = rng.gen(),

                                13 => gene.additional_action_condition2 = rng.gen(),
                                14 => gene.additional_action_param2 = rng.gen(),

                                15 => gene.additional_action1 = rng.gen(),
                                16 => gene.additional_action2 = rng.gen(),
                                17 => gene.additional_action3 = rng.gen(),

                                18 => gene.main_action = rng.gen(),
                                19 => gene.main_action_param = rng.gen(),
                                20 => gene.main_action_condition = rng.gen(),

                                21 => gene.self_lifespan = rng.gen(),

                                _ => *gene = rng.gen(),
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Distribution<Genome> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Genome {
        let seed_gene = rng.gen();
        Genome {
            active_gene: seed_gene,
            seed_gene,
            genes: rng.gen(),
            mutation_rate: rng.gen(),
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
    pub fn energy_capacity(&self) -> f32 {
        self.up.energy_capacity()
            + self.down.energy_capacity()
            + self.left.energy_capacity()
            + self.right.energy_capacity()
    }
}

impl Distribution<Gene> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Gene {
        Gene {
            up: rng.gen(),
            down: rng.gen(),
            left: rng.gen(),
            right: rng.gen(),

            main_action_condition: rng.gen(),
            main_action_param: rng.gen(),
            main_action: rng.gen(),

            additional_action_condition1: rng.gen(),
            additional_action_param1: rng.gen(),

            additional_action_condition2: rng.gen(),
            additional_action_param2: rng.gen(),

            additional_action1: rng.gen(),
            additional_action2: rng.gen(),
            additional_action3: rng.gen(),

            condition_1: rng.gen(),
            param_1: rng.gen(),

            condition_2: rng.gen(),
            param_2: rng.gen(),

            alt_gene1: rng.gen(),
            alt_gene2: rng.gen(),
            alt_gene3: rng.gen(),

            self_lifespan: rng.gen(),
        }
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
    pub fn energy_capacity(&self) -> f32 {
        use GeneDirectionAction::*;
        match self {
            MakeLeaf(_) => 1.2,
            MakeRoot(_) => 0.4,
            MakeReactor(_) => 0.8,
            MultiplySelf(_, _) => 1.2,
            CreateSeed(_) => 1.2,
            MakeFilter(_) => 0.6,
            Nothing => 0.,
            KillCell => 0.,
        }
    }
}

impl Distribution<GeneDirectionAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneDirectionAction {
        match rng.gen_range(0..=8) {
            0 => GeneDirectionAction::MultiplySelf(rng.gen(), rng.gen()),
            1 => GeneDirectionAction::MakeLeaf(rng.gen()),
            2 => GeneDirectionAction::MakeRoot(rng.gen()),
            3 => GeneDirectionAction::MakeReactor(rng.gen()),
            4 => GeneDirectionAction::MakeFilter(rng.gen()),
            5 => GeneDirectionAction::KillCell,
            6 => GeneDirectionAction::CreateSeed(rng.gen()),

            _ => GeneDirectionAction::Nothing,
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

impl Distribution<GeneCondition> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneCondition {
        use GeneCondition::*;
        match rng.gen_range(0..=31) {
            0 => LifeUp,
            1 => LifeDown,
            2 => LifeLeft,
            3 => LifeRight,

            4 => LethalOrganicUp,
            5 => LethalOrganicDown,
            6 => LethalOrganicLeft,
            7 => LethalOrganicRight,

            8 => LethalEnergyUp,
            9 => LethalEnergyDown,
            10 => LethalEnergyLeft,
            11 => LethalEnergyRight,

            12 => RandomMT,
            13 => LifeEnergyMT,

            14 => OrganicCenterMT,
            15 => OrganicUpMT,
            16 => OrganicDownMT,
            17 => OrganicLeftMT,
            18 => OrganicRightMT,

            19 => SoilEnergyCenterMT,
            20 => SoilEnergyUpMT,
            21 => SoilEnergyDownMT,
            22 => SoilEnergyLeftMT,
            23 => SoilEnergyRightMT,

            24 => AirPollutionCenterMT,
            25 => AirPollutionUpMT,
            26 => AirPollutionDownMT,
            27 => AirPollutionLeftMT,
            28 => AirPollutionRightMT,

            29 => Always,
            30 => Never,

            _ => StepsDividesP,
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
}

impl Distribution<GeneAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneAction {
        use GeneAction::*;
        match rng.gen_range(0..=13) {
            0 => MoveOrganicUp,
            1 => MoveOrganicDown,
            2 => MoveOrganicLeft,
            3 => MoveOrganicRight,

            4 => MoveOrganicFromUp,
            5 => MoveOrganicFromDown,
            6 => MoveOrganicFromLeft,
            7 => MoveOrganicFromRight,

            8 => DoNothing,

            9 => ChangeActiveGene(rng.gen()),

            10 => KillUpLeft,
            11 => KillUpRight,
            12 => KillDownLeft,
            _ => KillDownRight,
        }
    }
}
