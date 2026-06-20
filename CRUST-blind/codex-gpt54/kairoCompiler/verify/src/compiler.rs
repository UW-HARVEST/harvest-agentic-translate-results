use crate::buffer::{buffer_create, buffer_peek, buffer_read, buffer_write, Buffer};
use crate::vector::{vector_create, vector_push, Vector};
use std::fmt::Debug;
use std::fs::File;
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

    pub fn from_file(path: impl Into<PathBuf>, file: File) -> Self {
        Self {
            file,
            path: path.into(),
        }
    }

    pub fn next_char(&mut self) -> char {
        let mut buf = [0u8; 1];
        match self.file.read(&mut buf) {
            Ok(1) => buf[0] as char,
            _ => '\0',
        }
    }

    pub fn peek_char(&mut self) -> char {
        let pos = self.file.stream_position().unwrap_or(0);
        let c = self.next_char();
        let _ = self.file.seek(SeekFrom::Start(pos));
        c
    }

    pub fn push_char(&mut self, c: char) {
        let pos = self.file.stream_position().unwrap_or(0);
        if pos > 0 {
            let _ = self.file.seek(SeekFrom::Start(pos - 1));
            let _ = c;
        }
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

#[derive(Debug, Default, Clone)]
pub struct LexProcess {
    pub pos: Pos,
    pub token_vec: Option<Vector>,
    pub compiler: Option<Box<CompileProcess>>,
    pub current_expression_count: i32,
    pub parentheses_buffer: Option<Buffer>,
    pub function: Option<LexProcessFunctions>,
    pub private: Option<()>,
}

fn to_node(node: crate::node::Node) -> Node {
    Node {
        r#type: node.r#type,
        flags: node.flags,
        pos: node.pos,
        binded: NodeBinded {
            owner: node.binded.owner.map(|n| Box::new(to_node(*n))),
            function: node.binded.function.map(|n| Box::new(to_node(*n))),
        },
        cval: node.cval,
        sval: node.sval,
        inum: node.inum,
        lnum: node.lnum,
        llnum: node.llnum,
    }
}

fn from_node(node: &Node) -> crate::node::Node {
    crate::node::Node {
        r#type: node.r#type,
        flags: node.flags,
        pos: node.pos.clone(),
        binded: crate::node::NodeBinded {
            owner: node.binded.owner.clone().map(|n| Box::new(from_node(&n))),
            function: node.binded.function.clone().map(|n| Box::new(from_node(&n))),
        },
        cval: node.cval,
        sval: node.sval.clone(),
        inum: node.inum,
        lnum: node.lnum,
        llnum: node.llnum,
    }
}

pub fn compile_file(filename: &str, out_filename: &str, flags: i32) -> i32 {
    let Some(mut process) = crate::cprocess::compile_process_create(filename, out_filename, flags) else {
        return COMPILER_FAILED_WITH_ERRORS;
    };

    let mut lex_process = crate::lex_process::lex_process_create(
        process.clone(),
        crate::lexer::COMPILER_LEX_FUNCTIONS,
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

pub fn compile_process_create(filename: &str, filename_out: &str, flags: i32) -> CompileProcess {
    crate::cprocess::compile_process_create(filename, filename_out, flags).unwrap_or_default()
}

pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    let Some(compiler) = lex_process.compiler.as_mut() else {
        return '\0';
    };
    let Some(file) = compiler.cfile.fp.as_mut() else {
        return '\0';
    };

    compiler.pos.col += 1;
    let c = file.next_char();
    if c == '\n' {
        compiler.pos.line += 1;
        compiler.pos.col = 1;
    }
    c
}

pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    lex_process
        .compiler
        .as_mut()
        .and_then(|compiler| compiler.cfile.fp.as_mut())
        .map(ClonableFile::peek_char)
        .unwrap_or('\0')
}

pub fn compile_process_push_char(lex_process: &mut LexProcess, c: char) {
    if let Some(file) = lex_process
        .compiler
        .as_mut()
        .and_then(|compiler| compiler.cfile.fp.as_mut())
    {
        file.push_char(c);
    }
}

pub fn compiler_error(compiler: &mut CompileProcess, msg: &str) {
    eprintln!(
        "{} on line {}, col {} in file {}",
        msg,
        compiler.pos.line,
        compiler.pos.col,
        compiler.pos.filename.clone().unwrap_or_default()
    );
}

pub fn compiler_warning(compiler: &mut CompileProcess, msg: &str) {
    eprintln!(
        "{} on line {}, col {} in file {}",
        msg,
        compiler.pos.line,
        compiler.pos.col,
        compiler.pos.filename.clone().unwrap_or_default()
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
            filename: None,
        },
        token_vec: Some(vector_create(8)),
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
    while let Some(ch) = process.function.map(|f| (f.peek_char)(process)) {
        if ch == '\0' {
            break;
        }
        let token = if ch.is_ascii_whitespace() && ch != '\n' {
            (process.function.unwrap().next_char)(process);
            continue;
        } else if ch == '\n' {
            (process.function.unwrap().next_char)(process);
            Token {
                r#type: TOKEN_TYPE_NEWLINE,
                pos: process.pos.clone(),
                ..Token::default()
            }
        } else if ch.is_ascii_digit() {
            let mut s = String::new();
            while let Some(c) = process.function.map(|f| (f.peek_char)(process)) {
                if !c.is_ascii_digit() {
                    break;
                }
                s.push((process.function.unwrap().next_char)(process));
            }
            Token {
                r#type: TOKEN_TYPE_NUMBER,
                llnum: s.parse().ok(),
                pos: process.pos.clone(),
                ..Token::default()
            }
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let mut s = String::new();
            while let Some(c) = process.function.map(|f| (f.peek_char)(process)) {
                if !(c.is_ascii_alphanumeric() || c == '_') {
                    break;
                }
                s.push((process.function.unwrap().next_char)(process));
            }
            Token {
                r#type: if matches!(s.as_str(), "include" | "int" | "return") {
                    TOKEN_TYPE_KEYWORD
                } else {
                    TOKEN_TYPE_IDENTIFIER
                },
                sval: Some(s),
                pos: process.pos.clone(),
                ..Token::default()
            }
        } else if ch == '"' {
            (process.function.unwrap().next_char)(process);
            let mut s = String::new();
            while let Some(c) = process.function.map(|f| (f.next_char)(process)) {
                if c == '"' || c == '\0' {
                    break;
                }
                s.push(c);
            }
            Token {
                r#type: TOKEN_TYPE_STRING,
                sval: Some(s),
                pos: process.pos.clone(),
                ..Token::default()
            }
        } else {
            let c = (process.function.unwrap().next_char)(process);
            Token {
                r#type: TOKEN_TYPE_SYMBOL,
                cval: Some(c),
                pos: process.pos.clone(),
                ..Token::default()
            }
        };

        if let Some(vec) = process.token_vec.as_mut() {
            vector_push(vec, &crate::lexer::store_token(token));
        }
    }

    LEXICAL_ANALYSIS_ALL_OK
}

