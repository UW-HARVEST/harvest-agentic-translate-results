use crate::buffer::Buffer;
use crate::vector::Vector;
use std::fs::File;
use std::fmt::Debug;
use std::io::{self};
use std::io::Read as _;
use std::path::PathBuf;

// Constants
// Typical compile results.
pub const COMPILER_FILE_COMPILED_OK: i32 = 0;
pub const COMPILER_FAILED_WITH_ERRORS: i32 = 1;
// Parse status codes.
pub const PARSE_ALL_OK: i32 = 0;
pub const PARSE_GENERAL_ERROR: i32 = 1;
// Node types.
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
// Lexical Analysis results.
pub const LEXICAL_ANALYSIS_ALL_OK: i32 = 0;
pub const LEXICAL_ANALYSIS_INPUT_ERROR: i32 = 1;
// Token types.
pub const TOKEN_TYPE_IDENTIFIER: i32 = 0;
pub const TOKEN_TYPE_KEYWORD: i32 = 1;
pub const TOKEN_TYPE_OPERATOR: i32 = 2;
pub const TOKEN_TYPE_SYMBOL: i32 = 3;
pub const TOKEN_TYPE_NUMBER: i32 = 4;
pub const TOKEN_TYPE_STRING: i32 = 5;
pub const TOKEN_TYPE_COMMENT: i32 = 6;
pub const TOKEN_TYPE_NEWLINE: i32 = 7;
// Number types.
pub const NUMBER_TYPE_NORMAL: i32 = 0;
pub const NUMBER_TYPE_LONG: i32 = 1;
pub const NUMBER_TYPE_FLOAT: i32 = 2;
pub const NUMBER_TYPE_DOUBLE: i32 = 3;

// Structs

/// Clonable File. Reads the file's contents into a buffer for cheap re-use.
#[derive(Debug)]
pub struct ClonableFile {
    file: File,
    path: PathBuf,
    /// Cached contents of the file.
    pub content: Vec<u8>,
    /// Current read position into `content`.
    pub read_pos: usize,
    /// Pushback buffer (LIFO of single bytes).
    pub pushback: Vec<u8>,
}

impl ClonableFile {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let mut file = File::open(&path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        // Reopen so the original file handle is at start; we don't actually use it.
        let file = File::open(&path)?;
        Ok(Self {
            file,
            path,
            content,
            read_pos: 0,
            pushback: Vec::new(),
        })
    }
}

impl Clone for ClonableFile {
    fn clone(&self) -> Self {
        let file = File::open(&self.path).expect("Failed to reopen file");
        Self {
            file,
            path: self.path.clone(),
            content: self.content.clone(),
            read_pos: self.read_pos,
            pushback: self.pushback.clone(),
        }
    }
}

/// Represents a position in a file (line, column, and filename).
#[derive(Debug, Default, Clone)]
pub struct Pos {
    pub line: i32,
    pub col: i32,
    pub filename: Option<String>,
}

/// Represents a numeric token type.
#[derive(Debug, Default, Clone)]
pub struct TokenNumber {
    pub r#type: i32,
}

/// Represents a token in the compiler.
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

// Re-export the canonical Node from the `node` module so we have a single shared type.
pub use crate::node::{Node, NodeBinded};

/// Represents a compiler process, including file pointers and associated data.
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

// Re-export the canonical LexProcess types from `lex_process` so the rest of the
// codebase (cprocess.rs, lexer.rs, parser.rs) and this module agree on a single type.
pub use crate::lex_process::{LexProcess, LexProcessFunctions};

// Function Declarations

/// Compiles a file from `filename` to `out_filename` with specified flags.
pub fn compile_file(filename: &str, out_filename: &str, flags: i32) -> i32 {
    let process = match crate::cprocess::compile_process_create(filename, out_filename, flags) {
        Some(p) => p,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };

    let functions = LexProcessFunctions {
        next_char: crate::cprocess::compile_process_next_char,
        peek_char: crate::cprocess::compile_process_peek_char,
        push_char: crate::cprocess::compile_process_push_char,
    };

    let mut lex_process = crate::lex_process::lex_process_create(process, functions, None);

    let lex_res = crate::lexer::lex(&mut lex_process);
    if lex_res != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    // Move data from lex_process back into the compile process and parse.
    let mut compile_process = match lex_process.compiler.take() {
        Some(b) => *b,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };
    compile_process.token_vec = lex_process.token_vec.take();

    let parse_res = crate::parser::parse(&mut compile_process);
    if parse_res != PARSE_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    COMPILER_FILE_COMPILED_OK
}

