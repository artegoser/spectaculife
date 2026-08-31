use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
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

        // A duplicate free used to put the same slot into free_list twice, after
        // which two live Stem cells could receive the same genome slot. Keep the
        // pool fail-safe even if a caller makes that mistake again.
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

impl Distribution<MutationRate> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> MutationRate {
        MutationRate(rng.gen_range(1..=12))
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
        LifeSpan(rng.gen_range(50..=1000))
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

    pub const fn get_gene(&self, loc: GeneLocation) -> Gene {
        self.genes[loc.0 as usize]
    }

    pub fn mutate(&mut self) {
        let mut rng = thread_rng();
        let rate = self.mutation_rate.0.clamp(1, 100) as u32;

        // Mutation happens only when a new seed is created. A successful event
        // changes one (occasionally two) loci instead of repeatedly randomising
        // large parts of the genome.
        if !rng.gen_ratio(rate, 100) {
            return;
        }

        let edits = if rng.gen_ratio(rate.min(20), 100) { 2 } else { 1 };
        for _ in 0..edits {
            self.mutate_one(&mut rng);
        }

        // Let mutation rate itself evolve, but only gradually.
        if rng.gen_ratio(1, 10) {
            if rng.gen_bool(0.5) {
                self.mutation_rate.0 = self.mutation_rate.0.saturating_add(1).min(25);
            } else {
                self.mutation_rate.0 = self.mutation_rate.0.saturating_sub(1).max(1);
            }
        }
    }

    fn mutate_one<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        let mutation = rng.gen_range(0..=22);
        if mutation == 22 {
            self.seed_gene = rng.gen();
            return;
        }

        let gene_idx = match rng.gen_range(0..4) {
            0 => self.seed_gene.0 as usize,
            1 => self.active_gene.0 as usize,
            _ => rng.gen_range(0..MAX_GENES as usize),
        };
        let gene = &mut self.genes[gene_idx];

        match mutation {
            0 => gene.up = rng.gen(),
            1 => gene.down = rng.gen(),
            2 => gene.left = rng.gen(),
            3 => gene.right = rng.gen(),

            4 => {
                gene.condition_1 = rng.gen();
                gene.param_1 = gene.condition_1.random_param(rng);
            }
            5 => gene.param_1 = gene.condition_1.random_param(rng),
            6 => {
                gene.condition_2 = rng.gen();
                gene.param_2 = gene.condition_2.random_param(rng);
            }
            7 => gene.param_2 = gene.condition_2.random_param(rng),

            8 => gene.alt_gene1 = rng.gen(),
            9 => gene.alt_gene2 = rng.gen(),
            10 => gene.alt_gene3 = rng.gen(),

            11 => {
                gene.additional_action_condition1 = rng.gen();
                gene.additional_action_param1 =
                    gene.additional_action_condition1.random_param(rng);
            }
            12 => {
                gene.additional_action_param1 =
                    gene.additional_action_condition1.random_param(rng);
            }
            13 => {
                gene.additional_action_condition2 = rng.gen();
                gene.additional_action_param2 =
                    gene.additional_action_condition2.random_param(rng);
            }
            14 => {
                gene.additional_action_param2 =
                    gene.additional_action_condition2.random_param(rng);
            }

            15 => gene.additional_action1 = rng.gen(),
            16 => gene.additional_action2 = rng.gen(),
            17 => gene.additional_action3 = rng.gen(),

            18 => {
                gene.main_action_condition = rng.gen();
                gene.main_action_param = gene.main_action_condition.random_param(rng);
            }
            19 => gene.main_action_param = gene.main_action_condition.random_param(rng),
            20 => gene.main_action = rng.gen(),
            21 => gene.self_lifespan = rng.gen(),
            _ => unreachable!(),
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
        let main_action_condition: GeneCondition = rng.gen();
        let additional_action_condition1: GeneCondition = rng.gen();
        let additional_action_condition2: GeneCondition = rng.gen();
        let condition_1: GeneCondition = rng.gen();
        let condition_2: GeneCondition = rng.gen();

        Gene {
            up: rng.gen(),
            down: rng.gen(),
            left: rng.gen(),
            right: rng.gen(),

            main_action_condition,
            main_action_param: main_action_condition.random_param(rng),
            main_action: rng.gen(),

            additional_action_condition1,
            additional_action_param1: additional_action_condition1.random_param(rng),

            additional_action_condition2,
            additional_action_param2: additional_action_condition2.random_param(rng),

            additional_action1: rng.gen(),
            additional_action2: rng.gen(),
            additional_action3: rng.gen(),

            condition_1,
            param_1: condition_1.random_param(rng),

            condition_2,
            param_2: condition_2.random_param(rng),

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
            MultiplySelf(_, _) => 0.8,
            CreateSeed(_) => 0.8,
            MakeFilter(_) => 0.6,
            Nothing => 0.,
            KillCell => 0.,
        }
    }
}

impl Distribution<GeneDirectionAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneDirectionAction {
        // A random genome must be supercritical often enough for selection to
        // have something larger than a 2-3-cell dead end to work with.
        match rng.gen_range(0..16) {
            0..=5 => GeneDirectionAction::MultiplySelf(rng.gen(), rng.gen()),
            6 => GeneDirectionAction::MakeLeaf(rng.gen()),
            7 => GeneDirectionAction::MakeRoot(rng.gen()),
            8 => GeneDirectionAction::MakeReactor(rng.gen()),
            9 => GeneDirectionAction::MakeFilter(rng.gen()),
            10 => GeneDirectionAction::CreateSeed(rng.gen()),
            11..=14 => GeneDirectionAction::Nothing,
            _ => GeneDirectionAction::KillCell,
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
    fn random_param<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        use GeneCondition::*;

        match self {
            LifeUp | LifeDown | LifeLeft | LifeRight
            | LethalOrganicUp | LethalOrganicDown | LethalOrganicLeft | LethalOrganicRight
            | LethalEnergyUp | LethalEnergyDown | LethalEnergyLeft | LethalEnergyRight
            | Always | Never => 0,
            RandomMT => rng.gen(),
            LifeEnergyMT => rng.gen_range(0..=64),
            OrganicCenterMT | OrganicUpMT | OrganicDownMT | OrganicLeftMT | OrganicRightMT => {
                rng.gen_range(0..=16)
            }
            SoilEnergyCenterMT | SoilEnergyUpMT | SoilEnergyDownMT | SoilEnergyLeftMT
            | SoilEnergyRightMT => rng.gen_range(0..=32),
            AirPollutionCenterMT | AirPollutionUpMT | AirPollutionDownMT | AirPollutionLeftMT
            | AirPollutionRightMT => rng.gen_range(0..=64),
            StepsDividesP => rng.gen_range(1..=64),
        }
    }
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

    WaitStep,
    Die,
}

impl Distribution<GeneAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneAction {
        use GeneAction::*;
        match rng.gen_range(0..=15) {
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
            13 => KillDownRight,

            14 => WaitStep,

            _ => Die,
        }
    }
}
