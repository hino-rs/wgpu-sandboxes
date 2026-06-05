pub struct Object {
    pub x: f32,
    pub y: f32,

    // 速度
    pub vx: f32,
    pub vy: f32,

    // 加速
    pub ax: f32,
    pub ay: f32,

    pub c: f32, // 抵抗
    pub k: f32, // 硬さ
    pub g: f32, // 重力
}

impl Default for Object {
    fn default() -> Self {
        Object {
            x: 0.0,
            y: 0.5,

            vx: 0.0,
            vy: 0.0,

            ax: 0.0,
            ay: 0.0,

            c: 0.5,
            k: 4.0,
            g: 1.0,
        }
    }
}

impl Object {
    pub fn calc(&mut self, dt: f32) {
        let m = 1.0;

        self.ay = ((-self.k * self.y) - (self.c * self.vy)) / m;
        self.vy += self.ay * dt;
        self.y += self.vy * dt;

        self.ax = ((-self.k * self.x) - (self.c * self.vx)) / m;
        self.vx += self.ax * dt;
        self.x += self.vx * dt;

        self.y -= self.g * dt;
    }

    pub fn reset(&mut self) {
        self.x = 0.0;
        self.y = 0.5;
        self.vy = 0.0;
        self.ay = 0.0;
    }
}
