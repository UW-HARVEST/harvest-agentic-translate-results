use crate::buffer::{buffer_create, Buffer};
use crate::lex_process as lex_process_mod;
use crate::lexer;
use crate::parser;
use crate::vector::{vector_create, Vector};
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;

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

#[derive(Debug)]
pub struct ClonableFile {
    file: File,
    path: PathBuf,
}

impl ClonableFile {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let file = File::open(&path)?;
        Ok(Self { file, path })
    }

    pub fn create(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .read(true)
            .open(&path)?;
        Ok(Self { file, path })
    }

    pub fn read_char(&mut self) -> io::Result<Option<char>> {
        let mut buf = [0u8; 1];
        match self.file.read(&mut buf)? {
            0 => Ok(None),
            _ => Ok(Some(buf[0] as char)),
        }
    }

    pub fn peek_char(&mut self) -> io::Result<Option<char>> {
        let pos = self.file.stream_position()?;
        let c = self.read_char()?;
        self.file.seek(SeekFrom::Start(pos))?;
        Ok(c)
    }

    pub fn push_char(&mut self) -> io::Result<()> {
        let pos = self.file.stream_position()?;
        if pos > 0 {
            self.file.seek(SeekFrom::Start(pos - 1))?;
        }
        Ok(())
    }

    pub fn path_string(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

impl Clone for ClonableFile {
    fn clone(&self) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .or_else(|_| File::open(&self.path))
            .expect("failed to reopen file");
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

#[derive(Clone, Debug)]
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

fn eof_char() -> char {
    '\0'
}

fn encode_index(idx: u64, element_size: usize) -> Vec<u8> {
    let mut out = vec![0; element_size.max(8)];
    out[..8].copy_from_slice(&idx.to_le_bytes());
    out
}

pub fn compile_file(filename: &str, out_filename: &str, flags: i32) -> i32 {
    let mut process = match crate::cprocess::compile_process_create(filename, out_filename, flags) {
        Some(process) => process,
        None => return COMPILER_FAILED_WITH_ERRORS,
    };

    let mut lex_process = lex_process_mod::lex_process_create(
        process.clone(),
        lexer::COMPILER_LEX_FUNCTIONS,
        None,
    );

    if lexer::lex(&mut lex_process) != LEXICAL_ANALYSIS_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    process.token_vec = lex_process.token_vec.clone();
    if parser::parse(&mut process) != PARSE_ALL_OK {
        return COMPILER_FAILED_WITH_ERRORS;
    }

    COMPILER_FILE_COMPILED_OK
}

pub fn compile_process_create(filename: &str, filename_out: &str, flags: i32) -> CompileProcess {
    crate::cprocess::compile_process_create(filename, filename_out, flags).unwrap_or(CompileProcess {
        flags,
        ..CompileProcess::default()
    })
}

pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    let Some(compiler) = lex_process.compiler.as_mut() else {
        return eof_char();
    };
    let Some(file) = compiler.cfile.fp.as_mut() else {
        return eof_char();
    };

    compiler.pos.col += 1;
    match file.read_char() {
        Ok(Some(c)) => {
            if c == '\n' {
                compiler.pos.line += 1;
                compiler.pos.col = 1;
            }
            c
        }
        _ => eof_char(),
    }
}

pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    let Some(compiler) = lex_process.compiler.as_mut() else {
        return eof_char();
    };
    let Some(file) = compiler.cfile.fp.as_mut() else {
        return eof_char();
    };

    file.peek_char().ok().flatten().unwrap_or(eof_char())
}

pub fn compile_process_push_char(lex_process: &mut LexProcess, _c: char) {
    if let Some(compiler) = lex_process.compiler.as_mut() {
        if let Some(file) = compiler.cfile.fp.as_mut() {
            let _ = file.push_char();
        }
    }
}

pub fn compiler_error(compiler: &mut CompileProcess, msg: &str) {
    eprintln!(
        "{} on line {}, col {} in file {}",
        msg,
        compiler.pos.line,
        compiler.pos.col,
        compiler
            .pos
            .filename
            .clone()
            .or_else(|| compiler.cfile.abs_path.clone())
            .unwrap_or_default()
    );
    compiler.flags = i32::MIN;
}

pub fn compiler_warning(compiler: &mut CompileProcess, msg: &str) {
    eprintln!(
        "{} on line {}, col {} in file {}",
        msg,
        compiler.pos.line,
        compiler.pos.col,
        compiler
            .pos
            .filename
            .clone()
            .or_else(|| compiler.cfile.abs_path.clone())
            .unwrap_or_default()
    );
}

pub fn lex_process_create(
    compiler: CompileProcess,
    functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    LexProcess {
        pos: Pos {
            line: 1,
            col: 1,
            ..Pos::default()
        },
        token_vec: Some(vector_create(std::mem::size_of::<Token>())),
        compiler: Some(Box::new(compiler)),
        current_expression_count: 0,
        parentheses_buffer: None,
        function: Some(functions),
        private,
    }
}

pub fn lex_process_free(_process: LexProcess) {}

pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}

pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}

