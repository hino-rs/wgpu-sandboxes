use crate::cell::Cell;
// const NEIGHBORS: [(isize, isize); 8] = [
//     (-1, -1), (0, -1), (1, -1),
//     (-1,  0),          (1,  0),
//     (-1,  1), (0,  1), (1,  1),
// ];

#[derive(Clone)]
pub struct Board {
    pub num_grid_per_row: usize,
    pub grid_size: usize,
    pub current: Vec<Cell>,
    pub next: Vec<Cell>,
}

impl Board {
    pub fn cells(&self) -> &[Cell] {
        &self.current
    }

    pub fn new(num_grid_per_row: usize) -> Self {
        let grid_size = num_grid_per_row * num_grid_per_row;

        let mut current = Self::empty_board(grid_size);

        for i in 0..current.len() {
            if rand::random_bool(0.25) {
                current[i] = Cell::Alive;
            }
        }

        Self {
            num_grid_per_row,
            grid_size,
            next: current.clone(),
            current,
        }
    }
     
    pub fn empty_board(grid_size: usize) -> Vec<Cell> {
        vec![Cell::Dead; grid_size]
    }

    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.num_grid_per_row + x
    }

    pub fn unravel_index(&self, index: usize) -> (usize, usize) {
        (
            index % self.num_grid_per_row,
            index / self.num_grid_per_row,
        )
    }

    pub fn count_alive_neighbors(&self, current_grid: &Vec<Cell>, x: usize, y: usize) -> usize {
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

    pub fn update(&mut self) {
        let width = self.num_grid_per_row;

        for y in 0..width {
            for x in 0..width {
                let alive_count = self.count_alive_neighbors(&self.current, x, y);
                let index = self.index(x, y);
                match (alive_count, self.current[index]) {
                    (3, Cell::Dead) => {
                        self.next[index] = Cell::Alive;
                    }
                    (2 | 3, Cell::Alive) => {
                        self.next[index] = Cell::Alive;
                    }
                    (_, _) => {
                        self.next[index] = Cell::Dead;
                    }
                }
            }
        }

        std::mem::swap(&mut self.current, &mut self.next);
    }
}