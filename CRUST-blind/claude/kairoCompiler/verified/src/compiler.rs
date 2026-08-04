use crate::buffer::Buffer;
use crate::vector::Vector;
use std::fs::File;
use std::fmt::Debug;
use std::io::{self, Read, Seek, SeekFrom, Write};
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
/// Clonable File
#[derive(Debug)]
pub struct ClonableFile {
    file: File,
    path: PathBuf,  // Store the file path
}
impl ClonableFile {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let file = File::open(&path)?;
        Ok(Self { file, path })
    }

    /// Reads a single byte. Returns None on EOF.
    pub fn read_byte(&mut self) -> Option<u8> {
        let mut buf = [0u8; 1];
        match self.file.read(&mut buf) {
            Ok(0) => None,
            Ok(_) => Some(buf[0]),
            Err(_) => None,
        }
    }

    /// Peeks the next byte without consuming.
    pub fn peek_byte(&mut self) -> Option<u8> {
        let mut buf = [0u8; 1];
        match self.file.read(&mut buf) {
            Ok(0) => None,
            Ok(_) => {
                // seek back by 1 so the next read sees the same byte
                let _ = self.file.seek(SeekFrom::Current(-1));
                Some(buf[0])
            }
            Err(_) => None,
        }
    }

    /// Pushes a byte back so it'll be returned by the next read.
    pub fn unget_byte(&mut self, _b: u8) {
        // We approximate ungetc by seeking back one byte.
        let _ = self.file.seek(SeekFrom::Current(-1));
    }

    /// Writes a byte to the file (for output).
    pub fn write_byte(&mut self, b: u8) {
        let _ = self.file.write_all(&[b]);
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
pub any: Option<()>, // placeholder
pub num: TokenNumber,
pub whitespace: bool,
pub between_brackets: Option<String>,
}
/// Represents a node in the compiler's AST.
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
// The union fields collapsed into optional typed fields:
pub cval: Option<char>,
pub sval: Option<String>,
pub inum: Option<u32>,
pub lnum: Option<u64>,
pub llnum: Option<u64>,
}
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
/// Represents a set of lexing function pointers.
#[derive(Clone, Debug)]
pub struct LexProcessFunctions {
// Using Rust function pointer types.
// These can also be Fn traits with `'static` if you prefer fully idiomatic closures.
pub next_char: fn(&mut LexProcess) -> char,
pub peek_char: fn(&mut LexProcess) -> char,
pub push_char: fn(&mut LexProcess, char),
}
/// Represents a lex process for token generation.
#[derive(Debug, Default)]
pub struct LexProcess {
pub pos: Pos,
pub token_vec: Option<Vector>,
pub compiler: Option<Box<CompileProcess>>,
pub current_expression_count: i32,
pub parentheses_buffer: Option<Buffer>,
pub function: Option<LexProcessFunctions>,
pub private: Option<()>, // placeholder
}
// Function Declarations
/// Compiles a file from `filename` to `out_filename` with specified flags.
pub fn compile_file(filename: &str, out_filename: &str, flags: i32) -> i32 {
    let mut process = match crate::cprocess::compile_process_create(filename, out_filename, flags) {
        Some(p) => p,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };

    // Build the lex_process::LexProcessFunctions
    let lex_funcs = crate::lex_process::LexProcessFunctions {
        next_char: crate::cprocess::compile_process_next_char,
        peek_char: crate::cprocess::compile_process_peek_char,
        push_char: crate::cprocess::compile_process_push_char,
    };

    let mut lex_process = crate::lex_process::lex_process_create(
        process.clone(),
        lex_funcs,
        None,
    );

    if crate::lexer::lex(&mut lex_process) != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    process.token_vec = lex_process.token_vec.clone();

    if crate::parser::parse(&mut process) != PARSE_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    COMPILER_FILE_COMPILED_OK
}

/// Creates a new compile process for the specified input/output filenames and flags.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> CompileProcess {
    crate::cprocess::compile_process_create(filename, filename_out, flags)
        .unwrap_or_else(|| CompileProcess::default())
}

/// Reads the next character in the lex process (compiler-module variant).
pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return '\u{FFFF}',
    };
    compiler.pos.col += 1;
    let c: char = match compiler.cfile.fp.as_mut() {
        Some(cfile) => match cfile.read_byte() {
            Some(b) => b as char,
            None => '\u{FFFF}',
        },
        None => '\u{FFFF}',
    };
    if c == '\n' {
        compiler.pos.line += 1;
        compiler.pos.col = 1;
    }
    c
}

/// Peeks the next character without consuming it in the lex process (compiler-module variant).
pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return '\u{FFFF}',
    };
    match compiler.cfile.fp.as_mut() {
        Some(cfile) => match cfile.peek_byte() {
            Some(b) => b as char,
            None => '\u{FFFF}',
        },
        None => '\u{FFFF}',
    }
}

/// Pushes a character back into the lex process (compiler-module variant).
pub fn compile_process_push_char(lex_process: &mut LexProcess, c: char) {
    let compiler = match lex_process.compiler.as_mut() {
        Some(cm) => cm,
        None => return,
    };
    if let Some(cfile) = compiler.cfile.fp.as_mut() {
        cfile.unget_byte(c as u8);
    }
}

