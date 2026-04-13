use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} A B", args[0]);
        std::process::exit(2);
    }
    let a: i32 = args[1].parse().expect("invalid integer");
    let b: i32 = args[2].parse().expect("invalid integer");

    let r_call = driver::op_fn(a, b);
    let mut acc = driver::init_for();
    driver::run_loop(&mut acc, driver::REPEAT);

    let x1 = driver::helper_call(a, b);
    let x2 = driver::helper_ptr(a, b);
    let x3 = driver::use_generated(driver::REPEAT);
    let g = driver::g_op(a, b);

    println!("op={} call={} acc={} g.call={}", driver::G_OP_NAME, r_call, acc, g);
    println!("summary={}", r_call + acc + x1 + x2 + x3 + g);
}
