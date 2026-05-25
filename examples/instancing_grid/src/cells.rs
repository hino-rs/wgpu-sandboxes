pub const NUM_GRID_PER_ROW: usize = 128;
pub const GRID_SIZE: usize = (NUM_GRID_PER_ROW * NUM_GRID_PER_ROW) as usize;
pub const GAP: f32 = 1.0;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Cell {
    Empty,
    Black,
    White,
}