/// Logs a compiler error message.
pub fn compiler_error(compiler: &mut CompileProcess, msg: &str) {
    eprint!("{}", msg);
    let filename = compiler.pos.filename.clone().unwrap_or_default();
    eprintln!(" on line {}, col {} in file {}", compiler.pos.line, compiler.pos.col, filename);
    // The C version exits the process. We emulate with a panic-like behavior would be unsafe.
    // Use std::process::exit to match exit(-1).
    std::process::exit(255);
}

/// Logs a compiler warning message.
pub fn compiler_warning(compiler: &mut CompileProcess, msg: &str) {
    eprint!("{}", msg);
    let filename = compiler.pos.filename.clone().unwrap_or_default();
    eprintln!(" on line {}, col {} in file {}", compiler.pos.line, compiler.pos.col, filename);
}

/// Creates a new lex process given a compiler, function pointers, and private data.
pub fn lex_process_create(
    compiler: CompileProcess,
    functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    let mut process = LexProcess::default();
    process.function = Some(functions);
    process.token_vec = Some(crate::vector::vector_create(std::mem::size_of::<Token>()));
    process.compiler = Some(Box::new(compiler));
    process.private = private;
    process.pos.line = 1;
    process.pos.col = 1;
    process
}

/// Frees resources used by a lex process.
pub fn lex_process_free(_process: LexProcess) {
    // Drop handles cleanup
}

/// Returns the private data associated with a lex process.
pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}

/// Returns the token vector of a lex process.
pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}

/// Runs the lexical analysis on a lex process. Adapts the compiler::LexProcess to
/// the lex_process::LexProcess used by the lexer module.
pub fn lex(process: &mut LexProcess) -> i32 {
    let compiler_box = process.compiler.take();
    if compiler_box.is_none() {
        return LEXICAL_ANALYSIS_INPUT_ERROR;
    }
    let compiler = *compiler_box.unwrap();
    let lex_funcs = crate::lex_process::LexProcessFunctions {
        next_char: crate::cprocess::compile_process_next_char,
        peek_char: crate::cprocess::compile_process_peek_char,
        push_char: crate::cprocess::compile_process_push_char,
    };
    let mut lp = crate::lex_process::lex_process_create(compiler, lex_funcs, process.private);
    let res = crate::lexer::lex(&mut lp);
    process.token_vec = lp.token_vec.take();
    process.compiler = lp.compiler.take();
    res
}

/// Determines if a token is a keyword matching the given value.
pub fn token_is_keyword(token: &Token, value: &str) -> bool {
    let mut t = token.clone();
    crate::token::token_is_keyword(&mut t, value)
}

/// Creates a lex process for a string rather than a file, for quick token building.
pub fn tokens_build_for_string(compiler: CompileProcess, s: &str) -> LexProcess {
    let mut buffer = crate::buffer::buffer_create();
    crate::buffer::buffer_printf(&mut buffer, s);

    let lex_funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };

    let process = lex_process_create(compiler, lex_funcs, None);
    process
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

/// Pops a node from some global or external node stack (placeholder).
pub fn node_pop() -> Option<Node> {
    let n = crate::node::node_pop();
    Some(node_from_node_module(&n))
}

/// Peeks the top node from a global or external node stack (placeholder).
pub fn node_peek() -> Option<Node> {
    let n = crate::node::node_peek();
    Some(node_from_node_module(&n))
}

/// Peeks the top node from a global or external node stack, returning null if none.
pub fn node_peek_or_null() -> Option<Node> {
    crate::node::node_peek_or_null().map(|n| node_from_node_module(&n))
}

/// Pushes a node onto some global or external node stack.
pub fn node_push(node: Node) {
    let n = node_to_node_module(&node);
    crate::node::node_push(&n);
}

/// Sets the provided vector references in some global or external node context.
pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    crate::node::node_set_vector(vec, root_vec);
}

/// Creates a new node and returns it.
pub fn node_create(node: &Node) -> Node {
    let n = node_to_node_module(node);
    let created = crate::node::node_create(&n);
    node_from_node_module(&created)
}

/// Convert a `crate::node::Node` to a top-level Node.
fn node_from_node_module(n: &crate::node::Node) -> Node {
    Node {
        r#type: n.r#type,
        flags: n.flags,
        pos: n.pos.clone(),
        binded: NodeBinded::default(),
        cval: n.cval,
        sval: n.sval.clone(),
        inum: n.inum,
        lnum: n.lnum,
        llnum: n.llnum,
    }
}

/// Convert a top-level Node to a `crate::node::Node`.
fn node_to_node_module(n: &Node) -> crate::node::Node {
    crate::node::Node {
        r#type: n.r#type,
        flags: n.flags,
        pos: n.pos.clone(),
        binded: crate::node::NodeBinded::default(),
        cval: n.cval,
        sval: n.sval.clone(),
        inum: n.inum,
        lnum: n.lnum,
        llnum: n.llnum,
    }
}