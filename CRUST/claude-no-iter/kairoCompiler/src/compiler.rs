use crate::buffer::Buffer;
use crate::vector::Vector;
use std::cell::RefCell;
use std::collections::HashMap;
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

// =============================================================================
// Internal storage for in-progress compilation.
// =============================================================================

thread_local! {
    /// Maps a file's absolute path to (contents, current read index).
    /// Used so that `compile_process_*_char` can read characters one at a time
    /// without having to keep a `File` handle around.
    pub(crate) static FILE_BUFFERS: RefCell<HashMap<String, (Vec<u8>, usize)>> =
        RefCell::new(HashMap::new());

    /// All `Token`s produced during the current lex pass. The `token_vec`
    /// field of `CompileProcess`/`LexProcess` only acts as a counter; the real
    /// token data lives here.
    pub(crate) static TOKEN_STORAGE: RefCell<Vec<Token>> = RefCell::new(Vec::new());
}

pub(crate) fn read_file_to_buffer(path: &str) -> io::Result<()> {
    let mut f = File::open(path)?;
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes)?;
    FILE_BUFFERS.with(|m| {
        m.borrow_mut().insert(path.to_string(), (bytes, 0));
    });
    Ok(())
}

pub(crate) fn buf_next(path: &str) -> char {
    FILE_BUFFERS.with(|m| {
        let mut map = m.borrow_mut();
        if let Some((bytes, idx)) = map.get_mut(path) {
            if *idx >= bytes.len() {
                return '\0';
            }
            let c = bytes[*idx] as char;
            *idx += 1;
            c
        } else {
            '\0'
        }
    })
}

pub(crate) fn buf_peek(path: &str) -> char {
    FILE_BUFFERS.with(|m| {
        let map = m.borrow();
        if let Some((bytes, idx)) = map.get(path) {
            if *idx >= bytes.len() {
                return '\0';
            }
            bytes[*idx] as char
        } else {
            '\0'
        }
    })
}

pub(crate) fn buf_push(path: &str) {
    FILE_BUFFERS.with(|m| {
        let mut map = m.borrow_mut();
        if let Some((_bytes, idx)) = map.get_mut(path) {
            if *idx > 0 {
                *idx -= 1;
            }
        }
    });
}

// =============================================================================
// Function Implementations
// =============================================================================

/// Compiles a file from `filename` to `out_filename` with specified flags.
pub fn compile_file(filename: &str, out_filename: &str, flags: i32) -> i32 {
    let mut process = compile_process_create(filename, out_filename, flags);
    if process.cfile.fp.is_none() {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    // Reset token storage for a fresh pass.
    TOKEN_STORAGE.with(|t| t.borrow_mut().clear());

    let lex_functions = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };

    let mut lex_process = lex_process_create(process, lex_functions, None);

    if lex(&mut lex_process) != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    // Move the token vector into the compiler.
    process = *lex_process.compiler.take().unwrap();
    process.token_vec = lex_process.token_vec.take();

    if parse(&mut process) != PARSE_ALL_OK {
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
    use crate::vector::vector_create;

    let cfile_fp = match ClonableFile::new(filename) {
        Ok(f) => Some(f),
        Err(_) => return CompileProcess::default(),
    };

    // Cache the file's contents for later byte-by-byte reads.
    if read_file_to_buffer(filename).is_err() {
        return CompileProcess::default();
    }

    let ofile = if filename_out.is_empty() {
        None
    } else {
        match File::create(filename_out) {
            Ok(file) => Some(ClonableFile {
                file,
                path: PathBuf::from(filename_out),
            }),
            Err(_) => None,
        }
    };

    CompileProcess {
        flags,
        pos: Pos {
            line: 1,
            col: 1,
            filename: Some(filename.to_string()),
        },
        cfile: CompileProcessInputFile {
            fp: cfile_fp,
            abs_path: Some(filename.to_string()),
        },
        token_vec: None,
        node_vec: Some(vector_create(std::mem::size_of::<usize>())),
        node_tree_vec: Some(vector_create(std::mem::size_of::<usize>())),
        ofile,
    }
}

/// Reads the next character in the lex process.
pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    let path_opt = lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone());
    let path = match path_opt {
        Some(p) => p,
        None => return '\0',
    };
    if let Some(comp) = lex_process.compiler.as_mut() {
        comp.pos.col += 1;
    }
    let c = buf_next(&path);
    if c == '\n' {
        if let Some(comp) = lex_process.compiler.as_mut() {
            comp.pos.line += 1;
            comp.pos.col = 1;
        }
    }
    c
}

