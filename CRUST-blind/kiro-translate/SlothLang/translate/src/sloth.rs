use crate::{parser, throw, slothvm, stack};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: sloth <source file>");
        std::process::exit(1);
    }
    let mut program = parser::parse(&args[1]);
    let x = slothvm::execute(&mut program);
    println!("Returned: {}", x);
}
