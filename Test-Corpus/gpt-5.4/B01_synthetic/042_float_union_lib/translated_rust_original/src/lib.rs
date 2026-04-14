#[unsafe(no_mangle)]
pub extern "C" fn driver(f: f64) {
    let x = f.to_bits();
    println!("{:x} {:a} {:.4}", x, f, f);
}