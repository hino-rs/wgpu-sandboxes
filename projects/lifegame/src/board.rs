// const NEIGHBORS: [(isize, isize); 8] = [
//     (-1, -1), (0, -1), (1, -1),
//     (-1,  0),          (1,  0),
//     (-1,  1), (0,  1), (1,  1),
// ];

use std::collections::VecDeque;

use crate::utils::ratio_to_u8;
use crate::{shape::INITIAL_GAP_SIZE, state::State};

const INITIAL_RANDOM_RATIO: f64 = 0.25;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Cell {
    Alive,
    Dead,
}

#[derive(Clone)]
pub struct Board {
    pub num_grid_per_row: usize,
    pub grid_size: usize,
    pub current: Vec<Cell>,
    pub next: Vec<Cell>,
    pub tick_count: u64,

    pub delay: u64,
    pub pause: bool,
    pub next_tick: bool,
    pub cell_colors: Colors,
    pub random_ratio: f64,
    pub alive_dead_count: (u64, u64),
    pub record: VecDeque<Record>,
    pub bg_color: [f32; 3],
    pub gap_size: f32,
}

#[derive(Clone)]
pub struct Record {
    pub alive_count: u64,
    pub dead_count: u64,
}

#[derive(Clone, Copy)]
pub struct Colors(pub Color, pub Color);

