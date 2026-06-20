use emlang::em as em;
use emlang::parser as parser;
pub fn parse(path: &str) -> em::Program {
    let mut p = parser::Parser::new();
    if p.load_file(path) != 0 {
        eprintln!("Error: Failed to open file '{path}'");
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
    println!(
        ":O emlang :)\nhttps://github.com/lordoftrident/emlang\n\nUsage: {path} FILE | OPTIONS\nOptions:\n  -h, --help    Show the usage"
    );
}
pub fn main(){}
