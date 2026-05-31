pub struct Math;

impl Math {
    // mix(a, b, t) = a + t * (b - a)
    pub fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
        [
            a[0] + t * (b[0] - a[0]),
            a[1] + t * (b[1] - a[1]),
            a[2] + t * (b[2] - a[2]),
            a[3] + t * (b[3] - a[3]),
        ]
    }
}