use std::env;
use std::ffi::c_int;
use std::io::Write;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        let _ = write!(std::io::stderr(), "usage: {} A B\n", args[0]);
        process::exit(2);
    }
    let a: c_int = args[1].parse().unwrap_or(0);
    let b: c_int = args[2].parse().unwrap_or(0);

    let r_call = driver::op_fn(a, b);
    let acc = driver::run_loop();

    let x1 = driver::helper_call(a, b);
    let x2 = driver::helper_ptr(a, b);
    let x3 = driver::use_generated(driver::REPEAT);
    let g = driver::g_op(a, b);

    let _ = write!(std::io::stdout(), "op={} call={} acc={} g.call={}\n", driver::g_op_name(), r_call, acc, g);
    let _ = write!(std::io::stdout(), "summary={}\n", r_call + acc + x1 + x2 + x3 + g);
}
