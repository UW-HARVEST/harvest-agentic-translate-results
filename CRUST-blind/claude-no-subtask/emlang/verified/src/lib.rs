pub mod data;
pub mod em;
pub mod env;
pub mod parser;
pub mod stack;

pub mod utils;

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
            eprintln!(
                "Error at {}:{}:{}: {}",
                result.path, result.row, result.col, err
            );
            std::process::exit(1);
        }
    }
}

pub fn usage(path: &str) {
    println!(
        ":O emlang :)\n\
         https://github.com/lordoftrident/emlang\n\n\
         Usage: {} FILE | OPTIONS\n\
         Options:\n  \
         -h, --help    Show the usage",
        path
    );
}
