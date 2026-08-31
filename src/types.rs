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
    pub const ALL: [CellDir; 4] = [CellDir::Up, CellDir::Down, CellDir::Left, CellDir::Right];

    pub const fn opposite(&self) -> CellDir {
        match self {
            CellDir::Up => CellDir::Down,
            CellDir::Down => CellDir::Up,
            CellDir::Left => CellDir::Right,
            CellDir::Right => CellDir::Left,
        }
    }

    pub const fn offset(&self) -> (i64, i64) {
        match self {
            CellDir::Up => (0, -1),
            CellDir::Down => (0, 1),
            CellDir::Left => (-1, 0),
            CellDir::Right => (1, 0),
        }
    }
}

impl Distribution<CellDir> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CellDir {
        match rng.gen_range(0..4) {
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
    pub soil_energy_visible: bool,
    pub energy_directions_visible: bool,
    pub hud_visible: bool,

    pub simulation_step: usize,

    pub next_organism_id: u64,
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
            soil_energy_visible: true,
            energy_directions_visible: true,
            hud_visible: true,

            simulation_step: 0,

            next_organism_id: 1,
        }
    }
}

impl State {
    pub fn allocate_organism_id(&mut self) -> u64 {
        let id = self.next_organism_id.max(1);
        self.next_organism_id = id.saturating_add(1);
        id
    }
}