pub fn token_is_keyword(token: &Token, value: &str) -> bool {
    let mut token = token.clone();
    crate::token::token_is_keyword(&mut token, value)
}

pub fn tokens_build_for_string(compiler: CompileProcess, str_: &str) -> LexProcess {
    let mut buffer = buffer_create();
    for c in str_.chars() {
        buffer_write(&mut buffer, c);
    }
    buffer_write(&mut buffer, '\0');

    fn next_char(process: &mut LexProcess) -> char {
        process
            .parentheses_buffer
            .as_mut()
            .map(buffer_read)
            .unwrap_or('\0')
    }

    fn peek_char(process: &mut LexProcess) -> char {
        process
            .parentheses_buffer
            .as_ref()
            .map(buffer_peek)
            .unwrap_or('\0')
    }

    fn push_char(process: &mut LexProcess, c: char) {
        if let Some(buf) = process.parentheses_buffer.as_mut() {
            buffer_write(buf, c);
        }
    }

    let mut lex_process = lex_process_create(
        compiler,
        LexProcessFunctions {
            next_char,
            peek_char,
            push_char,
        },
        None,
    );
    lex_process.parentheses_buffer = Some(buffer);
    let _ = lex(&mut lex_process);
    lex_process
}

pub fn parse(process: &mut CompileProcess) -> i32 {
    crate::parser::parse(process)
}

pub fn token_is_symbol(token: &Token, c: char) -> bool {
    crate::token::token_is_symbol(token, c)
}

pub fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    crate::token::token_is_nl_or_comment_or_newline_separator(token)
}

pub fn node_pop() -> Option<Node> {
    Some(to_node(crate::node::node_pop()))
}

pub fn node_peek() -> Option<Node> {
    Some(to_node(crate::node::node_peek()))
}

pub fn node_peek_or_null() -> Option<Node> {
    crate::node::node_peek_or_null().map(to_node)
}

pub fn node_push(node: Node) {
    crate::node::node_push(&from_node(&node));
}

pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    crate::node::node_set_vector(vec, root_vec);
}

pub fn node_create(node: &Node) -> Node {
    to_node(crate::node::node_create(&from_node(node)))
}
