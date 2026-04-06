fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

#[no_mangle]
pub extern "C" fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

#[cfg(not(test))]
mod _export_main {
    use super::*;
    use std::io::Read;

    #[no_mangle]
    pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
        let mut input = String::new();
        let _ = std::io::stdin().read_to_string(&mut input);
        let mut x: f32 = 0.0;
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(v) = token.parse::<f32>() {
                x = v;
            }
        }
        driver(x);
        0
    }
}
