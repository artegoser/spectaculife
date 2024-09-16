use bevy::prelude::Resource;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellDir {
    Up,
    Down,
    Left,
    Right,
}

impl CellDir {
    pub fn opposite(&self) -> CellDir {
        match self {
            CellDir::Up => CellDir::Down,
            CellDir::Down => CellDir::Up,
            CellDir::Left => CellDir::Right,
            CellDir::Right => CellDir::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Copy, Resource)]
pub struct Settings {
    pub w: u32,
    pub h: u32,
}