/// Creates a new compile process for the specified input/output filenames and flags.
/// (Mirrors the cprocess version but returns CompileProcess directly.)
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> CompileProcess {
    crate::cprocess::compile_process_create(filename, filename_out, flags).unwrap_or_default()
}

/// Reads the next character in the lex process.
pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    crate::cprocess::compile_process_next_char(lex_process)
}

/// Peeks the next character without consuming it in the lex process.
pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    crate::cprocess::compile_process_peek_char(lex_process)
}

/// Pushes a character back into the lex process.
pub fn compile_process_push_char(lex_process: &mut LexProcess, c: char) {
    crate::cprocess::compile_process_push_char(lex_process, c)
}

/// Logs a compiler error message.
pub fn compiler_error(compiler: &mut CompileProcess, msg: &str) {
    eprintln!(
        "{} on line {}, col {} in file {}",
        msg,
        compiler.pos.line,
        compiler.pos.col,
        compiler.pos.filename.as_deref().unwrap_or("<unknown>")
    );
    std::process::exit(-1);
}

/// Logs a compiler warning message.
pub fn compiler_warning(compiler: &mut CompileProcess, msg: &str) {
    eprintln!(
        "{} on line {}, col {} in file {}",
        msg,
        compiler.pos.line,
        compiler.pos.col,
        compiler.pos.filename.as_deref().unwrap_or("<unknown>")
    );
}

/// Creates a new lex process given a compiler, function pointers, and private data.
pub fn lex_process_create(
    compiler: CompileProcess,
    functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    crate::lex_process::lex_process_create(compiler, functions, private)
}

/// Frees resources used by a lex process.
pub fn lex_process_free(process: LexProcess) {
    crate::lex_process::lex_process_free(process)
}

/// Returns the private data associated with a lex process.
pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    crate::lex_process::lex_process_private(process)
}

/// Returns the token vector of a lex process.
pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    crate::lex_process::lex_process_tokens(process)
}

/// Runs the lexical analysis on a lex process.
pub fn lex(process: &mut LexProcess) -> i32 {
    crate::lexer::lex(process)
}

/// Determines if a token is a keyword matching the given value.
pub fn token_is_keyword(token: &Token, value: &str) -> bool {
    // The original C version mutates token; provide a non-mutating variant here.
    // (Equivalent to comparing the type and the value.)
    token.r#type == TOKEN_TYPE_KEYWORD
        && token.sval.as_deref().map(|s| s == value).unwrap_or(false)
}

/// Creates a lex process for a string rather than a file, for quick token building.
pub fn tokens_build_for_string(_compiler: CompileProcess, _str: &str) -> LexProcess {
    // Build a buffer-backed lex process.
    let mut lp = LexProcess::default();
    let mut buf = crate::buffer::buffer_create();
    for ch in _str.chars() {
        crate::buffer::buffer_write(&mut buf, ch);
    }
    lp.parentheses_buffer = Some(buf);
    lp.compiler = Some(Box::new(_compiler));
    lp.token_vec = Some(crate::vector::vector_create(std::mem::size_of::<usize>()));
    lp
}

/// Parses the tokens to form an AST.
pub fn parse(process: &mut CompileProcess) -> i32 {
    crate::parser::parse(process)
}

/// Checks if a token is a specific symbol.
pub fn token_is_symbol(token: &Token, c: char) -> bool {
    crate::token::token_is_symbol(token, c)
}

/// Checks if a token is a newline, comment, or other "skip" token.
pub fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    crate::token::token_is_nl_or_comment_or_newline_separator(token)
}

/// Pops a node from some global or external node stack.
pub fn node_pop() -> Option<Node> {
    Some(crate::node::node_pop())
}

/// Peeks the top node from a global or external node stack.
pub fn node_peek() -> Option<Node> {
    Some(crate::node::node_peek())
}

/// Peeks the top node from a global or external node stack, returning null if none.
pub fn node_peek_or_null() -> Option<Node> {
    crate::node::node_peek_or_null()
}

/// Pushes a node onto some global or external node stack.
pub fn node_push(node: Node) {
    crate::node::node_push(&node);
}

/// Sets the provided vector references in some global or external node context.
pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    crate::node::node_set_vector(vec, root_vec)
}

/// Creates a new node and returns it.
pub fn node_create(_node: &Node) -> Node {
    crate::node::node_create(_node)
}

// Placate dead-code lint for the cached file handle which is intentionally retained.
#[allow(dead_code)]
fn _keep_file_field_alive(c: &ClonableFile) -> &File {
    &c.file
}
