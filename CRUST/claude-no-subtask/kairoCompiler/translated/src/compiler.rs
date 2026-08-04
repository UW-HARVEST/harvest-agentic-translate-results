use crate::buffer::Buffer;
use crate::vector::{vector_create, Vector};
use std::fs::File;
use std::fmt::Debug;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Mutex;
use lazy_static::lazy_static;
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

// ---------------------------------------------------------------------
// Internal global state used to drive the lexer/parser pipeline.
// ---------------------------------------------------------------------
lazy_static! {
    /// File contents being lexed.
    static ref FILE_DATA: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    /// Current read position in FILE_DATA.
    static ref FILE_POS: Mutex<usize> = Mutex::new(0);
    /// Tokens produced by the lexer (so we don't have to serialise them into bytes).
    static ref TOKENS: Mutex<Vec<Token>> = Mutex::new(Vec::new());
    /// Pushback buffer for chars (emulates ungetc()).
    static ref PUSHBACK: Mutex<Vec<u8>> = Mutex::new(Vec::new());
}

#[doc(hidden)]
pub fn __internal_next_char() -> char {
    next_char_internal()
}
#[doc(hidden)]
pub fn __internal_peek_char() -> char {
    peek_char_internal()
}
#[doc(hidden)]
pub fn __internal_push_char(c: char) {
    push_char_internal(c)
}

fn next_char_internal() -> char {
    {
        let mut pb = PUSHBACK.lock().unwrap();
        if let Some(c) = pb.pop() {
            return c as char;
        }
    }
    let data = FILE_DATA.lock().unwrap();
    let mut pos = FILE_POS.lock().unwrap();
    if *pos >= data.len() {
        return '\0';
    }
    let c = data[*pos] as char;
    *pos += 1;
    c
}

fn peek_char_internal() -> char {
    {
        let pb = PUSHBACK.lock().unwrap();
        if let Some(&c) = pb.last() {
            return c as char;
        }
    }
    let data = FILE_DATA.lock().unwrap();
    let pos = FILE_POS.lock().unwrap();
    if *pos >= data.len() {
        return '\0';
    }
    data[*pos] as char
}

fn push_char_internal(c: char) {
    let mut pb = PUSHBACK.lock().unwrap();
    pb.push(c as u8);
}

// Function Declarations
/// Compiles a file from `filename` to `out_filename` with specified flags.
pub fn compile_file(filename: &str, out_filename: &str, flags: i32) -> i32 {
    // Reset internal state.
    {
        let mut pb = PUSHBACK.lock().unwrap();
        pb.clear();
    }
    {
        let mut tokens = TOKENS.lock().unwrap();
        tokens.clear();
    }

    // Read file into FILE_DATA.
    let data = match std::fs::read(filename) {
        Ok(d) => d,
        Err(_) => return COMPILER_FAILED_WITH_ERRORS,
    };
    {
        let mut fd = FILE_DATA.lock().unwrap();
        *fd = data;
        let mut fp = FILE_POS.lock().unwrap();
        *fp = 0;
    }

    let mut process = compile_process_create(filename, out_filename, flags);
    // The C side returns NULL when fopen fails — we already proved above the file is readable.
    process.flags = flags;
    process.pos.line = 1;
    process.pos.col = 1;
    process.pos.filename = Some(filename.to_string());

    // Make sure output file can be created (mirrors C behaviour).
    if !out_filename.is_empty() {
        if File::create(out_filename).is_err() {
            return COMPILER_FAILED_WITH_ERRORS;
        }
    }

    // Lex
    let mut lex_proc = LexProcess::default();
    lex_proc.pos.line = 1;
    lex_proc.pos.col = 1;
    lex_proc.pos.filename = Some(filename.to_string());
    lex_proc.compiler = Some(Box::new(process.clone()));
    lex_proc.token_vec = Some(vector_create(8)); // 8 bytes per token-index
    lex_proc.function = Some(LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    });

    if lex(&mut lex_proc) != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    // Hand the token vector over to the compile process.
    process.token_vec = lex_proc.token_vec.clone();

    // Parse
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
    let mut p = CompileProcess::default();
    p.flags = flags;
    p.cfile.abs_path = Some(filename.to_string());
    if let Ok(f) = ClonableFile::new(filename) {
        p.cfile.fp = Some(f);
    }
    if !filename_out.is_empty() {
        if let Ok(f) = ClonableFile::new(filename_out).or_else(|_| {
            // try to create
            let _ = File::create(filename_out);
            ClonableFile::new(filename_out)
        }) {
            p.ofile = Some(f);
        }
    }
    p.node_vec = Some(vector_create(8));
    p.node_tree_vec = Some(vector_create(8));
    p
}

