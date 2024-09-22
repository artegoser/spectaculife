use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
};

pub const MAX_GENES: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MutationRate(pub u8);

impl Distribution<MutationRate> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> MutationRate {
        MutationRate(rng.gen_range(5..100))
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
pub struct LifeSpan(pub u8);

impl Distribution<LifeSpan> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LifeSpan {
        LifeSpan(rng.gen())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Genome {
    pub genes: [Gene; MAX_GENES as usize],
    pub active_gene: GeneLocation,
    pub mutation_rate: MutationRate,
}

impl Genome {
    pub const fn get_active_gene(&self) -> Gene {
        self.get_gene(self.active_gene)
    }

    const fn get_gene(&self, loc: GeneLocation) -> Gene {
        self.genes[loc.0 as usize]
    }

    pub fn mutate(&mut self) {
        let mut rng = thread_rng();

        if rng.gen_ratio(self.mutation_rate.0 as u32, 100) {
            match rng.gen_range(0..3) {
                1 => self.mutation_rate = rng.gen(),
                2 => self.active_gene = rng.gen(),
                _ => {
                    for i in 0..MAX_GENES {
                        if rng.gen_ratio(self.mutation_rate.0 as u32, 100) {
                            let gene = self.genes.get_mut(i as usize).unwrap();

                            match rng.gen_range(0..=15) {
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

                                11 => gene.action_condition = rng.gen(),
                                12 => gene.action_param = rng.gen(),
                                13 => gene.action = rng.gen(),

                                14 => gene.alt_gene_action = rng.gen(),

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
        Genome {
            active_gene: rng.gen(),
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

    pub action_condition: Option<GeneCondition>,
    pub action_param: u8,
    pub action: GeneAction,

    pub condition_1: GeneCondition,
    pub param_1: u8,

    pub condition_2: GeneCondition,
    pub param_2: u8,

    pub alt_gene1: GeneLocation,
    pub alt_gene2: GeneLocation,
    pub alt_gene3: GeneLocation,
    pub alt_gene_action: GeneLocation,
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

            action_condition: rng.gen(),
            action_param: rng.gen(),
            action: rng.gen(),

            condition_1: rng.gen(),
            param_1: rng.gen(),

            condition_2: rng.gen(),
            param_2: rng.gen(),

            alt_gene1: rng.gen(),
            alt_gene2: rng.gen(),
            alt_gene3: rng.gen(),

            alt_gene_action: rng.gen(),
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
    Nothing,
}

impl GeneDirectionAction {
    pub const fn energy_capacity(&self) -> f32 {
        match self {
            GeneDirectionAction::MakeLeaf(_) => 1.,
            GeneDirectionAction::MakeRoot(_) => 1.,
            GeneDirectionAction::MakeReactor(_) => 1.4,
            GeneDirectionAction::MultiplySelf(_, _) => 0.2,
            GeneDirectionAction::Nothing => 0.,
            GeneDirectionAction::KillCell => 0.5,
            GeneDirectionAction::MakeFilter(_) => 1.,
        }
    }
}

impl Distribution<GeneDirectionAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneDirectionAction {
        match rng.gen_range(0..=6) {
            0 => GeneDirectionAction::MultiplySelf(rng.gen(), rng.gen()),
            1 => GeneDirectionAction::MakeLeaf(rng.gen()),
            2 => GeneDirectionAction::MakeRoot(rng.gen()),
            3 => GeneDirectionAction::MakeReactor(rng.gen()),
            4 => GeneDirectionAction::MakeFilter(rng.gen()),
            5 => GeneDirectionAction::KillCell,

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
}

impl Distribution<GeneCondition> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneCondition {
        use GeneCondition::*;
        match rng.gen_range(0..=30) {
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
            _ => Never,
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

    MoveOrganicUpFromDown,
    MoveOrganicUpFromLeft,
    MoveOrganicUpFromRight,

    MoveOrganicDownFromUp,
    MoveOrganicDownFromLeft,
    MoveOrganicDownFromRight,

    MoveOrganicLeftFromUp,
    MoveOrganicLeftFromDown,
    MoveOrganicLeftFromRight,

    MoveOrganicRightFromUp,
    MoveOrganicRightFromDown,
    MoveOrganicRightFromLeft,
}

impl Distribution<GeneAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneAction {
        use GeneAction::*;
        match rng.gen_range(0..=19) {
            0 => MoveOrganicUp,
            1 => MoveOrganicDown,
            2 => MoveOrganicLeft,
            3 => MoveOrganicRight,

            4 => MoveOrganicFromUp,
            5 => MoveOrganicFromDown,
            6 => MoveOrganicFromLeft,
            7 => MoveOrganicFromRight,

            8 => MoveOrganicUpFromDown,
            9 => MoveOrganicUpFromLeft,
            10 => MoveOrganicUpFromRight,

            11 => MoveOrganicDownFromUp,
            12 => MoveOrganicDownFromLeft,
            13 => MoveOrganicDownFromRight,

            14 => MoveOrganicLeftFromUp,
            15 => MoveOrganicLeftFromDown,
            16 => MoveOrganicLeftFromRight,

            17 => MoveOrganicRightFromUp,
            18 => MoveOrganicRightFromDown,
            _ => MoveOrganicRightFromLeft,
        }
    }
}
