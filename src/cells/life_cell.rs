use crate::types::CellDir;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LifeCell {
    Alive(AliveLifeCell),

    #[default]
    Dead,
}

impl LifeCell {
    pub const fn texture_id(&self) -> u32 {
        match self {
            Self::Alive(alive_life_cell) => alive_life_cell.texture_id(),
            Self::Dead => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AliveLifeCell {
    pub ty: LifeCellType,

    pub energy: f32,

    pub energy_to: (bool, bool, bool, bool),
    pub parent: Option<CellDir>,
}

impl AliveLifeCell {
    pub fn new(cell: LifeCellType, energy: f32, parent: Option<CellDir>) -> Self {
        Self {
            ty: cell,
            energy,

            energy_to: (false, false, false, false),
            parent,
        }
    }

    pub const fn texture_id(&self) -> u32 {
        self.ty.texture_id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeCellType {
    // Pipe(PipeCell),
    Cancer,
}

impl LifeCellType {
    pub const fn texture_id(&self) -> u32 {
        match self {
            LifeCellType::Cancer => 7,
        }
    }

    pub const fn consumption(&self) -> f32 {
        match self {
            LifeCellType::Cancer => 1.,
        }
    }
}
