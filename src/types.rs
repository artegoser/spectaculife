use bevy::prelude::Resource;

#[derive(Debug, Clone)]
pub enum CellDir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Resource)]
pub struct Settings {
    pub w: u32,
    pub h: u32,
}