/// Peeks the next character without consuming it in the lex process.
pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    let path_opt = lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone());
    match path_opt {
        Some(p) => buf_peek(&p),
        None => '\0',
    }
}

/// Pushes a character back into the lex process.
pub fn compile_process_push_char(lex_process: &mut LexProcess, _c: char) {
    let path_opt = lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone());
    if let Some(p) = path_opt {
        buf_push(&p);
    }
}

/// Logs a compiler error message.
pub fn compiler_error(compiler: &mut CompileProcess, msg: &str) {
    eprint!("{}", msg);
    eprintln!(
        " on line {}, col {} in file {}",
        compiler.pos.line,
        compiler.pos.col,
        compiler.pos.filename.as_deref().unwrap_or("<unknown>")
    );
    // The C version exits the process here. We avoid that in safe Rust;
    // callers can check return codes.
}

/// Logs a compiler warning message.
pub fn compiler_warning(compiler: &mut CompileProcess, msg: &str) {
    eprint!("{}", msg);
    eprintln!(
        " on line {}, col {} in file {}",
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
    use crate::vector::vector_create;
    LexProcess {
        pos: Pos {
            line: 1,
            col: 1,
            filename: compiler.cfile.abs_path.clone(),
        },
        token_vec: Some(vector_create(std::mem::size_of::<usize>())),
        compiler: Some(Box::new(compiler)),
        current_expression_count: 0,
        parentheses_buffer: None,
        function: Some(functions),
        private,
    }
}

/// Frees resources used by a lex process.
pub fn lex_process_free(_process: LexProcess) {
    // Drops automatically.
}

/// Returns the private data associated with a lex process.
pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}

/// Returns the token vector of a lex process.
pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}

// -----------------------------------------------------------------------------
// Lexer
// -----------------------------------------------------------------------------

fn push_token(lex_process: &mut LexProcess, token: Token) {
    use crate::vector::vector_push;
    let idx = TOKEN_STORAGE.with(|t| {
        let mut t = t.borrow_mut();
        t.push(token);
        t.len() - 1
    });
    if let Some(vec) = lex_process.token_vec.as_mut() {
        let bytes = (idx as usize).to_ne_bytes();
        vector_push(vec, &bytes);
    }
}

fn lexer_last_token(lex_process: &LexProcess) -> Option<Token> {
    if let Some(vec) = lex_process.token_vec.as_ref() {
        if vec.count == 0 {
            return None;
        }
        // The last index pushed
        let off = ((vec.rindex - 1) as usize) * vec.esize;
        if off + vec.esize > vec.data.len() {
            return None;
        }
        let mut idx_bytes = [0u8; std::mem::size_of::<usize>()];
        idx_bytes.copy_from_slice(&vec.data[off..off + std::mem::size_of::<usize>()]);
        let idx = usize::from_ne_bytes(idx_bytes);
        TOKEN_STORAGE.with(|t| t.borrow().get(idx).cloned())
    } else {
        None
    }
}

fn lex_get_pos(lex_process: &LexProcess) -> Pos {
    lex_process.pos.clone()
}

fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

fn is_alpha_or_us(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}

fn is_alnum_or_us(c: char) -> bool {
    is_alpha_or_us(c) || is_digit(c)
}

fn is_keyword_str(s: &str) -> bool {
    matches!(
        s,
        "auto"
            | "break"
            | "case"
            | "char"
            | "const"
            | "continue"
            | "default"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "extern"
            | "float"
            | "for"
            | "goto"
            | "if"
            | "inline"
            | "int"
            | "long"
            | "register"
            | "restrict"
            | "return"
            | "short"
            | "signed"
            | "sizeof"
            | "static"
            | "struct"
            | "switch"
            | "typedef"
            | "union"
            | "unsigned"
            | "void"
            | "volatile"
            | "while"
            | "_Alignas"
            | "_Alignof"
            | "_Atomic"
            | "_Bool"
            | "_Complex"
            | "_Generic"
            | "_Imaginary"
            | "_Noreturn"
            | "_Static_assert"
            | "_Thread_local"
            | "__ignore_typecheck"
    )
}

