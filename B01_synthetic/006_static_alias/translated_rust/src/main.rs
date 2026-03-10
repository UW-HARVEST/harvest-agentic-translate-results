use std::env;

static mut INNER: i32 = 1;

/// When outer and inner alias (both point to INNER), *outer >= inner is always
/// true (they're equal), so inner += *outer doubles the value, returns &inner.
/// When they don't alias, the original branching logic applies.
fn static_alias_no_alias(outer: &mut i32) -> bool {
    unsafe {
        if *outer >= INNER {
            INNER += *outer;
            true // returns &inner
        } else {
            *outer += INNER;
            false // returns outer
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Error: should only be two (integer) arguments!");
        std::process::exit(1);
    }

    let initial_value: i32 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            std::process::exit(1);
        }
    };

    let iterations: i32 = match args[2].parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: second argument must be an integer!");
            std::process::exit(1);
        }
    };

    let mut outer_val = initial_value;
    let mut points_to_inner = false;

    for _ in 0..iterations {
        if points_to_inner {
            // outer == &inner: *outer >= inner is always true (same location)
            // inner += *outer doubles INNER
            unsafe {
                INNER += INNER;
            }
            // still points to inner
        } else {
            points_to_inner = static_alias_no_alias(&mut outer_val);
        }
        let val = if points_to_inner {
            unsafe { INNER }
        } else {
            outer_val
        };
        println!("{}", val);
    }
}
