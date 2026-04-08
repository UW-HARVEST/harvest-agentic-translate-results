use emlang::em as em;
use emlang::parser as parser;
use emlang::env;
use emlang::stack;

pub fn parse(path: &str) -> em::Program {
    let mut p = parser::Parser::new();
    if p.load_file(path) != 0 {
        eprintln!("Error: Failed to open file '{}'", path);
        std::process::exit(1);
    }
    let result = p.parse();
    match result.prog {
        Ok(prog) => prog,
        Err(err) => {
            eprintln!("Error at {}:{}:{}: {}", result.path, result.row, result.col, err);
            std::process::exit(1);
        }
    }
}

pub fn usage(path: &str) {
    println!(":O emlang :)");
    println!("https://github.com/lordoftrident/emlang");
    println!();
    println!("Usage: {} FILE | OPTIONS", path);
    println!("Options:");
    println!("  -h, --help    Show the usage");
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Error: No file provided");
        eprintln!("Try '{} -h'", args[0]);
        std::process::exit(1);
    }
    if args[1] == "-h" || args[1] == "--help" {
        usage(&args[0]);
        return;
    }

    let prog = parse(&args[1]);
    let mut e = env::Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let result = e.run(&prog);

    match result.em {
        Ok(_) => std::process::exit(result.ex as i32),
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}