fn lexer_peek(lex_process: &mut LexProcess) -> char {
    let func = lex_process.function.as_ref().unwrap().peek_char;
    func(lex_process)
}

fn lexer_next(lex_process: &mut LexProcess) -> char {
    let func = lex_process.function.as_ref().unwrap().next_char;
    let c = func(lex_process);
    lex_process.pos.col += 1;
    if c == '\n' {
        lex_process.pos.line += 1;
        lex_process.pos.col = 1;
    }
    c
}

fn read_number_string(lex_process: &mut LexProcess) -> String {
    let mut s = String::new();
    loop {
        let c = lexer_peek(lex_process);
        if !is_digit(c) {
            break;
        }
        s.push(c);
        lexer_next(lex_process);
    }
    s
}

fn read_identifier(lex_process: &mut LexProcess) -> String {
    let mut s = String::new();
    loop {
        let c = lexer_peek(lex_process);
        if !is_alnum_or_us(c) {
            break;
        }
        s.push(c);
        lexer_next(lex_process);
    }
    s
}

fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let s = read_number_string(lex_process);
    let value: u64 = s.parse().unwrap_or(0);
    // Check for L/f suffix.
    let nt = match lexer_peek(lex_process) {
        'L' => {
            lexer_next(lex_process);
            NUMBER_TYPE_LONG
        }
        'f' => {
            lexer_next(lex_process);
            NUMBER_TYPE_FLOAT
        }
        _ => NUMBER_TYPE_NORMAL,
    };
    Token {
        r#type: TOKEN_TYPE_NUMBER,
        llnum: Some(value),
        num: TokenNumber { r#type: nt },
        pos: lex_get_pos(lex_process),
        ..Token::default()
    }
}

fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let s = read_identifier(lex_process);
    let pos = lex_get_pos(lex_process);
    if is_keyword_str(&s) {
        Token {
            r#type: TOKEN_TYPE_KEYWORD,
            sval: Some(s),
            pos,
            ..Token::default()
        }
    } else {
        Token {
            r#type: TOKEN_TYPE_IDENTIFIER,
            sval: Some(s),
            pos,
            ..Token::default()
        }
    }
}

fn token_make_string_lit(lex_process: &mut LexProcess, start: char, end: char) -> Token {
    // Skip the start delimiter.
    let first = lexer_next(lex_process);
    debug_assert_eq!(first, start);
    let mut s = String::new();
    loop {
        let c = lexer_next(lex_process);
        if c == end || c == '\0' {
            break;
        }
        if c == '\\' {
            // Skip the next char and continue.
            continue;
        }
        s.push(c);
    }
    Token {
        r#type: TOKEN_TYPE_STRING,
        sval: Some(s),
        pos: lex_get_pos(lex_process),
        ..Token::default()
    }
}

fn token_make_symbol(lex_process: &mut LexProcess) -> Token {
    let c = lexer_next(lex_process);
    Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some(c),
        pos: lex_get_pos(lex_process),
        ..Token::default()
    }
}

fn token_make_operator(lex_process: &mut LexProcess) -> Token {
    let c = lexer_next(lex_process);
    let mut s = c.to_string();
    // Try to read a second operator char.
    let nxt = lexer_peek(lex_process);
    let is_op2 = matches!(
        nxt,
        '+' | '-' | '*' | '/' | '=' | '>' | '<' | '|' | '&' | '^' | '%' | '~' | '!'
    );
    if is_op2 && c != '(' && c != '[' && c != ',' && c != '.' && c != '?' {
        s.push(nxt);
        lexer_next(lex_process);
    }
    Token {
        r#type: TOKEN_TYPE_OPERATOR,
        sval: Some(s),
        pos: lex_get_pos(lex_process),
        ..Token::default()
    }
}

fn token_make_newline(lex_process: &mut LexProcess) -> Token {
    lexer_next(lex_process);
    Token {
        r#type: TOKEN_TYPE_NEWLINE,
        pos: lex_get_pos(lex_process),
        ..Token::default()
    }
}

