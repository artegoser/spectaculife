use super::{BirthDirective, LifeType};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

pub const MAX_GENES: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Genome {
    pub active_gene: u8,
    pub genes: [Gene; MAX_GENES as usize],
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

    pub next_gene: u8,
}

impl Gene {
    pub fn to_birth_directive(&self, genome: Genome) -> BirthDirective {
        BirthDirective::new(
            self.up.to_life_type(genome),
            self.down.to_life_type(genome),
            self.left.to_life_type(genome),
            self.right.to_life_type(genome),
        )
    }
}

impl Distribution<Gene> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Gene {
        Gene {
            up: rand::random(),
            down: rand::random(),
            left: rand::random(),
            right: rand::random(),
            next_gene: rng.gen_range(0..MAX_GENES),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneAction {
    MakeLeaf,
    MakeRoot,
    MakeReactor,
    MultiplySelf(u8),
    Nothing,
}

impl GeneAction {
    pub fn to_life_type(&self, mut genome: Genome) -> Option<LifeType> {
        match self {
            GeneAction::MakeLeaf => Some(LifeType::Leaf),
            GeneAction::MakeRoot => Some(LifeType::Root),
            GeneAction::MultiplySelf(next_gene) => {
                genome.active_gene = *next_gene;

                Some(LifeType::Stem(genome))
            }
            GeneAction::Nothing => None,
            GeneAction::MakeReactor => Some(LifeType::Reactor),
        }
    }
}

impl Distribution<GeneAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GeneAction {
        match rng.gen_range(0..=100) {
            0..=15 => GeneAction::MultiplySelf(rng.gen_range(0..MAX_GENES)),
            16..=25 => GeneAction::MakeLeaf,
            36..=45 => GeneAction::MakeRoot,
            46..=50 => GeneAction::MakeReactor,
            _ => GeneAction::Nothing,
        }
    }
}
