use bevy::prelude::Resource;

use crate::{
    types::{CellDir, Coord, Settings},
    utils::get_continual_coord,
};

#[derive(Debug, Clone, Resource, Default)]
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

#[derive(Debug, PartialEq)]
pub struct Area<'a, T> {
    pub up: &'a mut T,
    pub down: &'a mut T,
    pub left: &'a mut T,
    pub right: &'a mut T,

    pub up_left: &'a mut T,
    pub up_right: &'a mut T,

    pub down_left: &'a mut T,
    pub down_right: &'a mut T,

    pub center: &'a mut T,

    pub x: u32,
    pub y: u32,
}

impl<'a, T: std::default::Default + std::clone::Clone + std::marker::Copy> Area<'a, T> {
    pub fn new(grid: *mut Grid<T>, x: u32, y: u32) -> Self {
        unsafe {
            Self {
                up: (&mut *grid).get_mut(x as i64, y as i64 - 1),
                left: (&mut *grid).get_mut(x as i64 - 1, y as i64),
                center: (&mut *grid).uget_mut(x, y),
                right: (&mut *grid).get_mut(x as i64 + 1, y as i64),
                down: (&mut *grid).get_mut(x as i64, y as i64 + 1),

                up_left: (&mut *grid).get_mut(x as i64 - 1, y as i64 - 1),
                up_right: (&mut *grid).get_mut(x as i64 + 1, y as i64 - 1),

                down_left: (&mut *grid).get_mut(x as i64 - 1, y as i64 + 1),
                down_right: (&mut *grid).get_mut(x as i64 + 1, y as i64 + 1),

                x,
                y,
            }
        }
    }

    pub fn get_up_coord(&self, settings: &Settings) -> Coord {
        Coord {
            x: self.x,
            y: get_continual_coord(self.y as i64 - 1, settings.h),
        }
    }

    pub fn get_down_coord(&self, settings: &Settings) -> Coord {
        Coord {
            x: self.x,
            y: get_continual_coord(self.y as i64 + 1, settings.h),
        }
    }

    pub fn get_left_coord(&self, settings: &Settings) -> Coord {
        Coord {
            x: get_continual_coord(self.x as i64 - 1, settings.w),
            y: self.y,
        }
    }

    pub fn get_right_coord(&self, settings: &Settings) -> Coord {
        Coord {
            x: get_continual_coord(self.x as i64 + 1, settings.w),
            y: self.y,
        }
    }

    pub fn get_center_coord(&self, _: &Settings) -> Coord {
        Coord {
            x: self.x,
            y: self.y,
        }
    }

    pub fn coord_from_dir(&self, dir: &CellDir, settings: &Settings) -> Coord {
        match dir {
            CellDir::Up => self.get_up_coord(settings),
            CellDir::Down => self.get_down_coord(settings),
            CellDir::Left => self.get_left_coord(settings),
            CellDir::Right => self.get_right_coord(settings),
        }
    }
}
