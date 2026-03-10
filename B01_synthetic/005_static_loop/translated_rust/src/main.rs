fn static_sum(update: i32) -> i32 {
    static mut SUM: i32 = 0;
    unsafe {
        SUM += update;
        SUM
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

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
