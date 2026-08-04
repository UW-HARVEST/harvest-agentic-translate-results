use crate::{parser, throw, slothvm, stack};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: sloth <source file>");
        std::process::exit(1);
    }
    let file_name = &args[1];
    let mut program = parser::parse(file_name);
    let x = slothvm::execute(&mut program);
    println!("Returned: {}", x);
    parser::free_program(program);
    let _ = throw::math_err;
    let _ = stack::Stack::new;
}
