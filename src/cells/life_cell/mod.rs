use genome::Genome;

use crate::{grid::Area, types::CellDir, utils::merge_energy};

use super::WorldCell;

mod genome;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LifeCell {
    Alive(AliveCell),

    #[default]
    Dead,
}

impl LifeCell {
    pub const fn texture_id(&self, area: Area<WorldCell>) -> u32 {
        match self {
            Self::Alive(alive_life_cell) => alive_life_cell.texture_id(area),
            Self::Dead => 0,
        }
    }

    pub const fn is_fertile(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.is_fertile(),
            Self::Dead => false,
        }
    }

    pub const fn can_transfer(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.can_transfer(),
            Self::Dead => false,
        }
    }

    pub const fn is_pipe_recipient(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.is_pipe_recipient(),
            Self::Dead => false,
        }
    }

    pub const fn is_alive(&self) -> bool {
        match self {
            Self::Alive(_) => false,
            Self::Dead => false,
        }
    }

    pub const fn energy(&self) -> f32 {
        match self {
            Self::Alive(alive_cell) => alive_cell.energy,
            Self::Dead => 0.,
        }
    }

    pub const fn organics(&self) -> u8 {
        match self {
            Self::Alive(alive_cell) => alive_cell.organics(),
            Self::Dead => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AliveCell {
    pub ty: LifeType,

    pub energy: f32,

    pub energy_to: EnergyDirections,
    pub parent: Option<CellDir>,
}

impl AliveCell {
    pub fn new(
        ty: LifeType,
        energy: f32,
        parent: Option<CellDir>,
        energy_to: EnergyDirections,
    ) -> Self {
        Self {
            ty,
            energy,

            energy_to,
            parent,
        }
    }

    pub const fn texture_id(&self, area: Area<WorldCell>) -> u32 {
        match self.ty {
            LifeType::Pipe => match merge_energy(&area, self.energy_to).to_tuple() {
                (false, false, false, false) => 7,
                (true, true, false, false) => 8,
                (false, false, true, true) => 9,
                (true, true, true, true) => 10,
                (false, true, true, false) => 11,
                (true, false, true, false) => 12,
                (true, false, false, true) => 13,
                (false, true, false, true) => 14,
                (false, true, true, true) => 15,
                (true, true, true, false) => 16,
                (true, false, true, true) => 17,
                (true, true, false, true) => 18,
                (false, true, false, false) => 19,
                (false, false, true, false) => 20,
                (true, false, false, false) => 21,
                (false, false, false, true) => 22,
            },
            LifeType::Leaf => 2,
            LifeType::Cancer => 6,
            LifeType::Stem(_) => 1,
            LifeType::Root => 3,
        }
    }

    pub const fn can_transfer(&self) -> bool {
        self.ty.can_transfer()
    }

    pub const fn is_pipe_recipient(&self) -> bool {
        self.ty.is_pipe_recipient()
    }

    pub const fn is_fertile(&self) -> bool {
        self.ty.is_fertile()
    }

    pub const fn consumption(&self) -> f32 {
        self.ty.consumption()
    }

    pub const fn organics(&self) -> u8 {
        self.ty.organics()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeType {
    Pipe,
    Leaf,
    Root,

    Stem(Genome),
    Cancer,
}

impl LifeType {
    pub const fn can_transfer(&self) -> bool {
        match self {
            LifeType::Pipe | LifeType::Cancer | LifeType::Leaf | LifeType::Root => true,
            _ => false,
        }
    }

    pub const fn is_pipe_recipient(&self) -> bool {
        match self {
            LifeType::Pipe | LifeType::Cancer | LifeType::Stem(_) => true,
            _ => false,
        }
    }

    pub const fn is_fertile(&self) -> bool {
        match self {
            LifeType::Stem(_) | LifeType::Cancer => true,
            _ => false,
        }
    }

    pub const fn consumption(&self) -> f32 {
        match self {
            LifeType::Pipe => 0.1,
            LifeType::Leaf => 0.5,
            LifeType::Cancer => 1.,
            LifeType::Stem(_) => 0.1,
            LifeType::Root => 0.3,
        }
    }

    pub const fn organics(&self) -> u8 {
        match self {
            LifeType::Pipe => 1,
            LifeType::Leaf => 2,
            LifeType::Cancer => 3,
            LifeType::Stem(_) => 1,
            LifeType::Root => 2,
        }
    }

    pub fn birth_capacity(&self) -> f32 {
        2. * self.consumption()
    }

    pub fn make_newborn_cell(self, parent_dir: CellDir, energy: f32) -> LifeCell {
        let new_cell_energy_directions = match self {
            Self::Leaf | Self::Root => EnergyDirections::from_direction(&parent_dir),
            _ => EnergyDirections::default(),
        };

        let new_cell_energy = self.birth_capacity() * 0.8 + energy * 0.5;

        LifeCell::Alive(AliveCell::new(
            self,
            new_cell_energy,
            Some(parent_dir),
            new_cell_energy_directions,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnergyDirections {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl EnergyDirections {
    pub const fn from_direction(dir: &CellDir) -> Self {
        match dir {
            CellDir::Up => EnergyDirections {
                up: true,
                down: false,
                left: false,
                right: false,
            },
            CellDir::Down => EnergyDirections {
                up: false,
                down: true,
                left: false,
                right: false,
            },
            CellDir::Left => EnergyDirections {
                up: false,
                down: false,
                left: true,
                right: false,
            },
            CellDir::Right => EnergyDirections {
                up: false,
                down: false,
                left: false,
                right: true,
            },
        }
    }

    pub const fn branches_amount(&self) -> u8 {
        let mut total = 0;

        if self.up {
            total += 1
        };

        if self.down {
            total += 1
        };

        if self.left {
            total += 1
        };

        if self.right {
            total += 1
        };

        total
    }

    pub const fn to_tuple(&self) -> (bool, bool, bool, bool) {
        (self.up, self.down, self.left, self.right)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BirthDirective {
    pub up: Option<LifeType>,
    pub down: Option<LifeType>,
    pub left: Option<LifeType>,
    pub right: Option<LifeType>,
}

impl BirthDirective {
    pub fn new(
        up: Option<LifeType>,
        down: Option<LifeType>,
        left: Option<LifeType>,
        right: Option<LifeType>,
    ) -> Self {
        Self {
            up,
            down,
            left,
            right,
        }
    }

    pub fn energy_capacity(&self) -> f32 {
        let mut total = 0.;

        if let Some(life_type) = self.up {
            total += life_type.birth_capacity()
        }

        if let Some(life_type) = self.down {
            total += life_type.birth_capacity()
        }

        if let Some(life_type) = self.left {
            total += life_type.birth_capacity()
        }

        if let Some(life_type) = self.right {
            total += life_type.birth_capacity()
        }

        total
    }

    // pub const fn cell_amount(&self) -> u8 {
    //     let mut total = 0;

    //     if self.up.is_some() {
    //         total += 1
    //     };

    //     if self.down.is_some() {
    //         total += 1
    //     };

    //     if self.left.is_some() {
    //         total += 1
    //     };

    //     if self.right.is_some() {
    //         total += 1
    //     };

    //     total
    // }
}
