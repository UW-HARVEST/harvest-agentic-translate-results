use crate::buffer::Buffer;
use crate::vector::Vector;
use std::fs::File;
use std::fmt::Debug;
use std::io::{self, Read};
use std::path::PathBuf;
// Constants
pub const COMPILER_FILE_COMPILED_OK: i32 = 0;
pub const COMPILER_FAILED_WITH_ERRORS: i32 = 1;
pub const PARSE_ALL_OK: i32 = 0;
pub const PARSE_GENERAL_ERROR: i32 = 1;
pub const NODE_TYPE_EXPRESSION: i32 = 0;
pub const NODE_TYPE_EXPRESSION_PARENTHESES: i32 = 1;
pub const NODE_TYPE_NUMBER: i32 = 2;
pub const NODE_TYPE_IDENTIFIER: i32 = 3;
pub const NODE_TYPE_STRING: i32 = 4;
pub const NODE_TYPE_VARIABLE: i32 = 5;
pub const NODE_TYPE_VARIABLE_LIST: i32 = 6;
pub const NODE_TYPE_FUNCTION: i32 = 7;
pub const NODE_TYPE_BODY: i32 = 8;
pub const NODE_TYPE_STATEMENT_RETURN: i32 = 9;
pub const NODE_TYPE_STATEMENT_IF: i32 = 10;
pub const NODE_TYPE_STATEMENT_ELSE: i32 = 11;
pub const NODE_TYPE_STATEMENT_WHILE: i32 = 12;
pub const NODE_TYPE_STATEMENT_DO_WHILE: i32 = 13;
pub const NODE_TYPE_STATEMENT_FOR: i32 = 14;
pub const NODE_TYPE_STATEMENT_BREAK: i32 = 15;
pub const NODE_TYPE_STATEMENT_CONTINUE: i32 = 16;
pub const NODE_TYPE_STATEMENT_SWITCH: i32 = 17;
pub const NODE_TYPE_STATEMENT_CASE: i32 = 18;
pub const NODE_TYPE_STATEMENT_DEFAULT: i32 = 19;
pub const NODE_TYPE_STATEMENT_GOTO: i32 = 20;
pub const NODE_TYPE_UNARY: i32 = 21;
pub const NODE_TYPE_TENARY: i32 = 22;
pub const NODE_TYPE_LABEL: i32 = 23;
pub const NODE_TYPE_STRUCT: i32 = 24;
pub const NODE_TYPE_UNION: i32 = 25;
pub const NODE_TYPE_BRACKET: i32 = 26;
pub const NODE_TYPE_CAST: i32 = 27;
pub const NODE_TYPE_BLANK: i32 = 28;
pub const LEXICAL_ANALYSIS_ALL_OK: i32 = 0;
pub const LEXICAL_ANALYSIS_INPUT_ERROR: i32 = 1;
pub const TOKEN_TYPE_IDENTIFIER: i32 = 0;
pub const TOKEN_TYPE_KEYWORD: i32 = 1;
pub const TOKEN_TYPE_OPERATOR: i32 = 2;
pub const TOKEN_TYPE_SYMBOL: i32 = 3;
pub const TOKEN_TYPE_NUMBER: i32 = 4;
pub const TOKEN_TYPE_STRING: i32 = 5;
pub const TOKEN_TYPE_COMMENT: i32 = 6;
pub const TOKEN_TYPE_NEWLINE: i32 = 7;
pub const NUMBER_TYPE_NORMAL: i32 = 0;
pub const NUMBER_TYPE_LONG: i32 = 1;
pub const NUMBER_TYPE_FLOAT: i32 = 2;
pub const NUMBER_TYPE_DOUBLE: i32 = 3;
// Structs
#[derive(Debug)]
pub struct ClonableFile {
    pub file: File,
    path: PathBuf,
}
impl ClonableFile {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let file = File::open(&path)?;
        Ok(Self { file, path })
    }
}
impl Clone for ClonableFile {
    fn clone(&self) -> Self {
        let file = File::open(&self.path).expect("Failed to reopen file");
        Self {
            file,
            path: self.path.clone(),
        }
    }
}
#[derive(Debug, Default, Clone)]
pub struct Pos {
    pub line: i32,
    pub col: i32,
    pub filename: Option<String>,
}
#[derive(Debug, Default, Clone)]
pub struct TokenNumber {
    pub r#type: i32,
}
#[derive(Debug, Default, Clone)]
pub struct Token {
    pub r#type: i32,
    pub flags: i32,
    pub pos: Pos,
    pub cval: Option<char>,
    pub sval: Option<String>,
    pub inum: Option<u32>,
    pub lnum: Option<u64>,
    pub llnum: Option<u64>,
    pub any: Option<()>,
    pub num: TokenNumber,
    pub whitespace: bool,
    pub between_brackets: Option<String>,
}
#[derive(Debug, Default, Clone)]
pub struct NodeBinded {
    pub owner: Option<Box<Node>>,
    pub function: Option<Box<Node>>,
}
#[derive(Debug, Default, Clone)]
pub struct Node {
    pub r#type: i32,
    pub flags: i32,
    pub pos: Pos,
    pub binded: NodeBinded,
    pub cval: Option<char>,
    pub sval: Option<String>,
    pub inum: Option<u32>,
    pub lnum: Option<u64>,
    pub llnum: Option<u64>,
}
#[derive(Debug, Default, Clone)]
pub struct CompileProcessInputFile {
    pub fp: Option<ClonableFile>,
    pub abs_path: Option<String>,
}
#[derive(Debug, Default, Clone)]
pub struct CompileProcess {
    pub flags: i32,
    pub pos: Pos,
    pub cfile: CompileProcessInputFile,
    pub token_vec: Option<Vector>,
    pub node_vec: Option<Vector>,
    pub node_tree_vec: Option<Vector>,
    pub ofile: Option<ClonableFile>,
}
#[derive(Clone, Copy, Debug)]
pub struct LexProcessFunctions {
    pub next_char: fn(&mut LexProcess) -> char,
    pub peek_char: fn(&mut LexProcess) -> char,
    pub push_char: fn(&mut LexProcess, char),
}
#[derive(Debug, Default)]
pub struct LexProcess {
    pub pos: Pos,
    pub token_vec: Option<Vector>,
    pub compiler: Option<Box<CompileProcess>>,
    pub current_expression_count: i32,
    pub parentheses_buffer: Option<Buffer>,
    pub function: Option<LexProcessFunctions>,
    pub private: Option<()>,
}
// Function Declarations
pub fn compile_file(_filename: &str, _out_filename: &str, _flags: i32) -> i32 {
    let process = match crate::cprocess::compile_process_create(_filename, _out_filename, _flags) {
        Some(p) => p,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };
    let mut lex_process = crate::lex_process::lex_process_create(
        process,
        crate::lexer::COMPILER_LEX_FUNCTIONS,
        None,
    );
    if crate::lexer::lex(&mut lex_process) != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }
    let mut process = *lex_process.compiler.take().unwrap();
    process.token_vec = lex_process.token_vec.take();
    if crate::parser::parse(&mut process) != PARSE_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }
    COMPILER_FILE_COMPILED_OK
}