fn handle_whitespace(lex_process: &mut LexProcess) {
    // Mark previous token as having trailing whitespace.
    let last_idx_opt = lex_process
        .token_vec
        .as_ref()
        .and_then(|v| {
            if v.count == 0 {
                None
            } else {
                let off = ((v.rindex - 1) as usize) * v.esize;
                let mut idx_bytes = [0u8; std::mem::size_of::<usize>()];
                idx_bytes.copy_from_slice(&v.data[off..off + std::mem::size_of::<usize>()]);
                Some(usize::from_ne_bytes(idx_bytes))
            }
        });
    if let Some(i) = last_idx_opt {
        TOKEN_STORAGE.with(|t| {
            if let Some(tok) = t.borrow_mut().get_mut(i) {
                tok.whitespace = true;
            }
        });
    }
    lexer_next(lex_process);
}

fn read_next_token(lex_process: &mut LexProcess) -> Option<Token> {
    loop {
        let c = lexer_peek(lex_process);
        match c {
            '\0' => return None,
            '$' => return None,
            ' ' | '\t' => {
                handle_whitespace(lex_process);
                continue;
            }
            '\n' => return Some(token_make_newline(lex_process)),
            '"' => return Some(token_make_string_lit(lex_process, '"', '"')),
            c if is_digit(c) => return Some(token_make_number(lex_process)),
            c if is_alpha_or_us(c) => return Some(token_make_identifier_or_keyword(lex_process)),
            '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&' | '(' | '['
            | ',' | '.' | '?' | '/' => return Some(token_make_operator(lex_process)),
            '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => {
                return Some(token_make_symbol(lex_process))
            }
            _ => {
                // Unknown char: skip.
                lexer_next(lex_process);
            }
        }
    }
}

/// Runs the lexical analysis on a lex process.
pub fn lex(process: &mut LexProcess) -> i32 {
    process.current_expression_count = 0;
    process.parentheses_buffer = None;

    // Set position filename.
    if let Some(comp) = process.compiler.as_ref() {
        process.pos.filename = comp.cfile.abs_path.clone();
    }

    while let Some(token) = read_next_token(process) {
        push_token(process, token);
    }
    LEXICAL_ANALYSIS_ALL_OK
}

/// Determines if a token is a keyword matching the given value.
pub fn token_is_keyword(token: &Token, value: &str) -> bool {
    token.r#type == TOKEN_TYPE_KEYWORD
        && match &token.sval {
            Some(s) => s == value,
            None => false,
        }
}

/// Creates a lex process for a string rather than a file, for quick token building.
pub fn tokens_build_for_string(compiler: CompileProcess, s: &str) -> LexProcess {
    // Reuse the file buffer mechanism by storing the string under a unique key.
    let key = format!("__string_buffer__{}", s.as_ptr() as usize);
    FILE_BUFFERS.with(|m| {
        m.borrow_mut()
            .insert(key.clone(), (s.as_bytes().to_vec(), 0));
    });
    let mut compiler = compiler;
    compiler.cfile.abs_path = Some(key.clone());

    let lex_functions = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let mut lp = lex_process_create(compiler, lex_functions, None);
    let _ = lex(&mut lp);
    lp
}

// -----------------------------------------------------------------------------
// Parser
// -----------------------------------------------------------------------------

fn vec_count(v: &Option<Vector>) -> i32 {
    v.as_ref().map(|v| v.count).unwrap_or(0)
}

fn vec_get_index(vec: &Vector, index: i32) -> Option<usize> {
    if index < 0 || index >= vec.count {
        return None;
    }
    let off = (index as usize) * vec.esize;
    if off + vec.esize > vec.data.len() {
        return None;
    }
    let mut idx_bytes = [0u8; std::mem::size_of::<usize>()];
    idx_bytes.copy_from_slice(&vec.data[off..off + std::mem::size_of::<usize>()]);
    Some(usize::from_ne_bytes(idx_bytes))
}

fn token_at(vec: &Vector, index: i32) -> Option<Token> {
    let i = vec_get_index(vec, index)?;
    TOKEN_STORAGE.with(|t| t.borrow().get(i).cloned())
}

