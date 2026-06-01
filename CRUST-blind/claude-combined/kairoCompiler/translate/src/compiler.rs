use crate::buffer::Buffer;
use crate::vector::Vector;
use std::fs::File;
use std::io;
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
    #[allow(dead_code)]
    file: File,
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
    pub private: Option<()>,
}

// Function Declarations

/// Compiles a file from `filename` to `out_filename` with specified flags.
pub fn compile_file(filename: &str, out_filename: &str, flags: i32) -> i32 {
    let process_opt = crate::cprocess::compile_process_create(filename, out_filename, flags);
    let process = match process_opt {
        Some(p) => p,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };

    let functions = crate::lex_process::LexProcessFunctions {
        next_char: crate::cprocess::compile_process_next_char,
        peek_char: crate::cprocess::compile_process_peek_char,
        push_char: crate::cprocess::compile_process_push_char,
    };
    let mut lex_process = crate::lex_process::lex_process_create(process, functions, None);

    if crate::lexer::lex(&mut lex_process) != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    // Move tokens into the compiler's token_vec
    let mut compile_process = match lex_process.compiler.take() {
        Some(b) => *b,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };
    compile_process.token_vec = lex_process.token_vec.take();

    if crate::parser::parse(&mut compile_process) != PARSE_ALL_OK {
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

/// Reads the next character in the lex process.
pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    // Bridge: convert &mut LexProcess (this module's) to &mut crate::lex_process::LexProcess
    // We don't reuse the cprocess function here because they take different LexProcess types.
    // Instead duplicate the logic by delegating to the cprocess version using a helper bridge.
    bridge_next_char(lex_process)
}

/// Peeks the next character without consuming it in the lex process.
pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    bridge_peek_char(lex_process)
}

/// Pushes a character back into the lex process.
pub fn compile_process_push_char(lex_process: &mut LexProcess, c: char) {
    bridge_push_char(lex_process, c)
}

// Bridges between this module's LexProcess and lex_process module's LexProcess.
// They share the same fields; we copy minimal fields needed.
fn make_bridge(lex_process: &LexProcess) -> crate::lex_process::LexProcess {
    crate::lex_process::LexProcess {
        pos: lex_process.pos.clone(),
        token_vec: lex_process.token_vec.clone(),
        compiler: lex_process.compiler.clone(),
        function: None,
        private: lex_process.private,
    }
}

fn bridge_next_char(lex_process: &mut LexProcess) -> char {
    let mut bridge = make_bridge(lex_process);
    let c = crate::cprocess::compile_process_next_char(&mut bridge);
    if let Some(b) = bridge.compiler.take() {
        lex_process.compiler = Some(b);
    }
    lex_process.pos = bridge.pos;
    c
}

fn bridge_peek_char(lex_process: &mut LexProcess) -> char {
    let mut bridge = make_bridge(lex_process);
    let c = crate::cprocess::compile_process_peek_char(&mut bridge);
    if let Some(b) = bridge.compiler.take() {
        lex_process.compiler = Some(b);
    }
    c
}

fn bridge_push_char(lex_process: &mut LexProcess, c: char) {
    let mut bridge = make_bridge(lex_process);
    crate::cprocess::compile_process_push_char(&mut bridge, c);
    if let Some(b) = bridge.compiler.take() {
        lex_process.compiler = Some(b);
    }
}

/// Logs a compiler error message.
pub fn compiler_error(_compiler: &mut CompileProcess, msg: &str) {
    eprintln!("compiler error: {}", msg);
}

/// Logs a compiler warning message.
pub fn compiler_warning(_compiler: &mut CompileProcess, msg: &str) {
    eprintln!("compiler warning: {}", msg);
}

/// Creates a new lex process given a compiler, function pointers, and private data.
pub fn lex_process_create(
    _compiler: CompileProcess,
    _functions: LexProcessFunctions,
    _private: Option<()>,
) -> LexProcess {
    LexProcess {
        pos: Pos { line: 1, col: 1, filename: None },
        token_vec: Some(crate::vector::vector_create(1)),
        compiler: Some(Box::new(_compiler)),
        current_expression_count: 0,
        parentheses_buffer: None,
        function: Some(_functions),
        private: _private,
    }
}

/// Frees resources used by a lex process.
pub fn lex_process_free(_process: LexProcess) {
    // dropped automatically
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
    // Bridge to crate::lexer::lex with crate::lex_process::LexProcess
    let mut bridge = crate::lex_process::LexProcess {
        pos: process.pos.clone(),
        token_vec: process.token_vec.clone(),
        compiler: process.compiler.clone(),
        function: None,
        private: process.private,
    };
    let res = crate::lexer::lex(&mut bridge);
    process.token_vec = bridge.token_vec.take();
    if let Some(b) = bridge.compiler.take() {
        process.compiler = Some(b);
    }
    process.pos = bridge.pos;
    res
}

/// Determines if a token is a keyword matching the given value.
pub fn token_is_keyword(token: &Token, value: &str) -> bool {
    // The C version has a bug where it assigns the type field, but for the read-only API
    // here we replicate the *return* semantics: if sval matches and we treat
    // TOKEN_TYPE_KEYWORD (=1) as truthy, the value returned is the eq result.
    match &token.sval {
        Some(s) => s == value,
        None => false,
    }
}

/// Creates a lex process for a string rather than a file, for quick token building.
pub fn tokens_build_for_string(compiler: CompileProcess, _str: &str) -> LexProcess {
    // Build a buffer-backed lex process. We don't actually run the full lexer here;
    // rather, we create a new lex_process with the string saved.
    let mut buf = crate::buffer::buffer_create();
    crate::buffer::buffer_printf(&mut buf, _str);
    let lp_funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let mut lp = lex_process_create(compiler, lp_funcs, None);
    lp.parentheses_buffer = Some(buf);
    lp
}

/// Parses the tokens to form an AST.
pub fn parse(process: &mut CompileProcess) -> i32 {
    crate::parser::parse(process)
}

/// Checks if a token is a specific symbol.
pub fn token_is_symbol(token: &Token, c: char) -> bool {
    token.r#type == TOKEN_TYPE_SYMBOL && token.cval == Some(c)
}

/// Checks if a token is a newline, comment, or other "skip" token.
pub fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    token.r#type == TOKEN_TYPE_NEWLINE
        || token.r#type == TOKEN_TYPE_COMMENT
        || token_is_symbol(token, '\\')
}

/// Pops a node from some global or external node stack.
pub fn node_pop() -> Option<Node> {
    let n = crate::node::node_pop();
    Some(node_from_node_module(&n))
}

/// Peeks the top node from a global or external node stack.
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
    let nm = node_to_node_module(node);
    let created = crate::node::node_create(&nm);
    node_from_node_module(&created)
}

// Conversion helpers between compiler::Node and node::Node (they have same fields)
fn node_to_node_module(n: &Node) -> crate::node::Node {
    crate::node::Node {
        r#type: n.r#type,
        flags: n.flags,
        pos: n.pos.clone(),
        binded: crate::node::NodeBinded {
            owner: None,
            function: None,
        },
        cval: n.cval,
        sval: n.sval.clone(),
        inum: n.inum,
        lnum: n.lnum,
        llnum: n.llnum,
    }
}

fn node_from_node_module(n: &crate::node::Node) -> Node {
    Node {
        r#type: n.r#type,
        flags: n.flags,
        pos: n.pos.clone(),
        binded: NodeBinded {
            owner: None,
            function: None,
        },
        cval: n.cval,
        sval: n.sval.clone(),
        inum: n.inum,
        lnum: n.lnum,
        llnum: n.llnum,
    }
}
