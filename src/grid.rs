use bevy::prelude::Resource;

use crate::utils::get_continual_coord;

#[derive(Debug, Clone, Resource)]
pub struct Grid<T> {
    grid: Vec<Vec<T>>,
    width: u32,
    height: u32,
}

impl<T: std::default::Default + std::clone::Clone> Grid<T> {
    pub fn new(width: u32, height: u32) -> Self {
        let grid: Vec<Vec<T>> = vec![vec![T::default(); width as usize]; height as usize];

        Self {
            grid,
            width,
            height,
        }
    }

    pub fn get<'a>(&'a self, x: i64, y: i64) -> &'a T {
        self.grid
            .get(get_continual_coord(y, self.height) as usize)
            .unwrap()
            .get(get_continual_coord(x, self.width) as usize)
            .unwrap()
    }

    pub fn uget<'a>(&'a self, x: u32, y: u32) -> &'a T {
        self.grid.get(y as usize).unwrap().get(x as usize).unwrap()
    }

    pub fn get_mut<'a>(&'a mut self, x: i64, y: i64) -> &'a mut T {
        self.grid
            .get_mut(get_continual_coord(y, self.height) as usize)
            .unwrap()
            .get_mut(get_continual_coord(x, self.width) as usize)
            .unwrap()
    }

    fn uget_mut<'a>(&'a mut self, x: u32, y: u32) -> &'a mut T {
        self.grid
            .get_mut(y as usize)
            .unwrap()
            .get_mut(x as usize)
            .unwrap()
    }

    pub fn set(&mut self, x: i64, y: i64, item: T) {
        let cell = self.get_mut(x, y);
        *cell = item;
    }

    pub fn uset(&mut self, x: u32, y: u32, item: T) {
        let cell = self.uget_mut(x, y);
        *cell = item;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Area<T> {
    pub up: T,
    pub left: T,
    pub center: T,
    pub right: T,
    pub down: T,
}

impl<T: std::default::Default + std::clone::Clone + std::marker::Copy> Area<T> {
    pub fn new(grid: &Grid<T>, x: u32, y: u32) -> Self {
        Self {
            up: *grid.get(x as i64, y as i64 - 1),
            left: *grid.get(x as i64 - 1, y as i64),
            center: *grid.uget(x, y),
            right: *grid.get(x as i64 + 1, y as i64),
            down: *grid.get(x as i64, y as i64 + 1),
        }
    }
}
