use std::env;
use std::sync::atomic::{AtomicI32, Ordering};

static SUM: AtomicI32 = AtomicI32::new(0);

fn static_sum(update: i32) -> i32 {
    SUM.fetch_add(update, Ordering::SeqCst) + update
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        std::process::exit(1);
    }

    let stride: i32 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            std::process::exit(1);
        }
    };

    for i in 0..10 {
        println!("{}", static_sum(i * stride));
    }
}
