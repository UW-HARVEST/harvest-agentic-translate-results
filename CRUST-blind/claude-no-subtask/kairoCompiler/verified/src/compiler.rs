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
pub fn compile_file(filename: &str, out_filename: &str, flags: i32) -> i32 {
    let mut process = match crate::cprocess::compile_process_create(filename, out_filename, flags) {
        Some(p) => p,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };

    // Build a lex process backed by the compiler.
    let lp_funcs = crate::lex_process::LexProcessFunctions {
        next_char: crate::cprocess::compile_process_next_char,
        peek_char: crate::cprocess::compile_process_peek_char,
        push_char: crate::cprocess::compile_process_push_char,
    };
    let mut lex_proc = crate::lex_process::lex_process_create(process.clone(), lp_funcs, None);
    if crate::lexer::lex(&mut lex_proc) != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    process.token_vec = lex_proc.token_vec.clone();
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
    crate::cprocess::compile_process_create(filename, filename_out, flags).unwrap_or_default()
}

/// Reads the next character in the lex process.
pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    // Move the per-process file handling into a temporary lex_process::LexProcess
    // so we reuse the cprocess implementation.
    forward_to_cprocess(lex_process, CprocessOp::Next, '\0')
}

/// Peeks the next character without consuming it in the lex process.
pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    forward_to_cprocess(lex_process, CprocessOp::Peek, '\0')
}

/// Pushes a character back into the lex process.
pub fn compile_process_push_char(lex_process: &mut LexProcess, c: char) {
    let _ = forward_to_cprocess(lex_process, CprocessOp::Push, c);
}

#[derive(Copy, Clone)]
enum CprocessOp {
    Next,
    Peek,
    Push,
}

fn forward_to_cprocess(lex_process: &mut LexProcess, op: CprocessOp, c: char) -> char {
    // We bridge by constructing a temporary lex_process::LexProcess that shares the
    // same compiler. Since compile_process_* functions in cprocess use the abs_path
    // as the key for their global state, this is safe.
    let compiler_box = lex_process
        .compiler
        .clone()
        .unwrap_or_else(|| Box::new(CompileProcess::default()));
    let mut tmp = crate::lex_process::LexProcess {
        pos: lex_process.pos.clone(),
        token_vec: None,
        compiler: Some(compiler_box),
        function: None,
        private: lex_process.private,
        current_expression_count: lex_process.current_expression_count,
        parentheses_buffer: lex_process.parentheses_buffer.clone(),
        private_buffer: None,
    };
    let res = match op {
        CprocessOp::Next => crate::cprocess::compile_process_next_char(&mut tmp),
        CprocessOp::Peek => crate::cprocess::compile_process_peek_char(&mut tmp),
        CprocessOp::Push => {
            crate::cprocess::compile_process_push_char(&mut tmp, c);
            '\0'
        }
    };
    // Sync position back
    lex_process.pos = tmp.pos;
    if let (Some(c1), Some(c2)) = (lex_process.compiler.as_mut(), tmp.compiler) {
        c1.pos = c2.pos.clone();
    }
    res
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
    // The C version calls exit(-1); we instead just return so callers can handle it.
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
    LexProcess {
        pos: Pos {
            line: 1,
            col: 1,
            filename: None,
        },
        token_vec: Some(crate::vector::vector_create(std::mem::size_of::<u64>())),
        compiler: Some(Box::new(compiler)),
        current_expression_count: 0,
        parentheses_buffer: None,
        function: Some(functions),
        private,
    }
}

/// Frees resources used by a lex process.
pub fn lex_process_free(_process: LexProcess) {
    // Drop happens automatically.
}

/// Returns the private data associated with a lex process.
pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}

/// Returns the token vector of a lex process.
pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}

