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
    // pub cell_colors: Colors,
    pub random_ratio: f64,
    pub alive_dead_count: (u64, u64),
    pub record: VecDeque<Record>,
    pub bg_color: [f32; 3],
    pub gap_size: f32,
    pub alive_cell_color: [f32; 3],
    pub dead_cell_color: [f32; 3],
    pub rule: Rule,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Rule {
    ConwaysGameOfLife,
    HighLife,
    Seeds,
    Maze,
    Mazectric,
    Replicator,
    DayAndNight,
    Morley,
    TwoxTwo,
    Walling,
    LiveFreeOrDie,
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Rule::ConwaysGameOfLife => "Conway's Game of Life",
                Rule::HighLife => "HighLife",
                Rule::Seeds => "Seeds",
                Rule::Maze => "Maze",
                Rule::Mazectric => "Mazectric",
                Rule::Replicator => "Replicator",
                Rule::DayAndNight => "Day & Night",
                Rule::Morley => "Morley",
                Rule::TwoxTwo => "2x2",
                Rule::Walling => "Walling",
                Rule::LiveFreeOrDie => "Live Free or Die",
            }
        )
    }
}

impl Rule {
    pub fn to_text(rule: Rule) -> &'static str {
        match rule {
            Rule::ConwaysGameOfLife => "Conway's Game of Life",
            Rule::HighLife => "HighLife",
            Rule::Seeds => "Seeds",
            Rule::Maze => "Maze",
            Rule::Mazectric => "Mazectric",
            Rule::Replicator => "Replicator",
            Rule::DayAndNight => "Day & Night",
            Rule::Morley => "Morley",
            Rule::TwoxTwo => "2x2",
            Rule::Walling => "Walling",
            Rule::LiveFreeOrDie => "Live Free or Die",
        }
    }
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
    pub fn alive_rgb_u8(&self) -> [u8; 3] {
        [
            ratio_to_u8(self.alive_cell_color[0]),
            ratio_to_u8(self.alive_cell_color[1]),
            ratio_to_u8(self.alive_cell_color[2]),
        ]
    }

    pub fn dead_rgb_u8(&self) -> [u8; 3] {
        [
            ratio_to_u8(self.dead_cell_color[0]),
            ratio_to_u8(self.dead_cell_color[1]),
            ratio_to_u8(self.dead_cell_color[2]),
        ]
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
            let new_height = if current_height > 1 {
                current_height - 1
            } else {
                0
            };
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
            state.update_instances(
                &self.current,
                self.num_grid_per_row,
                self.gap_size,
                self.alive_cell_color,
                self.dead_cell_color,
            );
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
            state.update_instances(
                &self.current,
                self.num_grid_per_row,
                self.gap_size,
                self.alive_cell_color,
                self.dead_cell_color,
            );
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
        record.push_back(Record {
            alive_count,
            dead_count,
        });

        Self {
            tick_count: 0,
            num_grid_per_row,
            grid_size,
            next: current.clone(),
            current,
            delay: 1,
            pause: false,
            next_tick: false,
            // cell_colors: Colors
            // (
            //     Color { r: 0.05, g: 0.05, b: 0.05 },
            //     Color { r: 0.95, g: 0.95, b: 0.95 },
            // ),
            random_ratio: INITIAL_RANDOM_RATIO,
            alive_dead_count: (alive_count, dead_count),
            record,
            bg_color: [1.0, 1.0, 1.0],
            gap_size: INITIAL_GAP_SIZE,
            alive_cell_color: [0.95, 0.95, 0.95],
            dead_cell_color: [0.05, 0.05, 0.05],
            rule: Rule::ConwaysGameOfLife,
        }
    }

    pub fn empty_board(grid_size: usize) -> Vec<Cell> {
        vec![Cell::Dead; grid_size]
    }

    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.num_grid_per_row + x
    }

    pub fn _unravel_index(&self, index: usize) -> (usize, usize) {
        (index % self.num_grid_per_row, index / self.num_grid_per_row)
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

        let dx_list = [length - 1, 0, 1];
        let dy_list = [length - 1, 0, 1];

        for &dx in &dx_list {
            for &dy in &dy_list {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = (x + dx) % length;
                let ny = (y + dy) % length;

                count += match current_grid[self.index(nx, ny)] {
                    Cell::Dead => 0,
                    Cell::Alive => 1,
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
                
                let next_cell = 
                    match self.rule {
                        Rule::ConwaysGameOfLife => Rule::conways_game_of_life(self.current[index], alive_count),
                        Rule::HighLife          => Rule::high_life(self.current[index], alive_count),
                        Rule::Seeds             => Rule::seeds(self.current[index], alive_count),
                        Rule::Maze              => Rule::maze(self.current[index], alive_count),
                        Rule::Mazectric         => Rule::mazectric(self.current[index], alive_count),
                        Rule::Replicator        => Rule::replicator(self.current[index], alive_count),
                        Rule::DayAndNight       => Rule::day_and_night(self.current[index], alive_count),
                        Rule::Morley            => Rule::morley(self.current[index], alive_count),
                        Rule::TwoxTwo           => Rule::two_x_two(self.current[index], alive_count),
                        Rule::Walling           => Rule::walling(self.current[index], alive_count),
                        Rule::LiveFreeOrDie     => Rule::live_free_or_die(self.current[index], alive_count),
                    };

                match next_cell {
                    Cell::Alive => alive += 1,
                    Cell::Dead  => dead += 1,
                }

                self.next[index] = next_cell;
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

impl Rule {
    fn conways_game_of_life(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 2 | 3) => {
                Cell::Alive
            }
            (Cell::Dead, 3) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn high_life(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 2 | 3) => {
                Cell::Alive
            }
            (Cell::Dead, 3 | 6) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }
    
    fn seeds(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Dead, 2) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn maze(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 1 | 2 | 3 | 4 | 5) => {
                Cell::Alive
            }
            (Cell::Dead, 3) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn mazectric(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 1 | 2 | 3 | 4) => {
                Cell::Alive
            }
            (Cell::Dead, 3) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn replicator(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 1 | 3 | 5 | 7) => {
                Cell::Alive
            }
            (Cell::Dead, 1 | 3 | 5 | 7) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn day_and_night(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 3 | 4 | 6 | 7 | 8) => {
                Cell::Alive
            }
            (Cell::Dead, 3 | 6 | 7 | 8) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn morley(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 2 | 4 | 5) => {
                Cell::Alive
            }
            (Cell::Dead, 3 | 6 | 8) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn two_x_two(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 1 | 2 | 5) => {
                Cell::Alive
            }
            (Cell::Dead, 3 | 6) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn walling(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 2 | 3 | 4 | 5) => {
                Cell::Alive
            }
            (Cell::Dead, 4 | 5 | 6 | 7 | 8) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }

    fn live_free_or_die(cell: Cell, alive_count: usize) -> Cell {
        match (cell, alive_count) {
            (Cell::Alive, 0) => {
                Cell::Alive
            }
            (Cell::Dead, 2) => {
                Cell::Alive
            }
            _ => {
                Cell::Dead
            }
        }
    }
}