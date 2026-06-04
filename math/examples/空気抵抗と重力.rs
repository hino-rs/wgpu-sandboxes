const G: f64 = 9.8; // 重力加速度
const K: f64 = 0.5; // 空気抵抗

fn main() {
    let h = 0.001;
    let t_end = 10.0;
    
    let mut v = 0.0;
    let mut t = 0.0;
    
    while t < t_end - (h / 2.0) {
        v = (v + h*G) / (1.0 + K*h);
        t += h;
    }

    println!("h={h}: v={v}");
}
