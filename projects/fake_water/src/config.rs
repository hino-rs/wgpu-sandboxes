#[derive(Clone, Copy)]
pub struct Configs {
    pub sys: SysConfig,
    pub sim: SimConfig,
}

#[derive(Clone, Copy)]
pub struct SysConfig {
    pub processor: Processor,
}

#[derive(Clone, Copy)]
pub struct SimConfig {
    pub brush_radius: isize,
    pub num_grid_per_row: u64,
}

impl Default for Configs {
    fn default() -> Self {
        let sys = SysConfig {
            processor: Processor::CPU,
        };

        let sim = SimConfig {
            brush_radius: 4,
            num_grid_per_row: 128,
        };
        
        Self { sys, sim }
    }
}

// Process
impl Configs {
    
}

// UI
impl Configs {

}

#[derive(PartialEq, Clone, Copy)]
pub enum Processor {
    CPU,
    GPU,
}
