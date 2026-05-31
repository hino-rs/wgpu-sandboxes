pub const INITIAL_NUM_GRID_PER_ROW: usize = 128;
pub const TERRAIN_COLOR: [f32; 4]  = [0.4, 0.3, 0.2, 1.0]; // 地面の色
pub const WATER_SHALLOW: [f32; 4]  = [0.0, 0.7, 0.8, 0.3]; // 浅い水
pub const WATER_DEEP: [f32; 4]  = [0.0, 0.1, 0.5, 0.9]; // 深い水（濃い青）
const NEIGHBORS: [(isize, isize); 4] = [
         ( 0, -1), 
    (-1,  0),  ( 1,  0),
         ( 0,  1),
];

pub struct Board {
    pub num_grid_per_row: usize,
    pub current_squares: Vec<Square>,
    pub next_squares: Vec<Square>,
    pub pause: bool,
    pub next_step: bool,
    pub delay: u8,
    pub bg_color: [f32; 3],
}

impl Board {
    pub fn empty_board(grid_size: usize) -> Vec<Square> {
        vec![Square::default(); grid_size]
    }

    pub fn new(num_grid_per_row: usize) -> Self {
        let grid_size = num_grid_per_row * num_grid_per_row;

        let mut current = Self::empty_board(grid_size);

        for (i, c) in current.iter_mut().enumerate() {
            let depth = 
                if i > (num_grid_per_row * num_grid_per_row - num_grid_per_row) {
                    1000.0
                } else {
                    // let depth = rand::random_range(0.0..=1.0);
                    ((i as f32 / num_grid_per_row as f32) / num_grid_per_row as f32) * 5.0
                };
            // println!("{depth}");
                
            *c = Square {
                depth,
                // puddle: rand::random_range(0.0..=depth),
                // puddle: 1.0 - depth,
                puddle: 0.0,
            };
        }

        Self {
            num_grid_per_row: INITIAL_NUM_GRID_PER_ROW,
            next_squares: current.clone(),
            current_squares: current,
            pause: false,
            next_step: false,
            delay: 0,
            bg_color: [1.0, 1.0, 1.0],
        }
    }

    pub fn step(&mut self) {
        let length = self.num_grid_per_row;

        self.next_squares = self.current_squares.clone();

        // パラメータ
        let flow_rate = 0.5;     // 流速
        let absorption = 0.01;   // 自身が最底値のときの蒸発率

        for i in 0..self.current_squares.len() {
            let mut current_cell = self.current_squares[i];
            current_cell.puddle *= 0.999;

            self.next_squares[i].puddle = current_cell.puddle;

            let (x, y) = (i % self.num_grid_per_row, i / self.num_grid_per_row);

            let mut lowest_x = x;
            let mut lowest_y = y;
            let current_cell = self.current_squares[i];
            let current_height = current_cell.depth + current_cell.puddle;
            let mut lowest_height = current_height;
            
            // 近傍探索
            for &(dx, dy) in &NEIGHBORS {
                let nx = x as isize + dx;
                let ny = y as isize + dy;

                if nx >= 0 && nx < length as isize && ny >= 0 && ny < length as isize {
                    let nx = nx as usize;
                    let ny = ny as usize;

                    let neighbor = self.current_squares[ny * length + nx];
                    let neighbor_height = neighbor.depth + neighbor.puddle;

                    if neighbor_height < lowest_height {
                        lowest_height = neighbor_height;
                        lowest_x = nx;
                        lowest_y = ny;
                    }
                }
            }

            // 探索結果に基づいた水量の変化処理
            if lowest_x == x && lowest_y == y {
                // 自身が最も低い場合
                if current_cell.puddle > 0.0 {
                    // 水を徐々に減らす
                    let next_puddle = self.next_squares[i].puddle - (current_cell.puddle * absorption);
                    self.next_squares[i].puddle = f32::max(0.0, next_puddle);
                }
            } else {
                //  自分より低い近傍が見つかった場合
                if current_cell.puddle > 0.0 {
                    // 高低差を計算
                    let height_diff = current_height - lowest_height;
                    
                    // 水が水平（同じ高さ）になろうとする移動量（高低差の半分が上限）
                    let target_flow = (height_diff * 0.5) * flow_rate;
                    
                    // 自分が持っている以上の水は流せないので制限する
                    let flow = f32::min(current_cell.puddle, target_flow);

                    if flow > 0.001 {
                        // 自分のマスから水を減らす
                        self.next_squares[i].puddle -= flow;

                        // 最も低かった隣人のマスに水を加える
                        let lowest_index = lowest_y * length + lowest_x;
                        self.next_squares[lowest_index].puddle += flow;
                    }
                }
            }
        }

        std::mem::swap(&mut self.current_squares, &mut self.next_squares);
    }


    pub fn update(&mut self) {
        self.step();
    }
}

#[derive(Clone, Copy, Default)]
pub struct Square {
    pub puddle: f32,
    pub depth: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
        ];

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub scale: f32,
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>() + mem::size_of::<[f32; 4]>() ) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }

    pub fn pure_instances(size: usize) -> Vec<Self> {
        vec![Self {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 1.0],
            scale: 1.0,
        }; size]
    }
}

pub const SQUARE: &[Vertex] = &[
    // 三角形1（左下半分）
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [1.0, 1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.0],
        color: [1.0, 1.0, 1.0],
    },
    // 三角形2（右上半分）
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.0],
        color: [1.0, 1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.0],
        color: [1.0, 1.0, 1.0],
    },
];
