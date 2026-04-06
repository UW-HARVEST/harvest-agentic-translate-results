#[no_mangle]
pub extern "C" fn driver(x: i32) {
    let mut j: i32 = 0;
    for i in 0..x {
        println!("{} {}", i, j);
        j += 2;
    }
}

// Export main symbol only for cdylib (not when linked as rlib by the binary)
#[cfg(all(not(feature = "_bin"), not(test)))]
mod _main_export {
    #[no_mangle]
    pub extern "C" fn main() -> i32 {
        use std::io::Read;
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input).unwrap();
        let x: i32 = input.split_whitespace().next().unwrap().parse().unwrap();
        super::driver(x);
        0
    }
}
