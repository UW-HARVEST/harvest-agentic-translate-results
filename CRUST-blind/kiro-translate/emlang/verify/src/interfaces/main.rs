use emlang::em as em;
use emlang::parser as parser;
pub fn parse(path: &str) -> em::Program {
    emlang::parse(path)
}
pub fn usage(path: &str) {
    emlang::usage(path);
}
pub fn main(){}
