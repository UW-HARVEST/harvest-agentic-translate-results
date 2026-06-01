use emlang::env;
use emlang::parser;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} FILE", args[0]);
        std::process::exit(1);
    }
    let path = &args[1];
    let mut p = parser::Parser::new();
    if p.load_file(path) != 0 {
        eprintln!("Error: Failed to open file '{}'", path);
        std::process::exit(1);
    }
    let result = p.parse();
    let prog = match result.prog {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!(
                "Error at {}:{}:{}: {}",
                result.path, result.row, result.col, e
            );
            std::process::exit(1);
        }
    };

    let mut e = env::Env::new(emlang::stack::DEFAULT_STACK_CAP, emlang::stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    match r.em {
        Ok(_) => std::process::exit(r.ex as i32),
        Err(err) => {
            eprintln!("Runtime error: {}", err);
            std::process::exit(1);
        }
    }
}
