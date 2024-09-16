use crate::types::CellDir;

#[derive(Debug, Clone, Copy)]
pub struct LifeCell {
    pub ty: LifeCellType,

    pub energy: f32,

    pub energy_to: (bool, bool, bool, bool),
    pub parent: Option<CellDir>,
}

impl LifeCell {
    pub fn new(cell: LifeCellType, energy: f32, parent: Option<CellDir>) -> Self {
        Self {
            ty: cell,
            energy,

            energy_to: (false, false, false, false),
            parent,
        }
    }

    pub fn texture_id(&self) -> u32 {
        self.ty.texture_id()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LifeCellType {
    // Pipe(PipeCell),
    Cancer,
}

impl LifeCellType {
    pub const fn texture_id(&self) -> u32 {
        match self {
            LifeCellType::Cancer => 6,
        }
    }

    pub const fn consumption(&self) -> f32 {
        match self {
            LifeCellType::Cancer => 1.,
        }
    }
}