pub fn compile_process_create(_filename: &str, _filename_out: &str, _flags: i32) -> CompileProcess {
    crate::cprocess::compile_process_create(_filename, _filename_out, _flags).unwrap_or_default()
}

pub fn compile_process_next_char(_lex_process: &mut LexProcess) -> char {
    crate::cprocess::compile_process_next_char(_lex_process)
}

pub fn compile_process_peek_char(_lex_process: &mut LexProcess) -> char {
    crate::cprocess::compile_process_peek_char(_lex_process)
}

pub fn compile_process_push_char(_lex_process: &mut LexProcess, _c: char) {
    crate::cprocess::compile_process_push_char(_lex_process, _c)
}

pub fn compiler_error(_compiler: &mut CompileProcess, _msg: &str) {
    eprintln!("{} on line {}, col {} in file {}",
        _msg, _compiler.pos.line, _compiler.pos.col,
        _compiler.pos.filename.as_deref().unwrap_or("<unknown>"));
    std::process::exit(-1);
}

pub fn compiler_warning(_compiler: &mut CompileProcess, _msg: &str) {
    eprintln!("{} on line {}, col {} in file {}",
        _msg, _compiler.pos.line, _compiler.pos.col,
        _compiler.pos.filename.as_deref().unwrap_or("<unknown>"));
}

pub fn lex_process_create(_compiler: CompileProcess, _functions: LexProcessFunctions, _private: Option<()>) -> LexProcess {
    crate::lex_process::lex_process_create(_compiler, _functions, _private)
}

pub fn lex_process_free(_process: LexProcess) {
    drop(_process);
}

pub fn lex_process_private(_process: &LexProcess) -> Option<()> {
    _process.private
}

pub fn lex_process_tokens(_process: &LexProcess) -> Option<&Vector> {
    _process.token_vec.as_ref()
}

pub fn lex(_process: &mut LexProcess) -> i32 {
    crate::lexer::lex(_process)
}

pub fn token_is_keyword(_token: &Token, _value: &str) -> bool {
    _token.r#type == TOKEN_TYPE_KEYWORD && _token.sval.as_deref() == Some(_value)
}

pub fn tokens_build_for_string(_compiler: CompileProcess, _str: &str) -> LexProcess {
    crate::lexer::tokens_build_for_string(_compiler, _str).unwrap_or_default()
}

pub fn parse(_process: &mut CompileProcess) -> i32 {
    crate::parser::parse(_process)
}

pub fn token_is_symbol(_token: &Token, _c: char) -> bool {
    crate::token::token_is_symbol(_token, _c)
}

pub fn token_is_nl_or_comment_or_newline_separator(_token: &Token) -> bool {
    crate::token::token_is_nl_or_comment_or_newline_separator(_token)
}

pub fn node_pop() -> Option<Node> {
    Some(crate::node::node_pop())
}

pub fn node_peek() -> Option<Node> {
    Some(crate::node::node_peek())
}

pub fn node_peek_or_null() -> Option<Node> {
    crate::node::node_peek_or_null()
}

pub fn node_push(_node: Node) {
    crate::node::node_push(&_node)
}

pub fn node_set_vector(_vec: Vector, _root_vec: Vector) {
    crate::node::node_set_vector(_vec, _root_vec)
}

pub fn node_create(_node: &Node) -> Node {
    crate::node::node_create(_node)
}
