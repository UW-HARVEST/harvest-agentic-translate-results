use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE_MODE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, PartialEq, Eq)]
pub enum tokens_t{
    L_PAREN,
    R_PAREN,
    LAMBDA,
    DOT,
    VARIABLE,
    ERROR,
    WHITESPACE,
    NEWLINE,
    EQ,
    QUOTE,
    COLON,
}
#[derive(Debug, PartialEq, Eq)]
pub enum AstNodeType {
LAMBDA_EXPR,
APPLICATION,
VAR,
DEFINITION,
}
#[derive(Debug)]
pub struct LambdaExpression {
pub parameter: String,
pub type_: String,
pub body: Option<Box<AstNode>>,
}
#[derive(Debug)]
pub struct Application {
pub function: Option<Box<AstNode>>,
pub argument: Option<Box<AstNode>>,
}
#[derive(Debug)]
pub struct Variable {
pub name: String,
pub type_: String,
}
#[derive(Debug)]
pub enum AstNodeUnion {
LambdaExpr(LambdaExpression),
Application(Application),
Variable(Variable),
}
#[derive(Debug)]
pub struct AstNode {
pub type_: AstNodeType,
pub node: AstNodeUnion,
}
impl Default for AstNode {
    fn default() -> Self {
        AstNode {
            type_: AstNodeType::VAR,
            node: AstNodeUnion::Variable(Variable {
                name: String::new(),
                type_: String::new(),
            }),
        }
    }
}
pub fn set_verbose(verbose: bool) {
    VERBOSE_MODE.store(verbose, Ordering::SeqCst);
}

pub fn is_verbose() -> bool {
    VERBOSE_MODE.load(Ordering::SeqCst)
}

pub fn print_ast_verbose(n: &AstNode) {
    if !is_verbose() {
        return;
    }
    let lambda_ast = ast_to_string(n);
    println!("{}", lambda_ast);
}
pub fn print_verbose(format: &str, args: fmt::Arguments) {
    if !is_verbose() {
        return;
    }
    println!();
    let s = format_args_to_string(format, args);
    print!("{}", s);
}
pub fn error(msg: &str, file: &str, line: i32, func: &str) {
    eprintln!("ERROR: {} at {}:{} in {}()", msg, file, line, func);
    std::process::exit(1);
}
pub fn format(fmt: &str, args: fmt::Arguments) -> String {
    format_args_to_string(fmt, args)
}

fn format_args_to_string(_fmt: &str, args: fmt::Arguments) -> String {
    // The format-string handling here is simplified. The Rust call sites
    // can pass fully-formatted Arguments via format_args! and we just stringify those.
    format!("{}", args)
}

pub fn append_to_buffer(buffer: &mut String, str: &str) {
    buffer.push_str(str);
}
pub fn append_ast_to_buffer(buffer: &mut String, node: &AstNode) {
    match node.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref lambda) = node.node {
                append_to_buffer(buffer, "(@");
                append_to_buffer(buffer, &lambda.parameter);
                append_to_buffer(buffer, " : ");
                append_to_buffer(buffer, &lambda.type_);
                append_to_buffer(buffer, " .");
                if let Some(ref body) = lambda.body {
                    append_ast_to_buffer(buffer, body);
                }
                append_to_buffer(buffer, ") ");
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = node.node {
                append_to_buffer(buffer, "(");
                if let Some(ref f) = app.function {
                    append_ast_to_buffer(buffer, f);
                }
                if let Some(ref a) = app.argument {
                    append_ast_to_buffer(buffer, a);
                }
                append_to_buffer(buffer, ") ");
            }
        }
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(ref v) = node.node {
                append_to_buffer(buffer, "(");
                append_to_buffer(buffer, &v.name);
                if !v.type_.is_empty() {
                    append_to_buffer(buffer, " : ");
                    append_to_buffer(buffer, &v.type_);
                }
                append_to_buffer(buffer, ") ");
            }
        }
        AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(ref v) = node.node {
                append_to_buffer(buffer, "(");
                append_to_buffer(buffer, &v.name);
                append_to_buffer(buffer, ") ");
            }
        }
    }
}
pub fn ast_to_string(node: &AstNode) -> String {
    let mut buffer = String::new();
    append_ast_to_buffer(&mut buffer, node);
    buffer
}
