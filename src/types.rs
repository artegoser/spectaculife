use bevy::prelude::Resource;

#[derive(Debug, Clone, Copy)]
pub enum CellDir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Resource)]
pub struct Settings {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy, Resource)]
pub struct World {
    life:
}
