use bevy::prelude::Resource;
use rand::{
    distributions::{Distribution, Standard},
    seq::SliceRandom,
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Coord {
    pub x: u32,
    pub y: u32,
}

impl Coord {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, Resource)]
pub struct Settings {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Resource)]
pub struct State {
    pub cursor_position: Coord,
    pub paused: bool,
    pub initialized: bool,

    pub organic_visible: bool,
    pub life_visible: bool,
    pub pollution_visible: bool,
}

impl State {
    pub fn from_settings(settings: &Settings) -> Self {
        State {
            initialized: false,
            paused: false,
            cursor_position: Coord::default(),

            organic_visible: true,
            life_visible: true,
            pollution_visible: true,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            initialized: false,
            paused: false,
            cursor_position: Coord::default(),

            organic_visible: true,
            life_visible: true,
            pollution_visible: true,
        }
    }
}
