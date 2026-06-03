pub struct Rgb(pub [f32; 3]);
pub struct Rgba(pub [f32; 4]);

impl Rgb {
    pub const BLACK: Self = Self([0.0, 0.0, 0.0]);
    pub const WHITE: Self = Self([1.0, 1.0, 1.0]);

    pub fn new(r: f32, g: f32, b: f32) -> Rgb {
        Rgb([r, g, b])
    }

    pub fn wgpu(&self, a: f64) -> wgpu::Color {
        wgpu::Color { 
            r: self.0[0] as f64, 
            g: self.0[1] as f64, 
            b: self.0[2] as f64, 
            a,
        }
    }
}

impl Rgba {
    pub const BLACK: Self = Self([0.0, 0.0, 0.0, 1.0]);
    pub const WHITE: Self = Self([1.0, 1.0, 1.0, 1.0]);

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Rgba {
        Rgba([r, g, b, a])
    }

    pub fn wgpu(&self) -> wgpu::Color {
        wgpu::Color { 
            r: self.0[0] as f64, 
            g: self.0[1] as f64, 
            b: self.0[2] as f64, 
            a: self.0[3] as f64,
        }
    }
}
