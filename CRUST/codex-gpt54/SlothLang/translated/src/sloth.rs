use crate::{parser, throw, slothvm, stack};
fn main() {
    let mut args = std::env::args();
    let _program_name = args.next();

    let Some(file_name) = args.next() else {
        eprintln!("Usage: sloth <source file>");
        std::process::exit(1);
    };

    if args.next().is_some() {
        eprintln!("Usage: sloth <source file>");
        std::process::exit(1);
    }

    let mut program = parser::parse(&file_name);
    let x = slothvm::execute(&mut program);
    println!("Returned: {x}");
    parser::free_program(program);
}