/// Runs the lexical analysis on a lex process.
pub fn lex(process: &mut LexProcess) -> i32 {
    // Bridge to lex_process::LexProcess for the actual lexer.
    let compiler_box = process
        .compiler
        .clone()
        .unwrap_or_else(|| Box::new(CompileProcess::default()));
    let mut tmp = crate::lex_process::LexProcess {
        pos: process.pos.clone(),
        token_vec: process.token_vec.clone(),
        compiler: Some(compiler_box),
        // Use the standard cprocess function pointers. The `function` field
        // in compiler::LexProcess is informational only and doesn't bridge
        // directly because of the differing LexProcess struct types.
        function: Some(crate::lex_process::LexProcessFunctions {
            next_char: crate::cprocess::compile_process_next_char,
            peek_char: crate::cprocess::compile_process_peek_char,
            push_char: crate::cprocess::compile_process_push_char,
        }),
        private: process.private,
        current_expression_count: process.current_expression_count,
        parentheses_buffer: process.parentheses_buffer.clone(),
        private_buffer: None,
    };
    let res = crate::lexer::lex(&mut tmp);
    process.token_vec = tmp.token_vec;
    process.pos = tmp.pos;
    process.current_expression_count = tmp.current_expression_count;
    process.parentheses_buffer = tmp.parentheses_buffer;
    res
}

/// Determines if a token is a keyword matching the given value.
pub fn token_is_keyword(token: &Token, value: &str) -> bool {
    let mut t = token.clone();
    crate::token::token_is_keyword(&mut t, value)
}

/// Creates a lex process for a string rather than a file, for quick token building.
pub fn tokens_build_for_string(compiler: CompileProcess, s: &str) -> LexProcess {
    use crate::buffer::buffer_printf;
    let mut buf = crate::buffer::buffer_create();
    buffer_printf(&mut buf, s);
    let mut lp = LexProcess {
        pos: Pos {
            line: 1,
            col: 1,
            filename: None,
        },
        token_vec: Some(crate::vector::vector_create(std::mem::size_of::<u64>())),
        compiler: Some(Box::new(compiler)),
        current_expression_count: 0,
        parentheses_buffer: None,
        function: None,
        private: None,
    };
    let _ = buf;
    let _ = lex(&mut lp);
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

/// Pops a node from some global or external node stack (placeholder).
pub fn node_pop() -> Option<Node> {
    Some(crate::node::node_pop().into())
}

/// Peeks the top node from a global or external node stack (placeholder).
pub fn node_peek() -> Option<Node> {
    Some(crate::node::node_peek().into())
}

/// Peeks the top node from a global or external node stack, returning null if none (placeholder).
pub fn node_peek_or_null() -> Option<Node> {
    crate::node::node_peek_or_null().map(Into::into)
}

/// Pushes a node onto some global or external node stack (placeholder).
pub fn node_push(node: Node) {
    crate::node::node_push(&node.into());
}

/// Sets the provided vector references in some global or external node context.
pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    crate::node::node_set_vector(vec, root_vec);
}

/// Creates a new node and returns it.
pub fn node_create(node: &Node) -> Node {
    crate::node::node_create(&node.clone().into()).into()
}

// Conversions between compiler::Node and node::Node (they are structurally
// identical but distinct types).
impl From<crate::node::Node> for Node {
    fn from(n: crate::node::Node) -> Self {
        Node {
            r#type: n.r#type,
            flags: n.flags,
            pos: n.pos,
            binded: NodeBinded {
                owner: n.binded.owner.map(|b| Box::new((*b).into())),
                function: n.binded.function.map(|b| Box::new((*b).into())),
            },
            cval: n.cval,
            sval: n.sval,
            inum: n.inum,
            lnum: n.lnum,
            llnum: n.llnum,
        }
    }
}

impl From<Node> for crate::node::Node {
    fn from(n: Node) -> Self {
        crate::node::Node {
            r#type: n.r#type,
            flags: n.flags,
            pos: n.pos,
            binded: crate::node::NodeBinded {
                owner: n.binded.owner.map(|b| Box::new((*b).into())),
                function: n.binded.function.map(|b| Box::new((*b).into())),
            },
            cval: n.cval,
            sval: n.sval,
            inum: n.inum,
            lnum: n.lnum,
            llnum: n.llnum,
        }
    }
}