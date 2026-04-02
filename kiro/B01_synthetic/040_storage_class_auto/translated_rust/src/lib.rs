#[no_mangle]
pub extern "C" fn driver(x: i32) {
    let y: i32 = x.wrapping_mul(2).wrapping_add(300);
    println!("{}", y);
}

#[cfg(feature = "cdylib_main")]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).unwrap_or(0);
    let x: i32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
    driver(x);
    0
}
