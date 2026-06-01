// Binary entry point for impcheck_parse
// Includes modules from src/ directly

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
#[path = "../trusted_parser.rs"]
mod trusted_parser;
#[path = "../main_parse.rs"]
mod main_parse;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() as i32;
    let res = main_parse::main(argc, args);
    std::process::exit(res);
}
