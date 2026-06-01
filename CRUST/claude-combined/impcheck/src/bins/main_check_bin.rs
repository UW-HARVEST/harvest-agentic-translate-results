// Binary entry point for impcheck_check

#[path = "../checker_interface.rs"]
mod checker_interface;
#[path = "../trusted_utils.rs"]
mod trusted_utils;
#[path = "../secret.rs"]
mod secret;
#[path = "../siphash.rs"]
mod siphash;
#[path = "../siphash_global.rs"]
mod siphash_global;
#[path = "../confirm.rs"]
mod confirm;
#[path = "../lrat_check.rs"]
mod lrat_check;
#[path = "../top_check.rs"]
mod top_check;
#[path = "../trusted_checker.rs"]
mod trusted_checker;
#[path = "../main_check.rs"]
mod main_check;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() as i32;
    let res = main_check::main(argc, args);
    std::process::exit(res);
}
