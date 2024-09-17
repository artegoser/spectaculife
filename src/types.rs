use bevy::prelude::Resource;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellDir {
    Up,
    Down,
    Left,
    Right,
}

impl CellDir {
    pub const fn opposite(&self) -> CellDir {
        match self {
            CellDir::Up => CellDir::Down,
            CellDir::Down => CellDir::Up,
            CellDir::Left => CellDir::Right,
            CellDir::Right => CellDir::Left,
        }
    }
}

impl Distribution<CellDir> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CellDir {
        match rng.gen_range(0..=4) {
            0 => CellDir::Up,
            1 => CellDir::Down,
            2 => CellDir::Left,
            _ => CellDir::Right,
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
