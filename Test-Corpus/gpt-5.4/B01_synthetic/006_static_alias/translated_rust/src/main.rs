use std::env;
use std::process;
use std::sync::atomic::{AtomicI32, Ordering};

static INNER: AtomicI32 = AtomicI32::new(1);

enum RunningSum {
    Outer(i32),
    Inner,
}

fn static_alias(running_sum: &mut RunningSum) {
    match running_sum {
        RunningSum::Outer(outer) => {
            let inner = INNER.load(Ordering::SeqCst);
            if *outer >= inner {
                INNER.fetch_add(*outer, Ordering::SeqCst);
                *running_sum = RunningSum::Inner;
            } else {
                *outer += inner;
            }
        }
        RunningSum::Inner => {}
    }
}

fn current_value(running_sum: &RunningSum) -> i32 {
    match running_sum {
        RunningSum::Outer(value) => *value,
        RunningSum::Inner => INNER.load(Ordering::SeqCst),
    }
}

fn parse_i32_arg(value: &str) -> Option<i32> {
    value.parse::<i32>().ok()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Error: should only be two (integer) arguments!");
        process::exit(1);
    }

    let initial_value = match parse_i32_arg(&args[1]) {
        Some(v) => v,
        None => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let iterations = match parse_i32_arg(&args[2]) {
        Some(v) => v,
        None => {
            println!("Error: second argument must be an integer!");
            process::exit(1);
        }
    };

    let mut running_sum = RunningSum::Outer(initial_value);
    for _ in 0..iterations {
        static_alias(&mut running_sum);
        println!("{}", current_value(&running_sum));
    }
}
