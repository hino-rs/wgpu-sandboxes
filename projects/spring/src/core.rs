pub struct Object {
    pub x: f32,
    pub y: f32,

    // 速度
    pub vx: f32,
    pub vy: f32,

    // 加速
    pub ax: f32,
    pub ay: f32,
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
        }
    }
}

impl Object {
    pub fn calc(&mut self, dt: f32, g: f32, k: f32, c: f32) {
        let m = 1.0;

        self.ay = ((-k * self.y) - (c * self.vy)) / m;
        self.vy += self.ay * dt;
        self.y += self.vy * dt;

        self.ax = ((-k * self.x) - (c * self.vx)) / m;
        self.vx += self.ax * dt;
        self.x += self.vx * dt;

        self.y -= g * dt;
    }

    pub fn reset(&mut self) {
        self.x = 0.0;
        self.y = 0.5;
        self.vy = 0.0;
        self.ay = 0.0;
    }

    pub fn update_pos(&mut self, new_x: f32, new_y: f32) {
        self.x = new_x;
        self.y = new_y;
    }
}
