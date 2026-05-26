use crate::cell::Cell;
// const NEIGHBORS: [(isize, isize); 8] = [
//     (-1, -1), (0, -1), (1, -1),
//     (-1,  0),          (1,  0),
//     (-1,  1), (0,  1), (1,  1),
// ];

#[derive(Clone, Copy)]
pub struct Board {
    pub num_grid_per_row: usize,
    pub grid_size: usize,
}

impl Board {
    pub fn new(num_grid_per_row: usize) -> Self {
        Self {
            num_grid_per_row,
            grid_size: num_grid_per_row * num_grid_per_row,
        }
    }

    pub fn empty_board(&self) -> Vec<Cell> {
        vec![Cell::Dead; self.grid_size]
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
}