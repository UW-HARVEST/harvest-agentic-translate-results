use crate::{parser, throw, slothvm, stack};
fn main() {
    let mut args = std::env::args();
    let _bin = args.next();
    let Some(file_name) = args.next() else {
        eprintln!("Usage: sloth <source file>");
        return;
    };
    if args.next().is_some() {
        eprintln!("Usage: sloth <source file>");
        return;
    }

    let mut program = parser::parse(&file_name);
    let value = slothvm::execute(&mut program);
    println!("Returned: {value}");
    parser::free_program(program);
}
