use bevy::prelude::Resource;

use crate::utils::get_continual_coord;

#[derive(Debug, Clone, Resource, Default)]
pub struct Grid<T> {
    data: Vec<T>,
    pub width: u32,
    pub height: u32,
}

impl<T: std::default::Default + std::clone::Clone> Grid<T> {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![T::default(); (width as usize) * (height as usize)],
            width,
            height,
        }
    }

    #[inline(always)]
    fn idx(&self, x: u32, y: u32) -> usize {
        y as usize * self.width as usize + x as usize
    }

    pub fn get<'a>(&'a self, x: i64, y: i64) -> &'a T {
        let wx = get_continual_coord(x, self.width);
        let wy = get_continual_coord(y, self.height);
        let i = self.idx(wx, wy);
        &self.data[i]
    }

    pub fn uget<'a>(&'a self, x: u32, y: u32) -> &'a T {
        let i = self.idx(x, y);
        &self.data[i]
    }

    pub fn get_mut<'a>(&'a mut self, x: i64, y: i64) -> &'a mut T {
        let wx = get_continual_coord(x, self.width);
        let wy = get_continual_coord(y, self.height);
        let i = self.idx(wx, wy);
        &mut self.data[i]
    }

    pub(crate) fn uget_mut<'a>(&'a mut self, x: u32, y: u32) -> &'a mut T {
        let i = self.idx(x, y);
        &mut self.data[i]
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

impl<T> Grid<T> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    pub fn cells(&self) -> &[T] {
        &self.data
    }

    #[inline(always)]
    pub fn cells_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    #[inline(always)]
    pub fn coords(&self, index: usize) -> (u32, u32) {
        let width = self.width as usize;
        ((index % width) as u32, (index / width) as u32)
    }

    #[inline(always)]
    pub fn wrapped_index(&self, x: i64, y: i64) -> usize {
        let wx = get_continual_coord(x, self.width);
        let wy = get_continual_coord(y, self.height);
        wy as usize * self.width as usize + wx as usize
    }

    #[inline(always)]
    pub fn offset_index(&self, index: usize, dx: i64, dy: i64) -> usize {
        let (x, y) = self.coords(index);
        self.wrapped_index(x as i64 + dx, y as i64 + dy)
    }
}
