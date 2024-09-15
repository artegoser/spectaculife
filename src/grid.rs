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

    pub fn get_mut<'a>(&'a mut self, x: i64, y: i64) -> &'a mut T {
        self.grid
            .get_mut(get_continual_coord(y as i64, self.height) as usize)
            .unwrap()
            .get_mut(get_continual_coord(x as i64, self.width) as usize)
            .unwrap()
    }

    pub fn set(&mut self, x: i64, y: i64, item: T) {
        let cell = self.get_mut(x, y);
        *cell = item;
    }
}
