use std::env;
use std::process;

static mut INNER: i32 = 1;

/// Returns a raw pointer to either the static INNER or the passed-in location,
/// exactly mirroring the C function's aliasing behavior.
unsafe fn static_alias(outer: *mut i32) -> *mut i32 {
    if *outer >= INNER {
        INNER += *outer;
        &raw mut INNER
    } else {
        *outer += INNER;
        outer
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Error: should only be two (integer) arguments!");
        process::exit(1);
    }

    let initial_value: i32 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let iterations: i32 = match args[2].parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: second argument must be an integer!");
            process::exit(1);
        }
    };

    unsafe {
        let mut initial_value = initial_value;
        let mut running_sum: *mut i32 = &mut initial_value;
        for _ in 0..iterations {
            running_sum = static_alias(running_sum);
            println!("{}", *running_sum);
        }
    }
}