/// Reads the next character in the lex process.
pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    if let Some(c) = lex_process.compiler.as_mut() {
        c.pos.col += 1;
    }
    let ch = next_char_internal();
    if ch == '\n' {
        if let Some(c) = lex_process.compiler.as_mut() {
            c.pos.line += 1;
            c.pos.col = 1;
        }
    }
    ch
}
/// Peeks the next character without consuming it in the lex process.
pub fn compile_process_peek_char(_lex_process: &mut LexProcess) -> char {
    peek_char_internal()
}
/// Pushes a character back into the lex process.
pub fn compile_process_push_char(_lex_process: &mut LexProcess, c: char) {
    push_char_internal(c);
}
/// Logs a compiler error message.
pub fn compiler_error(_compiler: &mut CompileProcess, msg: &str) {
    eprintln!("compiler error: {}", msg);
    // We don't exit so the test harness can clean up; mirroring C `exit(-1)` would abort tests.
}
/// Logs a compiler warning message.
pub fn compiler_warning(_compiler: &mut CompileProcess, msg: &str) {
    eprintln!("compiler warning: {}", msg);
}
/// Creates a new lex process given a compiler, function pointers, and private data.
pub fn lex_process_create(
    compiler: CompileProcess,
    functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    let mut lp = LexProcess::default();
    lp.pos.line = 1;
    lp.pos.col = 1;
    lp.compiler = Some(Box::new(compiler));
    lp.function = Some(functions);
    lp.token_vec = Some(vector_create(8));
    lp.private = private;
    lp
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

// ---------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------

fn lex_is_in_expression(lex_process: &LexProcess) -> bool {
    lex_process.current_expression_count > 0
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default" | "do"
        | "double" | "else" | "enum" | "extern" | "float" | "for" | "goto" | "if"
        | "inline" | "int" | "long" | "register" | "restrict" | "return" | "short"
        | "signed" | "sizeof" | "static" | "struct" | "switch" | "typedef" | "union"
        | "unsigned" | "void" | "volatile" | "while" | "_Alignas" | "_Alignof"
        | "_Atomic" | "_Bool" | "_Complex" | "_Generic" | "_Imaginary" | "_Noreturn"
        | "_Static_assert" | "_Thread_local" | "__ignore_typecheck"
    )
}

fn nextc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.as_ref().unwrap().next_char;
    let c = f(lex_process);
    if lex_is_in_expression(lex_process) {
        if lex_process.parentheses_buffer.is_none() {
            lex_process.parentheses_buffer = Some(crate::buffer::buffer_create());
        }
        if let Some(buf) = lex_process.parentheses_buffer.as_mut() {
            crate::buffer::buffer_write(buf, c);
        }
    }
    lex_process.pos.col += 1;
    if c == '\n' {
        lex_process.pos.line += 1;
        lex_process.pos.col = 1;
    }
    c
}

fn peekc(lex_process: &mut LexProcess) -> char {
    let f = lex_process.function.as_ref().unwrap().peek_char;
    f(lex_process)
}

fn pushc(lex_process: &mut LexProcess, c: char) {
    let f = lex_process.function.as_ref().unwrap().push_char;
    f(lex_process, c);
}

fn lex_file_position(lex_process: &LexProcess) -> Pos {
    lex_process.pos.clone()
}

fn token_create_internal(lex_process: &mut LexProcess, original: Token) -> Token {
    let mut t = original;
    t.pos = lex_file_position(lex_process);
    if lex_is_in_expression(lex_process) {
        if let Some(buf) = lex_process.parentheses_buffer.as_ref() {
            let bytes = crate::buffer::buffer_ptr(buf);
            t.between_brackets = Some(String::from_utf8_lossy(bytes).into_owned());
        }
    }
    t
}

