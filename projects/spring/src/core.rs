pub struct Object {
    pub x: f32,
    pub y: f32,

    pub vx: f32, // 速度
    pub ax: f32, // 加速
    pub cx: f32, // 抵抗
    pub kx: f32, // 硬さ
    pub gx: f32, // 重力

    pub vy: f32,
    pub ay: f32,
    pub cy: f32,
    pub ky: f32,
    pub gy: f32,
}

impl Default for Object {
    fn default() -> Self {
        Object {
            y: 0.0,
            vy: 0.0,
            ay: 0.0,
            cy: 0.5,
            ky: 4.0,
            gy: 1.0,

            x: 1.0,
            vx: 0.0,
            ax: 0.0,
            cx: 0.5,
            kx: 4.0,
            gx: 0.0,
        }
    }
}

impl Object {
    pub fn calc(&mut self, dt: f32) {
        let m = 1.0;

        self.ay = ((-self.ky * self.y) - (self.cy * self.vy)) / m;
        self.vy += self.ay * dt;
        self.y += self.vy * dt;
        
        self.ax = ((-self.kx * self.x) - (self.cx * self.vx)) / m;
        self.vx += self.ax * dt;
        self.x += self.vx * dt;

        self.y -= self.gy * dt;
    }

    pub fn reset(&mut self) {
        self.y = 1.0;
        self.vy = 0.0;
        self.ay = 0.0;
    }
}