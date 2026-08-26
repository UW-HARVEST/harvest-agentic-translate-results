use std::io;

fn driver(f: f64) {
    let x = f.to_bits();
    println!("{:016x} {:e} {:.4}", x, f, f);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let f: f64 = input.trim().parse().unwrap();
    driver(f);
}