#[derive(Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Board {
    pub fn alive_rgb_u8(&self) -> (u8, u8, u8) {
        (
            ratio_to_u8(self.cell_colors.0.r),
            ratio_to_u8(self.cell_colors.0.g),
            ratio_to_u8(self.cell_colors.0.b),
        )
    }

    pub fn dead_rgb_u8(&self) -> (u8, u8, u8) {
        (
            ratio_to_u8(self.cell_colors.1.r),
            ratio_to_u8(self.cell_colors.1.g),
            ratio_to_u8(self.cell_colors.1.b),
        )
    }

    pub fn reshuffle(&mut self) {
        let mut new_board = Self::empty_board(self.current.len());

        for c in &mut new_board {
            if rand::random_bool(self.random_ratio) {
                *c = Cell::Alive;
            }
        }

        self.current = new_board;
    }

    pub fn shrink(&mut self, state: &mut State, n: u8) {
        for _ in 0..n {
            let current_width = self.num_grid_per_row;
            
            if current_width <= 1 || self.current.is_empty() {
                return;
            }

            let new_width = current_width - 1;
            let current_height = (self.current.len() + current_width - 1) / current_width;
            let new_height = if current_height > 1 { current_height - 1 } else { 0 };
            let mut result = Vec::with_capacity(new_width * new_height);

            for i in 0..self.current.len() {
                let row = i / current_width;
                let col = i % current_width;
                
                if col < new_width && row < new_height {
                    result.push(self.current[i]);
                }
            }

            self.num_grid_per_row -= 1;
            self.current = result;
            self.next = self.current.clone();
            self.grid_size = self.current.len();

            state.update_instance_buffer(self.current.len());
            state.update_instances(&self.current, self.num_grid_per_row, self.gap_size, self.cell_colors);
        }
    }

    pub fn expand(&mut self, state: &mut State, n: u8) {
        for _ in 0..n {
            let current_width = self.num_grid_per_row;

            if current_width == 0 || self.current.is_empty() {
                return;
            }

            let current_height = (self.current.len() + current_width - 1) / current_width;

            let new_width = current_width + 1;
            let new_height = current_height + 1;

            let mut result = vec![Cell::Dead; new_width * new_height];

            for i in 0..self.current.len() {
                let row = i / current_width;
                let col = i % current_width;

                let new_index = row * new_width + col;
                result[new_index] = self.current[i];
            }

            self.num_grid_per_row += 1;
            self.current = result;
            self.next = self.current.clone();
            self.grid_size = self.current.len();

            state.update_instance_buffer(self.current.len());
            state.update_instances(&self.current, self.num_grid_per_row, self.gap_size, self.cell_colors);
        }
    }

    pub fn cells(&self) -> &[Cell] {
        &self.current
    }

    pub fn clear(&mut self) {
        let grid_size = self.current.len();

        self.current = Self::empty_board(grid_size);
        self.next = Self::empty_board(grid_size);
    }

    pub fn randomly_make_alive(&mut self) {
        for c in &mut self.current {
            if rand::random_bool(self.random_ratio) {
                *c = Cell::Alive;
            }
        }
    }

    pub fn randomly_make_dead(&mut self) {
        for c in &mut self.current {
            if rand::random_bool(self.random_ratio) {
                *c = Cell::Dead;
            }
        }
    }



    pub fn new(num_grid_per_row: usize) -> Self {
        let grid_size = num_grid_per_row * num_grid_per_row;

        let mut current = Self::empty_board(grid_size);

        let mut alive_count = 0;
        for c in &mut current {
            if rand::random_bool(INITIAL_RANDOM_RATIO) {
                *c = Cell::Alive;
                alive_count += 1;
            }
        }
        let dead_count = current.len() as u64 - alive_count;

        let mut record = VecDeque::with_capacity(100);
        record.push_back(Record { alive_count, dead_count });

        Self {
            tick_count: 0,
            num_grid_per_row,
            grid_size,
            next: current.clone(),
            current,
            delay: 1,
            pause: false,
            next_tick: false,
            cell_colors: Colors
            (
                Color { r: 0.05, g: 0.05, b: 0.05 }, 
                Color { r: 0.95, g: 0.95, b: 0.95 },
            ),
            random_ratio: INITIAL_RANDOM_RATIO,
            alive_dead_count: (alive_count, dead_count),
            record,
            bg_color: [1.0, 1.0, 1.0],
            gap_size: INITIAL_GAP_SIZE,
        }
    }
     
    pub fn empty_board(grid_size: usize) -> Vec<Cell> {
        vec![Cell::Dead; grid_size]
    }

    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.num_grid_per_row + x
    }

    pub fn _unravel_index(&self, index: usize) -> (usize, usize) {
        (
            index % self.num_grid_per_row,
            index / self.num_grid_per_row,
        )
    }

    pub fn count_alive_neighbors(&self, current_grid: &[Cell], x: usize, y: usize) -> usize {
        let length = self.num_grid_per_row;
        let mut count = 0;

        // for &(dx, dy) in &NEIGHBORS {
        //     nx = (x + length + dx) % length;
        //     ny = (y + length + dy) % length;

        //     let index = self.index(nx, ny);
        //     count += match current_grid[index] {
        //         Cell::Empty => 0,
        //         Cell::Full  => 1,
        //     };
        // }

        let dx_list = [length -1, 0, 1];
        let dy_list = [length -1, 0, 1];

        for &dx in &dx_list {
            for &dy in &dy_list {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = (x + dx) % length;
                let ny = (y + dy) % length;

                count += match current_grid[self.index(nx, ny)] {
                    Cell::Dead => 0,
                    Cell::Alive  => 1,
                };
            }
        }

        count
    }

    fn tick(&mut self) {
        self.tick_count += 1;

        let width = self.num_grid_per_row;
        let mut alive = 0;
        let mut dead = 0;

        for y in 0..width {
            for x in 0..width {
                let alive_count = self.count_alive_neighbors(&self.current, x, y);
                let index = self.index(x, y);
                match self.current[index] {
                    Cell::Alive => {
                        alive += 1;
                        if alive_count == 2 || alive_count == 3 {
                            self.next[index] = Cell::Alive;
                            continue;
                        }
                    }
                    Cell::Dead => {
                        dead += 1;
                        if alive_count == 3 {
                            self.next[index] = Cell::Alive;
                            continue;
                        }
                    }
                }
                self.next[index] = Cell::Dead;
                // match (alive_count, self.current[index]) {
                //     (3, Cell::Dead) => {
                //         self.next[index] = Cell::Alive;
                        
                //     }
                //     (2 | 3, Cell::Alive) => {
                //         self.next[index] = Cell::Alive;
                //     }
                //     (_, _) => {
                //         self.next[index] = Cell::Dead;
                //     }
                // }
            }
        }

        self.alive_dead_count = (alive, dead);
        std::mem::swap(&mut self.current, &mut self.next);

        if self.tick_count % 10 == 0 {
            if self.record.len() > 100 {
                self.record.pop_front();
            }
            self.record.push_back(Record {
                alive_count: alive,
                dead_count: dead,
            });
        }
    }

    pub fn update(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(self.delay));        

        if !self.pause {
            self.tick();
        } else {
            if self.next_tick {
                self.tick();
                self.next_tick = false;
            }
        }
    }

}
