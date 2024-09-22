use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
};

pub const MAX_GENES: u8 = 32;
pub const MUTATION_RATE: u32 = 25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneLocation {
    pub l: u8,
}

impl Distribution<GeneLocation> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneLocation {
        GeneLocation {
            l: rng.gen_range(0..MAX_GENES),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifeSpan {
    pub l: u8,
}

impl Distribution<LifeSpan> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LifeSpan {
        LifeSpan { l: rng.gen() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Genome {
    pub active_gene: GeneLocation,
    pub genes: [Gene; MAX_GENES as usize],
}

impl Genome {
    pub const fn get_active_gene(&self) -> Gene {
        self.get_gene(self.active_gene)
    }

    const fn get_gene(&self, loc: GeneLocation) -> Gene {
        self.genes[loc.l as usize]
    }

    pub fn mutate(&mut self) {
        let mut rng = thread_rng();

        if rng.gen_ratio(MUTATION_RATE, 100) {
            let gene = self
                .genes
                .get_mut(rng.gen_range(0..MAX_GENES) as usize)
                .unwrap();

            match rng.gen_range(0..12) {
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

                _ => *gene = rng.gen(),
            }
        }
    }
}

impl Distribution<Genome> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Genome {
        Genome {
            active_gene: rng.gen(),
            genes: rng.gen(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gene {
    pub up: GeneAction,
    pub down: GeneAction,
    pub left: GeneAction,
    pub right: GeneAction,

    pub condition_1: GeneCondition,
    pub param_1: u8,

    pub condition_2: GeneCondition,
    pub param_2: u8,

    pub alt_gene1: GeneLocation,
    pub alt_gene2: GeneLocation,
    pub alt_gene3: GeneLocation,
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

            condition_1: rng.gen(),
            param_1: rng.gen(),

            condition_2: rng.gen(),
            param_2: rng.gen(),

            alt_gene1: rng.gen(),
            alt_gene2: rng.gen(),
            alt_gene3: rng.gen(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneAction {
    MakeLeaf(LifeSpan),
    MakeRoot(LifeSpan),
    MakeReactor(LifeSpan),
    MakeFilter(LifeSpan),
    MultiplySelf(LifeSpan, GeneLocation),
    KillCell,
    Nothing,
}

impl GeneAction {
    pub const fn energy_capacity(&self) -> f32 {
        match self {
            GeneAction::MakeLeaf(_) => 1.,
            GeneAction::MakeRoot(_) => 1.,
            GeneAction::MakeReactor(_) => 1.4,
            GeneAction::MultiplySelf(_, _) => 0.2,
            GeneAction::Nothing => 0.,
            GeneAction::KillCell => 0.5,
            GeneAction::MakeFilter(_) => 1.,
        }
    }
}

impl Distribution<GeneAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneAction {
        match rng.gen_range(0..12) {
            0 => GeneAction::MultiplySelf(rng.gen(), rng.gen()),
            1 => GeneAction::MakeLeaf(rng.gen()),
            2 => GeneAction::MakeRoot(rng.gen()),
            3 => GeneAction::MakeReactor(rng.gen()),
            4 => GeneAction::MakeFilter(rng.gen()),
            5 => GeneAction::KillCell,

            _ => GeneAction::Nothing,
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
