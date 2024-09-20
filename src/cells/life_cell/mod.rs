use genome::Genome;

use crate::{
    grid::Area,
    types::CellDir::{self, *},
    utils::merge_energy,
};

use super::WorldCell;

mod genome;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LifeCell {
    Alive(AliveCell),

    #[default]
    Dead,
}

impl LifeCell {
    pub const fn texture_id(&self, area: &Area<WorldCell>) -> u32 {
        match self {
            Self::Alive(alive_life_cell) => alive_life_cell.texture_id(area),
            Self::Dead => 16,
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

    pub const fn is_pipe(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.is_pipe(),
            Self::Dead => false,
        }
    }

    pub const fn is_energy_generator(&self) -> bool {
        match self {
            Self::Alive(alive_cell) => alive_cell.is_energy_generator(),
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

    pub parent_dir: Option<CellDir>,
}

impl AliveCell {
    pub fn new(
        ty: LifeType,
        energy: f32,
        energy_to: EnergyDirections,
        parent_dir: Option<CellDir>,
    ) -> Self {
        Self {
            ty,
            energy,

            energy_to,

            parent_dir,
        }
    }

    pub const fn texture_id(&self, area: &Area<WorldCell>) -> u32 {
        match self.ty {
            LifeType::Pipe => match merge_energy(&area, self.energy_to).to_tuple() {
                (false, false, false, false) => 0,
                (true, true, false, false) => 1,
                (false, false, true, true) => 2,
                (true, true, true, true) => 3,
                (false, true, true, false) => 4,
                (true, false, true, false) => 5,
                (true, false, false, true) => 6,
                (false, true, false, true) => 7,
                (false, true, true, true) => 8,
                (true, true, true, false) => 9,
                (true, false, true, true) => 10,
                (true, true, false, true) => 11,
                (false, true, false, false) => 12,
                (false, false, true, false) => 13,
                (true, false, false, false) => 14,
                (false, false, false, true) => 15,
            },
            LifeType::Leaf => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 23,
                        Down => 24,
                        Left => 25,
                        Right => 26,
                    }
                } else {
                    22
                }
            }
            LifeType::Cancer => 42,
            LifeType::Stem(_) => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 18,
                        Down => 19,
                        Left => 20,
                        Right => 21,
                    }
                } else {
                    17
                }
            }
            LifeType::Root => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 28,
                        Down => 29,
                        Left => 30,
                        Right => 31,
                    }
                } else {
                    27
                }
            }
            LifeType::Reactor => {
                if let Some(dir) = self.parent_dir {
                    match dir {
                        Up => 33,
                        Down => 34,
                        Left => 35,
                        Right => 36,
                    }
                } else {
                    32
                }
            }
        }
    }

    pub const fn can_transfer(&self) -> bool {
        self.ty.can_transfer()
    }

    pub const fn is_pipe_recipient(&self) -> bool {
        self.ty.is_pipe_recipient()
    }

    pub const fn is_pipe(&self) -> bool {
        self.ty.is_pipe()
    }

    pub const fn is_energy_generator(&self) -> bool {
        self.ty.is_energy_generator()
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
    Reactor,

    Stem(Genome),
    Cancer,
}

impl LifeType {
    pub const fn can_transfer(&self) -> bool {
        self.is_energy_generator() || self.is_pipe()
    }

    pub const fn is_energy_generator(&self) -> bool {
        match self {
            LifeType::Leaf | LifeType::Root | LifeType::Reactor => true,
            _ => false,
        }
    }

    pub const fn is_pipe(&self) -> bool {
        match self {
            LifeType::Pipe | LifeType::Cancer => true,
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
            LifeType::Root => 0.5,
            LifeType::Reactor => 0.7,
        }
    }

    pub const fn organics(&self) -> u8 {
        match self {
            LifeType::Pipe => 1,
            LifeType::Leaf => 2,
            LifeType::Cancer => 3,
            LifeType::Stem(_) => 1,
            LifeType::Root => 2,
            LifeType::Reactor => 1,
        }
    }

    pub fn birth_capacity(&self) -> f32 {
        2. * self.consumption()
    }

    pub fn make_newborn_cell(self, parent_dir: CellDir) -> LifeCell {
        let new_cell_energy_directions = if self.is_energy_generator() {
            EnergyDirections::from_direction(&parent_dir)
        } else {
            EnergyDirections::default()
        };

        let new_cell_energy = self.birth_capacity() * 0.8;

        LifeCell::Alive(AliveCell::new(
            self,
            new_cell_energy,
            new_cell_energy_directions,
            Some(parent_dir),
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
            Up => EnergyDirections {
                up: true,
                down: false,
                left: false,
                right: false,
            },
            Down => EnergyDirections {
                up: false,
                down: true,
                left: false,
                right: false,
            },
            Left => EnergyDirections {
                up: false,
                down: false,
                left: true,
                right: false,
            },
            Right => EnergyDirections {
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
