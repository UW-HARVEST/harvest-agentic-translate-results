use crate::buffer::Buffer;
use crate::vector::Vector;
use std::fs::File;
use std::fmt::Debug;
use std::io::{self, Read};
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
pub fn compile_file(_filename: &str, _out_filename: &str, _flags: i32) -> i32 {
    let mut process = match crate::cprocess::compile_process_create(_filename, _out_filename, _flags) {
        Some(p) => p,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };

    // perform lexical analysis
    let lex_funcs = crate::lexer::COMPILER_LEX_FUNCTIONS;
    let mut lex_process = crate::lex_process::lex_process_create(process.clone(), lex_funcs, None);
    if crate::lexer::lex(&mut lex_process) != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    // Move the lex token vec across to the process
    process.token_vec = lex_process.token_vec.clone();

    // perform parsing
    if crate::parser::parse(&mut process) != PARSE_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    COMPILER_FILE_COMPILED_OK
}
/// Creates a new compile process for the specified input/output filenames and flags.
pub fn compile_process_create(
    _filename: &str,
    _filename_out: &str,
    _flags: i32,
) -> CompileProcess {
    crate::cprocess::compile_process_create(_filename, _filename_out, _flags)
        .unwrap_or_default()
}

/// Convert from compiler::LexProcessFunctions to lex_process::LexProcessFunctions.
fn to_inner_lex_funcs(_outer: LexProcessFunctions) -> crate::lex_process::LexProcessFunctions {
    // We can't directly translate the function pointers because they take different parameter types
    // (compiler::LexProcess vs lex_process::LexProcess). Instead, we substitute equivalent functions
    // that operate on the inner type.
    crate::lex_process::LexProcessFunctions {
        next_char: crate::cprocess::compile_process_next_char,
        peek_char: crate::cprocess::compile_process_peek_char,
        push_char: crate::cprocess::compile_process_push_char,
    }
}
/// Reads the next character in the lex process.
pub fn compile_process_next_char(_lex_process: &mut LexProcess) -> char {
    let mut tmp = crate::lex_process::LexProcess::default();
    tmp.compiler = _lex_process.compiler.clone();
    let c = crate::cprocess::compile_process_next_char(&mut tmp);
    _lex_process.compiler = tmp.compiler;
    c
}
/// Peeks the next character without consuming it in the lex process.
pub fn compile_process_peek_char(_lex_process: &mut LexProcess) -> char {
    let mut tmp = crate::lex_process::LexProcess::default();
    tmp.compiler = _lex_process.compiler.clone();
    let c = crate::cprocess::compile_process_peek_char(&mut tmp);
    _lex_process.compiler = tmp.compiler;
    c
}
/// Pushes a character back into the lex process.
pub fn compile_process_push_char(_lex_process: &mut LexProcess, _c: char) {
    let mut tmp = crate::lex_process::LexProcess::default();
    tmp.compiler = _lex_process.compiler.clone();
    crate::cprocess::compile_process_push_char(&mut tmp, _c);
    _lex_process.compiler = tmp.compiler;
}
/// Logs a compiler error message.
pub fn compiler_error(_compiler: &mut CompileProcess, _msg: &str) {
    eprintln!(
        "{} on line {}, col {} in file {}",
        _msg,
        _compiler.pos.line,
        _compiler.pos.col,
        _compiler.pos.filename.as_deref().unwrap_or("")
    );
    std::process::exit(-1);
}
/// Logs a compiler warning message.
pub fn compiler_warning(_compiler: &mut CompileProcess, _msg: &str) {
    eprintln!(
        "{} on line {}, col {} in file {}",
        _msg,
        _compiler.pos.line,
        _compiler.pos.col,
        _compiler.pos.filename.as_deref().unwrap_or("")
    );
}
/// Creates a new lex process given a compiler, function pointers, and private data.
pub fn lex_process_create(
    _compiler: CompileProcess,
    _functions: LexProcessFunctions,
    _private: Option<()>,
) -> LexProcess {
    let inner_lp = crate::lex_process::lex_process_create(
        _compiler,
        crate::lex_process::LexProcessFunctions {
            next_char: |lp: &mut crate::lex_process::LexProcess| -> char {
                if let Some(funcs) = lp.function {
                    (funcs.next_char)(lp)
                } else {
                    '\u{FFFF}'
                }
            },
            peek_char: |lp: &mut crate::lex_process::LexProcess| -> char {
                if let Some(funcs) = lp.function {
                    (funcs.peek_char)(lp)
                } else {
                    '\u{FFFF}'
                }
            },
            push_char: |lp: &mut crate::lex_process::LexProcess, c: char| {
                if let Some(funcs) = lp.function {
                    (funcs.push_char)(lp, c);
                }
            },
        },
        _private,
    );
    LexProcess {
        pos: inner_lp.pos,
        token_vec: inner_lp.token_vec,
        compiler: inner_lp.compiler,
        current_expression_count: 0,
        parentheses_buffer: None,
        function: Some(_functions),
        private: inner_lp.private,
    }
}
/// Frees resources used by a lex process.
pub fn lex_process_free(_process: LexProcess) {
    drop(_process);
}
/// Returns the private data associated with a lex process.
pub fn lex_process_private(_process: &LexProcess) -> Option<()> {
    _process.private
}
/// Returns the token vector of a lex process.
pub fn lex_process_tokens(_process: &LexProcess) -> Option<&Vector> {
    _process.token_vec.as_ref()
}
/// Runs the lexical analysis on a lex process.
pub fn lex(_process: &mut LexProcess) -> i32 {
    // Build a parallel internal lex_process and call into the lexer crate.
    let mut inner = crate::lex_process::LexProcess::default();
    inner.pos = _process.pos.clone();
    inner.token_vec = _process.token_vec.clone();
    inner.compiler = _process.compiler.clone();
    inner.function = _process.function.clone().map(to_inner_lex_funcs);
    inner.private = _process.private;

    let r = crate::lexer::lex(&mut inner);

    _process.pos = inner.pos;
    _process.token_vec = inner.token_vec;
    _process.compiler = inner.compiler;
    _process.private = inner.private;

    r
}
/// Determines if a token is a keyword matching the given value.
pub fn token_is_keyword(_token: &Token, _value: &str) -> bool {
    let mut tok = _token.clone();
    crate::token::token_is_keyword(&mut tok, _value)
}
/// Creates a lex process for a string rather than a file, for quick token building.
pub fn tokens_build_for_string(_compiler: CompileProcess, _str: &str) -> LexProcess {
    // Set up a buffer-backed input by storing the string as the compile process file
    // contents in CFILE_STATE.
    let mut compiler = _compiler;
    let key = compiler
        .cfile
        .abs_path
        .clone()
        .unwrap_or_else(|| String::from("<inline>"));
    compiler.cfile.abs_path = Some(key.clone());
    {
        let mut state = crate::cprocess::CFILE_STATE.lock().unwrap();
        state.insert(
            key,
            crate::cprocess::FileState {
                bytes: _str.as_bytes().to_vec(),
                pos: 0,
            },
        );
    }
    // The compiler lex functions live in lex_process module; map them into the public type here.
    let outer_funcs = LexProcessFunctions {
        next_char: |lp: &mut LexProcess| -> char {
            // delegate to compile_process_next_char which uses lp.compiler
            compile_process_next_char(lp)
        },
        peek_char: |lp: &mut LexProcess| -> char { compile_process_peek_char(lp) },
        push_char: |lp: &mut LexProcess, c: char| compile_process_push_char(lp, c),
    };
    let mut lp = lex_process_create(compiler, outer_funcs, None);
    let _ = lex(&mut lp);
    lp
}
/// Parses the tokens to form an AST.
pub fn parse(_process: &mut CompileProcess) -> i32 {
    crate::parser::parse(_process)
}
/// Checks if a token is a specific symbol.
pub fn token_is_symbol(_token: &Token, _c: char) -> bool {
    crate::token::token_is_symbol(_token, _c)
}
/// Checks if a token is a newline, comment, or other "skip" token.
pub fn token_is_nl_or_comment_or_newline_separator(_token: &Token) -> bool {
    crate::token::token_is_nl_or_comment_or_newline_separator(_token)
}
/// Pops a node from some global or external node stack (placeholder).
pub fn node_pop() -> Option<Node> {
    let n = crate::node::node_pop();
    Some(node_from_internal(&n))
}
/// Peeks the top node from a global or external node stack (placeholder).
pub fn node_peek() -> Option<Node> {
    let n = crate::node::node_peek();
    Some(node_from_internal(&n))
}
/// Peeks the top node from a global or external node stack, returning null if none (placeholder).
pub fn node_peek_or_null() -> Option<Node> {
    crate::node::node_peek_or_null().map(|n| node_from_internal(&n))
}
/// Pushes a node onto some global or external node stack (placeholder).
pub fn node_push(_node: Node) {
    let n = node_to_internal(&_node);
    crate::node::node_push(&n);
}
/// Sets the provided vector references in some global or external node context.
pub fn node_set_vector(_vec: Vector, _root_vec: Vector) {
    crate::node::node_set_vector(_vec, _root_vec);
}
/// Creates a new node and returns it.
pub fn node_create(_node: &Node) -> Node {
    let n = node_to_internal(_node);
    let created = crate::node::node_create(&n);
    node_from_internal(&created)
}

fn node_to_internal(n: &Node) -> crate::node::Node {
    crate::node::Node {
        r#type: n.r#type,
        flags: n.flags,
        pos: n.pos.clone(),
        binded: crate::node::NodeBinded {
            owner: n.binded.owner.as_ref().map(|o| Box::new(node_to_internal(o))),
            function: n
                .binded
                .function
                .as_ref()
                .map(|o| Box::new(node_to_internal(o))),
        },
        cval: n.cval,
        sval: n.sval.clone(),
        inum: n.inum,
        lnum: n.lnum,
        llnum: n.llnum,
    }
}

fn node_from_internal(n: &crate::node::Node) -> Node {
    Node {
        r#type: n.r#type,
        flags: n.flags,
        pos: n.pos.clone(),
        binded: NodeBinded {
            owner: n
                .binded
                .owner
                .as_ref()
                .map(|o| Box::new(node_from_internal(o))),
            function: n
                .binded
                .function
                .as_ref()
                .map(|o| Box::new(node_from_internal(o))),
        },
        cval: n.cval,
        sval: n.sval.clone(),
        inum: n.inum,
        lnum: n.lnum,
        llnum: n.llnum,
    }
}