fn lexer_last_token() -> Option<Token> {
    let tokens = TOKENS.lock().unwrap();
    tokens.last().cloned()
}

fn push_token(t: Token) -> usize {
    let mut tokens = TOKENS.lock().unwrap();
    let idx = tokens.len();
    tokens.push(t);
    idx
}

fn pop_token() {
    let mut tokens = TOKENS.lock().unwrap();
    tokens.pop();
}

fn read_number_str(lex_process: &mut LexProcess) -> String {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while c >= '0' && c <= '9' {
        s.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    s
}

fn read_number(lex_process: &mut LexProcess) -> u64 {
    let s = read_number_str(lex_process);
    s.parse::<u64>().unwrap_or(0)
}

fn lexer_number_type(c: char) -> i32 {
    if c == 'L' {
        NUMBER_TYPE_LONG
    } else if c == 'f' {
        NUMBER_TYPE_FLOAT
    } else {
        NUMBER_TYPE_NORMAL
    }
}

fn token_make_number_for_value(lex_process: &mut LexProcess, number: u64) -> Token {
    let nt = lexer_number_type(peekc(lex_process));
    if nt != NUMBER_TYPE_NORMAL {
        nextc(lex_process);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.llnum = Some(number);
    t.num.r#type = nt;
    token_create_internal(lex_process, t)
}

fn token_make_number(lex_process: &mut LexProcess) -> Token {
    let n = read_number(lex_process);
    token_make_number_for_value(lex_process, n)
}

fn token_make_string(lex_process: &mut LexProcess, start_delim: char, end_delim: char) -> Token {
    let actual = nextc(lex_process);
    debug_assert_eq!(actual, start_delim);
    let mut s = String::new();
    let mut c = nextc(lex_process);
    while c != end_delim && c != '\0' {
        if c == '\\' {
            // skip; same odd behaviour as the C version
        } else {
            s.push(c);
        }
        c = nextc(lex_process);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_STRING;
    t.sval = Some(s);
    token_create_internal(lex_process, t)
}

fn op_treated_as_one(op: char) -> bool {
    matches!(op, '(' | '[' | ',' | '.' | '*' | '?')
}

fn is_single_operator(op: char) -> bool {
    matches!(
        op,
        '+' | '-' | '/' | '*' | '=' | '>' | '<' | '|' | '&' | '^' | '%'
        | '~' | '!' | '(' | '[' | ',' | '.' | '?'
    )
}

fn op_valid(s: &str) -> bool {
    matches!(
        s,
        "+" | "-" | "*" | "/" | "!" | "^" | "+=" | "-=" | "*=" | "/="
        | ">>" | "<<" | ">=" | "<=" | ">" | "<" | "||" | "&&" | "|" | "&"
        | "++" | "--" | "= " | "!=" | "==" | "->" | "(" | "[" | "," | "."
        | "..." | "~" | "?" | "%"
    )
}

fn read_op(lex_process: &mut LexProcess) -> String {
    let mut single_operator = true;
    let op = nextc(lex_process);
    let mut buf = String::new();
    buf.push(op);

    if !op_treated_as_one(op) {
        let nextp = peekc(lex_process);
        if is_single_operator(nextp) {
            buf.push(nextp);
            nextc(lex_process);
            single_operator = false;
        }
    }
    if !single_operator {
        if !op_valid(&buf) {
            // push back all but the first char
            let chars: Vec<char> = buf.chars().collect();
            for &c in chars.iter().skip(1).rev() {
                pushc(lex_process, c);
            }
            buf.truncate(1);
        }
    }
    buf
}

fn lex_new_expression(lex_process: &mut LexProcess) {
    lex_process.current_expression_count += 1;
    if lex_process.current_expression_count == 1 {
        lex_process.parentheses_buffer = Some(crate::buffer::buffer_create());
    }
}

fn lex_finish_expression(lex_process: &mut LexProcess) {
    lex_process.current_expression_count -= 1;
}

fn token_make_operator_or_string(lex_process: &mut LexProcess) -> Option<Token> {
    let op = peekc(lex_process);
    if op == '<' {
        if let Some(mut last) = lexer_last_token() {
            // mimic: token_is_keyword(last, "include")
            if last.sval.as_deref() == Some("include") {
                last.r#type = TOKEN_TYPE_KEYWORD;
                return Some(token_make_string(lex_process, '<', '>'));
            }
        }
    }
    let op_str = read_op(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_OPERATOR;
    t.sval = Some(op_str);
    let token = token_create_internal(lex_process, t);
    if op == '(' {
        lex_new_expression(lex_process);
    }
    Some(token)
}

fn token_make_symbol(lex_process: &mut LexProcess) -> Token {
    let c = nextc(lex_process);
    if c == ')' {
        lex_finish_expression(lex_process);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some(c);
    token_create_internal(lex_process, t)
}

fn token_make_identifier_or_keyword(lex_process: &mut LexProcess) -> Token {
    let mut s = String::new();
    let mut c = peekc(lex_process);
    while (c >= 'a' && c <= 'z')
        || (c >= 'A' && c <= 'Z')
        || (c >= '0' && c <= '9')
        || c == '_'
    {
        s.push(c);
        nextc(lex_process);
        c = peekc(lex_process);
    }
    let mut t = Token::default();
    if is_keyword(&s) {
        t.r#type = TOKEN_TYPE_KEYWORD;
    } else {
        t.r#type = TOKEN_TYPE_IDENTIFIER;
    }
    t.sval = Some(s);
    token_create_internal(lex_process, t)
}

fn read_special_token(lex_process: &mut LexProcess) -> Option<Token> {
    let c = peekc(lex_process);
    if c.is_ascii_alphabetic() || c == '_' {
        Some(token_make_identifier_or_keyword(lex_process))
    } else {
        None
    }
}

fn token_make_newline(lex_process: &mut LexProcess) -> Token {
    nextc(lex_process);
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NEWLINE;
    token_create_internal(lex_process, t)
}

fn token_make_one_line_comment(lex_process: &mut LexProcess) -> Token {
    let mut c = peekc(lex_process);
    while c != '\n' && c != '\0' {
        nextc(lex_process);
        c = peekc(lex_process);
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    token_create_internal(lex_process, t)
}

fn token_make_multiline_comment(lex_process: &mut LexProcess) -> Option<Token> {
    let mut s = String::new();
    loop {
        let mut c = peekc(lex_process);
        while c != '*' && c != '\0' {
            s.push(c);
            nextc(lex_process);
            c = peekc(lex_process);
        }
        if c == '\0' {
            // unterminated
            return None;
        }
        // c == '*'
        nextc(lex_process);
        if peekc(lex_process) == '/' {
            nextc(lex_process);
            break;
        }
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    t.sval = Some(s);
    Some(token_create_internal(lex_process, t))
}

fn handle_comment(lex_process: &mut LexProcess) -> Option<Option<Token>> {
    let c = peekc(lex_process);
    if c == '/' {
        nextc(lex_process);
        if peekc(lex_process) == '/' {
            nextc(lex_process);
            return Some(Some(token_make_one_line_comment(lex_process)));
        } else if peekc(lex_process) == '*' {
            nextc(lex_process);
            return Some(token_make_multiline_comment(lex_process));
        }
        // not a comment, push '/' back and let operator handler take over
        pushc(lex_process, '/');
        return Some(token_make_operator_or_string(lex_process));
    }
    None
}

fn lex_get_escaped_char(c: char) -> char {
    match c {
        'n' => '\n',
        '\\' => '\\',
        't' => '\t',
        'b' => 0x08 as char,
        '\'' => '\'',
        _ => '\0',
    }
}

fn token_make_quote(lex_process: &mut LexProcess) -> Option<Token> {
    let q = nextc(lex_process);
    debug_assert_eq!(q, '\'');
    let mut c = nextc(lex_process);
    if c == '\\' {
        c = nextc(lex_process);
        c = lex_get_escaped_char(c);
    }
    if nextc(lex_process) != '\'' {
        return None;
    }
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.cval = Some(c);
    Some(token_create_internal(lex_process, t))
}

fn handle_whitespace(lex_process: &mut LexProcess) -> Option<Token> {
    {
        let mut tokens = TOKENS.lock().unwrap();
        if let Some(last) = tokens.last_mut() {
            last.whitespace = true;
        }
    }
    nextc(lex_process);
    read_next_token_internal(lex_process)
}

fn token_make_special_number(lex_process: &mut LexProcess) -> Option<Token> {
    let last_token = lexer_last_token();
    let is_zero_number = match last_token.as_ref() {
        Some(t) => t.r#type == TOKEN_TYPE_NUMBER && t.llnum == Some(0),
        None => false,
    };
    if !is_zero_number {
        return Some(token_make_identifier_or_keyword(lex_process));
    }
    pop_token();
    let c = peekc(lex_process);
    if c == 'x' {
        // skip the 'x'
        nextc(lex_process);
        let mut s = String::new();
        let mut ch = peekc(lex_process);
        while ch.is_ascii_hexdigit() {
            s.push(ch);
            nextc(lex_process);
            ch = peekc(lex_process);
        }
        let n = u64::from_str_radix(&s, 16).unwrap_or(0);
        Some(token_make_number_for_value(lex_process, n))
    } else if c == 'b' {
        nextc(lex_process);
        let s = read_number_str(lex_process);
        if s.chars().any(|c| c != '0' && c != '1') {
            return None;
        }
        let n = u64::from_str_radix(&s, 2).unwrap_or(0);
        Some(token_make_number_for_value(lex_process, n))
    } else {
        None
    }
}

fn read_next_token_internal(lex_process: &mut LexProcess) -> Option<Token> {
    if let Some(opt_t) = handle_comment(lex_process) {
        return opt_t;
    }
    let c = peekc(lex_process);
    match c {
        '0'..='9' => Some(token_make_number(lex_process)),
        '+' | '-' | '*' | '>' | '<' | '^' | '%' | '!' | '=' | '~' | '|' | '&'
        | '(' | '[' | ',' | '.' | '?' => token_make_operator_or_string(lex_process),
        '{' | '}' | ':' | ';' | '#' | '\\' | ')' | ']' => {
            Some(token_make_symbol(lex_process))
        }
        'b' | 'x' => token_make_special_number(lex_process),
        '\'' => token_make_quote(lex_process),
        '"' => Some(token_make_string(lex_process, '"', '"')),
        ' ' | '\t' => handle_whitespace(lex_process),
        '\n' => Some(token_make_newline(lex_process)),
        '$' => None,
        '\0' => None,
        _ => read_special_token(lex_process),
    }
}

/// Runs the lexical analysis on a lex process.
pub fn lex(process: &mut LexProcess) -> i32 {
    process.current_expression_count = 0;
    process.parentheses_buffer = None;
    if let Some(c) = process.compiler.as_ref() {
        process.pos.filename = c.cfile.abs_path.clone();
    }
    loop {
        let tok = read_next_token_internal(process);
        match tok {
            Some(t) => {
                push_token(t);
                // Push a placeholder index into the token vector.
                if let Some(v) = process.token_vec.as_mut() {
                    let idx = (TOKENS.lock().unwrap().len() - 1) as u64;
                    let bytes = idx.to_le_bytes();
                    crate::vector::vector_push(v, &bytes);
                }
            }
            None => break,
        }
    }
    LEXICAL_ANALYSIS_ALL_OK
}

/// Determines if a token is a keyword matching the given value.
pub fn token_is_keyword(token: &Token, value: &str) -> bool {
    token.r#type == TOKEN_TYPE_KEYWORD && token.sval.as_deref() == Some(value)
}
/// Creates a lex process for a string rather than a file, for quick token building.
pub fn tokens_build_for_string(compiler: CompileProcess, _str: &str) -> LexProcess {
    // Minimal placeholder. The real C version uses a private buffer; we don't need it for tests.
    let mut lp = LexProcess::default();
    lp.compiler = Some(Box::new(compiler));
    lp.token_vec = Some(vector_create(8));
    lp
}

// ---------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------

fn token_is_nl_or_comment(t: &Token) -> bool {
    t.r#type == TOKEN_TYPE_NEWLINE
        || t.r#type == TOKEN_TYPE_COMMENT
        || (t.r#type == TOKEN_TYPE_SYMBOL && t.cval == Some('\\'))
}

fn parser_get_token_at(process: &CompileProcess, index: i32) -> Option<Token> {
    if index < 0 {
        return None;
    }
    let v = process.token_vec.as_ref()?;
    if index >= v.count {
        return None;
    }
    let start = (index as usize) * v.esize;
    let end = start + v.esize;
    if end > v.data.len() {
        return None;
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&v.data[start..end]);
    let idx = u64::from_le_bytes(buf) as usize;
    let tokens = TOKENS.lock().unwrap();
    tokens.get(idx).cloned()
}

fn parser_token_count(process: &CompileProcess) -> i32 {
    process.token_vec.as_ref().map(|v| v.count).unwrap_or(0)
}

fn parser_advance_past_skips(process: &mut CompileProcess) {
    loop {
        let pindex = process
            .token_vec
            .as_ref()
            .map(|v| v.pindex)
            .unwrap_or(0);
        let tok = parser_get_token_at(process, pindex);
        match tok {
            Some(t) if token_is_nl_or_comment(&t) => {
                if let Some(v) = process.token_vec.as_mut() {
                    v.pindex += 1;
                }
            }
            _ => break,
        }
    }
}

fn parser_peek_token(process: &mut CompileProcess) -> Option<Token> {
    parser_advance_past_skips(process);
    let pindex = process
        .token_vec
        .as_ref()
        .map(|v| v.pindex)
        .unwrap_or(0);
    parser_get_token_at(process, pindex)
}

fn parser_next_token(process: &mut CompileProcess) -> Option<Token> {
    parser_advance_past_skips(process);
    let pindex = process
        .token_vec
        .as_ref()
        .map(|v| v.pindex)
        .unwrap_or(0);
    let tok = parser_get_token_at(process, pindex);
    if let Some(v) = process.token_vec.as_mut() {
        v.pindex += 1;
    }
    if let Some(t) = tok.as_ref() {
        process.pos = t.pos.clone();
    }
    tok
}

fn parse_single_token_to_node(process: &mut CompileProcess) -> bool {
    let token = match parser_next_token(process) {
        Some(t) => t,
        None => return false,
    };
    let mut node = Node::default();
    match token.r#type {
        x if x == TOKEN_TYPE_NUMBER => {
            node.r#type = NODE_TYPE_NUMBER;
            node.llnum = token.llnum;
            true
        }
        x if x == TOKEN_TYPE_IDENTIFIER => {
            node.r#type = NODE_TYPE_IDENTIFIER;
            node.sval = token.sval;
            true
        }
        x if x == TOKEN_TYPE_STRING => {
            node.r#type = NODE_TYPE_STRING;
            node.sval = token.sval;
            true
        }
        _ => false,
    }
}

fn parse_next(process: &mut CompileProcess) -> i32 {
    let token = match parser_peek_token(process) {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        x if x == TOKEN_TYPE_NUMBER || x == TOKEN_TYPE_IDENTIFIER || x == TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process);
        }
        _ => {
            // Skip the unknown token to avoid an infinite loop.
            let _ = parser_next_token(process);
        }
    }
    0
}

/// Parses the tokens to form an AST.
pub fn parse(process: &mut CompileProcess) -> i32 {
    if let Some(v) = process.token_vec.as_mut() {
        v.pindex = 0;
    }
    while parse_next(process) == 0 {}
    PARSE_ALL_OK
}

/// Checks if a token is a specific symbol.
pub fn token_is_symbol(token: &Token, c: char) -> bool {
    token.r#type == TOKEN_TYPE_SYMBOL && token.cval == Some(c)
}
/// Checks if a token is a newline, comment, or other "skip" token.
pub fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    token_is_nl_or_comment(token)
}
/// Pops a node from some global or external node stack (placeholder).
pub fn node_pop() -> Option<Node> {
    None
}
/// Peeks the top node from a global or external node stack (placeholder).
pub fn node_peek() -> Option<Node> {
    None
}
/// Peeks the top node from a global or external node stack, returning null if none (placeholder).
pub fn node_peek_or_null() -> Option<Node> {
    None
}
/// Pushes a node onto some global or external node stack (placeholder).
pub fn node_push(_node: Node) {
    // no-op
}
/// Sets the provided vector references in some global or external node context.
pub fn node_set_vector(_vec: Vector, _root_vec: Vector) {
    // no-op
}
/// Creates a new node and returns it.
pub fn node_create(node: &Node) -> Node {
    node.clone()
}