fn parser_peek_no_increment(process: &CompileProcess) -> Option<Token> {
    let vec = process.token_vec.as_ref()?;
    token_at(vec, vec.pindex)
}

fn parser_peek_increment(process: &mut CompileProcess) -> Option<Token> {
    let vec = process.token_vec.as_mut()?;
    let tok = token_at(vec, vec.pindex);
    vec.pindex += 1;
    tok
}

fn parser_skip_nl_or_comments(process: &mut CompileProcess) {
    loop {
        match parser_peek_no_increment(process) {
            Some(tok) if token_is_nl_or_comment_or_newline_separator(&tok) => {
                if let Some(vec) = process.token_vec.as_mut() {
                    vec.pindex += 1;
                }
            }
            _ => break,
        }
    }
}

fn parse_single_token_to_node(process: &mut CompileProcess) -> bool {
    parser_skip_nl_or_comments(process);
    let token = match parser_peek_increment(process) {
        Some(t) => t,
        None => return false,
    };
    let node = match token.r#type {
        TOKEN_TYPE_NUMBER => Node {
            r#type: NODE_TYPE_NUMBER,
            llnum: token.llnum,
            ..Node::default()
        },
        TOKEN_TYPE_IDENTIFIER => Node {
            r#type: NODE_TYPE_IDENTIFIER,
            sval: token.sval.clone(),
            ..Node::default()
        },
        TOKEN_TYPE_STRING => Node {
            r#type: NODE_TYPE_STRING,
            sval: token.sval.clone(),
            ..Node::default()
        },
        _ => {
            // Skip unknown tokens.
            return true;
        }
    };
    let inner: crate::node::Node = (&node).into();
    crate::node::node_create(&inner);
    true
}

fn parse_next(process: &mut CompileProcess) -> i32 {
    parser_skip_nl_or_comments(process);
    let token = match parser_peek_no_increment(process) {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        TOKEN_TYPE_NUMBER | TOKEN_TYPE_IDENTIFIER | TOKEN_TYPE_STRING => {
            if !parse_single_token_to_node(process) {
                return -1;
            }
        }
        _ => {
            // Skip and advance.
            if let Some(vec) = process.token_vec.as_mut() {
                vec.pindex += 1;
            }
        }
    }
    0
}

/// Parses the tokens to form an AST.
pub fn parse(process: &mut CompileProcess) -> i32 {
    if let Some(vec) = process.token_vec.as_mut() {
        vec.pindex = 0;
    }
    // Prepare node vectors.
    let nv = process.node_vec.clone().unwrap_or_else(|| {
        crate::vector::vector_create(std::mem::size_of::<usize>())
    });
    let nrv = process.node_tree_vec.clone().unwrap_or_else(|| {
        crate::vector::vector_create(std::mem::size_of::<usize>())
    });
    crate::node::node_set_vector(nv, nrv);

    while parse_next(process) == 0 {
        // Loop until tokens are exhausted.
        if vec_count(&process.token_vec) == 0 {
            break;
        }
        let cur_idx = process.token_vec.as_ref().map(|v| v.pindex).unwrap_or(0);
        if cur_idx >= vec_count(&process.token_vec) {
            break;
        }
    }
    PARSE_ALL_OK
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

/// Pops a node from some global or external node stack (placeholder).
pub fn node_pop() -> Option<Node> {
    crate::node::node_pop_opt().map(|n| Node::from(&n))
}

/// Peeks the top node from a global or external node stack (placeholder).
pub fn node_peek() -> Option<Node> {
    crate::node::node_peek_opt().map(|n| Node::from(&n))
}

/// Peeks the top node from a global or external node stack, returning null if none (placeholder).
pub fn node_peek_or_null() -> Option<Node> {
    crate::node::node_peek_or_null().map(|n| Node::from(&n))
}

/// Pushes a node onto some global or external node stack (placeholder).
pub fn node_push(node: Node) {
    let inner: crate::node::Node = (&node).into();
    crate::node::node_push(&inner);
}

/// Sets the provided vector references in some global or external node context.
pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    crate::node::node_set_vector(vec, root_vec);
}

/// Creates a new node and returns it.
pub fn node_create(node: &Node) -> Node {
    let inner: crate::node::Node = node.into();
    let created = crate::node::node_create(&inner);
    Node::from(&created)
}