pub fn lex(process: &mut LexProcess) -> i32 {
    process.current_expression_count = 0;
    process.parentheses_buffer = Some(buffer_create());
    if let Some(compiler) = process.compiler.as_mut() {
        process.pos.filename = compiler.cfile.abs_path.clone();
    }

    loop {
        let c = compile_process_peek_char(process);
        if matches!(c, '\0' | '$') {
            break;
        }

        if c.is_ascii_whitespace() {
            let _ = compile_process_next_char(process);
            if c == '\n' {
                let token = Token {
                    r#type: TOKEN_TYPE_NEWLINE,
                    pos: process.pos.clone(),
                    ..Token::default()
                };
                let idx = lexer::store_token(token);
                if let Some(vec) = process.token_vec.as_mut() {
                    crate::vector::vector_push(vec, &encode_index(idx, std::mem::size_of::<Token>()));
                }
            }
            continue;
        }

        let token = if c.is_ascii_digit() {
            let mut text = String::new();
            while compile_process_peek_char(process).is_ascii_hexdigit() {
                text.push(compile_process_next_char(process));
            }
            let value = text.parse::<u64>().unwrap_or(0);
            Token {
                r#type: TOKEN_TYPE_NUMBER,
                llnum: Some(value),
                pos: process.pos.clone(),
                ..Token::default()
            }
        } else if c.is_ascii_alphabetic() || c == '_' {
            let mut text = String::new();
            while {
                let p = compile_process_peek_char(process);
                p.is_ascii_alphanumeric() || p == '_'
            } {
                text.push(compile_process_next_char(process));
            }
            Token {
                r#type: if is_keyword_text(&text) {
                    TOKEN_TYPE_KEYWORD
                } else {
                    TOKEN_TYPE_IDENTIFIER
                },
                sval: Some(text),
                pos: process.pos.clone(),
                ..Token::default()
            }
        } else if c == '"' {
            let _ = compile_process_next_char(process);
            let mut text = String::new();
            loop {
                let next = compile_process_next_char(process);
                if next == '"' || next == '\0' {
                    break;
                }
                text.push(next);
            }
            Token {
                r#type: TOKEN_TYPE_STRING,
                sval: Some(text),
                pos: process.pos.clone(),
                ..Token::default()
            }
        } else {
            let ch = compile_process_next_char(process);
            Token {
                r#type: if "{}:;#\\)]".contains(ch) { TOKEN_TYPE_SYMBOL } else { TOKEN_TYPE_OPERATOR },
                cval: Some(ch),
                sval: Some(ch.to_string()),
                pos: process.pos.clone(),
                ..Token::default()
            }
        };

        let idx = lexer::store_token(token);
        if let Some(vec) = process.token_vec.as_mut() {
            crate::vector::vector_push(vec, &encode_index(idx, std::mem::size_of::<Token>()));
        }
    }

    LEXICAL_ANALYSIS_ALL_OK
}

fn is_keyword_text(text: &str) -> bool {
    matches!(
        text,
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

pub fn token_is_keyword(token: &Token, value: &str) -> bool {
    token.r#type == TOKEN_TYPE_KEYWORD && token.sval.as_deref() == Some(value)
}

pub fn tokens_build_for_string(compiler: CompileProcess, text: &str) -> LexProcess {
    let mut path = std::env::temp_dir();
    path.push(format!("kairoCompiler-{}.txt", std::process::id()));
    let _ = std::fs::write(&path, text);
    let mut compiler = compiler;
    compiler.cfile.abs_path = Some(path.to_string_lossy().into_owned());
    compiler.cfile.fp = ClonableFile::new(&path).ok();
    let mut process = lex_process_create(
        compiler,
        LexProcessFunctions {
            next_char: compile_process_next_char,
            peek_char: compile_process_peek_char,
            push_char: compile_process_push_char,
        },
        None,
    );
    let _ = lex(&mut process);
    process
}

pub fn parse(process: &mut CompileProcess) -> i32 {
    parser::parse(process)
}

pub fn token_is_symbol(token: &Token, c: char) -> bool {
    token.r#type == TOKEN_TYPE_SYMBOL && token.cval == Some(c)
}

pub fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    token.r#type == TOKEN_TYPE_NEWLINE
        || token.r#type == TOKEN_TYPE_COMMENT
        || token_is_symbol(token, '\\')
}

fn convert_node(node: crate::node::Node) -> Node {
    Node {
        r#type: node.r#type,
        flags: node.flags,
        pos: node.pos,
        binded: NodeBinded::default(),
        cval: node.cval,
        sval: node.sval,
        inum: node.inum,
        lnum: node.lnum,
        llnum: node.llnum,
    }
}

pub fn node_pop() -> Option<Node> {
    Some(convert_node(crate::node::node_pop()))
}

pub fn node_peek() -> Option<Node> {
    Some(convert_node(crate::node::node_peek()))
}

pub fn node_peek_or_null() -> Option<Node> {
    crate::node::node_peek_or_null().map(convert_node)
}

pub fn node_push(node: Node) {
    crate::node::node_push(&crate::node::Node {
        r#type: node.r#type,
        flags: node.flags,
        pos: node.pos,
        binded: crate::node::NodeBinded::default(),
        cval: node.cval,
        sval: node.sval,
        inum: node.inum,
        lnum: node.lnum,
        llnum: node.llnum,
    });
}

pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    crate::node::node_set_vector(vec, root_vec);
}

pub fn node_create(node: &Node) -> Node {
    let created = crate::node::node_create(&crate::node::Node {
        r#type: node.r#type,
        flags: node.flags,
        pos: node.pos.clone(),
        binded: crate::node::NodeBinded::default(),
        cval: node.cval,
        sval: node.sval.clone(),
        inum: node.inum,
        lnum: node.lnum,
        llnum: node.llnum,
    });

    Node {
        r#type: created.r#type,
        flags: created.flags,
        pos: created.pos,
        binded: NodeBinded::default(),
        cval: created.cval,
        sval: created.sval,
        inum: created.inum,
        lnum: created.lnum,
        llnum: created.llnum,
    }
}
