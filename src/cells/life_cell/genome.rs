use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
};

pub const MAX_GENES: u8 = 32;
pub const MUTATION_RATE: u32 = 25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Genome {
    pub active_gene: u8,
    pub genes: [Gene; MAX_GENES as usize],
}

impl Genome {
    pub fn mutate(&mut self) {
        let mut rng = thread_rng();

        if rng.gen_ratio(MUTATION_RATE, 100) {
            let gene = self
                .genes
                .get_mut(rng.gen_range(0..MAX_GENES) as usize)
                .unwrap();
            *gene = rng.gen();
        }
    }
}

impl Distribution<Genome> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Genome {
        Genome {
            active_gene: rng.gen_range(0..MAX_GENES),
            genes: rand::random(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gene {
    pub up: GeneAction,
    pub down: GeneAction,
    pub left: GeneAction,
    pub right: GeneAction,

    pub condition: GeneCondition,
    pub secondary_gene: u8,
    pub param: u8,
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

            condition: rng.gen(),
            secondary_gene: rng.gen_range(0..MAX_GENES),
            param: rng.gen(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneAction {
    MakeLeaf(u8),
    MakeRoot(u8),
    MakeReactor(u8),
    MakeFilter(u8),
    MultiplySelf(
        /// LifeSpan
        u8,
        /// NextGene
        u8,
    ),
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
        match rng.gen_range(0..=12) {
            0 => GeneAction::MultiplySelf(rng.gen_range(1..255), rng.gen_range(0..MAX_GENES)),
            1 => GeneAction::MakeLeaf(rng.gen_range(1..255)),
            2 => GeneAction::MakeRoot(rng.gen_range(1..255)),
            3 => GeneAction::MakeReactor(rng.gen_range(1..255)),
            4 => GeneAction::KillCell,
            5 => GeneAction::MakeFilter(rng.gen_range(1..255)),

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
}

impl Distribution<GeneCondition> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneCondition {
        use GeneCondition::*;
        match rng.gen_range(0..=13) {
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
            _ => AirPollutionRightMT,
        }
    }
}
