use std::default;

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

#[derive(Debug, Clone, Copy, Resource)]
pub struct Settings {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Resource)]
pub struct State {
    pub cursor_position: Coord,
    pub paused: bool,
    pub restart: bool,
    pub cell_order_x: Vec<u32>,
    pub cell_order_y: Vec<u32>,
}

impl State {
    pub fn from_settings(settings: &Settings) -> Self {
        let mut rng = rand::thread_rng();

        let mut cell_order_x: Vec<u32> = (0..settings.w).collect();
        cell_order_x.shuffle(&mut rng);

        let mut cell_order_y: Vec<u32> = (0..settings.h).collect();
        cell_order_y.shuffle(&mut rng);

        State {
            restart: true,
            paused: false,
            cursor_position: Coord::default(),

            cell_order_x,
            cell_order_y,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            restart: true,
            paused: false,
            cursor_position: Coord::default(),

            cell_order_x: vec![],
            cell_order_y: vec![],
        }
    }
}
