// Binary entry point for impcheck_confirm

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
#[path = "../trusted_parser.rs"]
mod trusted_parser;
#[path = "../main_confirm.rs"]
mod main_confirm;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() as i32;
    let res = main_confirm::main(argc, args);
    std::process::exit(res);
}
