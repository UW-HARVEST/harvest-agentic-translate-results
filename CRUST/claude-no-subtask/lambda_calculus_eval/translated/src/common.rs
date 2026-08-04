use std::cell::RefCell;
use std::fmt;

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
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AstNodeType {
LAMBDA_EXPR,
APPLICATION,
VAR,
DEFINITION,
}
#[derive(Debug, Clone)]
pub struct LambdaExpression {
pub parameter: String,
pub type_: String,
pub body: Option<Box<AstNode>>,
}
#[derive(Debug, Clone)]
pub struct Application {
pub function: Option<Box<AstNode>>,
pub argument: Option<Box<AstNode>>,
}
#[derive(Debug, Clone)]
pub struct Variable {
pub name: String,
pub type_: String,
}
#[derive(Debug, Clone)]
pub enum AstNodeUnion {
LambdaExpr(LambdaExpression),
Application(Application),
Variable(Variable),
}
#[derive(Debug, Clone)]
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

thread_local! {
    static VERBOSE_MODE: RefCell<bool> = RefCell::new(false);
}

pub fn set_verbose(verbose: bool) {
    VERBOSE_MODE.with(|v| *v.borrow_mut() = verbose);
}

pub fn print_ast_verbose(n: &AstNode) {
    let verbose = VERBOSE_MODE.with(|v| *v.borrow());
    if !verbose {
        return;
    }
    let s = ast_to_string(n);
    println!("{}", s);
}

pub fn print_verbose(_format: &str, args: fmt::Arguments) {
    let verbose = VERBOSE_MODE.with(|v| *v.borrow());
    if !verbose {
        return;
    }
    println!();
    print!("{}", args);
}

pub fn error(msg: &str, file: &str, line: i32, func: &str) {
    eprintln!("ERROR: {} at {}:{} in {}()", msg, file, line, func);
    std::process::exit(1);
}

pub fn format(_fmt: &str, args: fmt::Arguments) -> String {
    args.to_string()
}

pub fn append_to_buffer(buffer: &mut String, str: &str) {
    buffer.push_str(str);
}

pub fn append_ast_to_buffer(buffer: &mut String, node: &AstNode) {
    match &node.node {
        AstNodeUnion::LambdaExpr(le) => {
            append_to_buffer(buffer, "(@");
            append_to_buffer(buffer, &le.parameter);
            append_to_buffer(buffer, " : ");
            append_to_buffer(buffer, &le.type_);
            append_to_buffer(buffer, " .");
            if let Some(body) = &le.body {
                append_ast_to_buffer(buffer, body);
            }
            append_to_buffer(buffer, ") ");
        }
        AstNodeUnion::Application(app) => {
            append_to_buffer(buffer, "(");
            if let Some(f) = &app.function {
                append_ast_to_buffer(buffer, f);
            }
            if let Some(a) = &app.argument {
                append_ast_to_buffer(buffer, a);
            }
            append_to_buffer(buffer, ") ");
        }
        AstNodeUnion::Variable(v) => {
            match node.type_ {
                AstNodeType::DEFINITION => {
                    append_to_buffer(buffer, "(");
                    append_to_buffer(buffer, &v.name);
                    append_to_buffer(buffer, ") ");
                }
                _ => {
                    append_to_buffer(buffer, "(");
                    append_to_buffer(buffer, &v.name);
                    if !v.type_.is_empty() {
                        append_to_buffer(buffer, " : ");
                        append_to_buffer(buffer, &v.type_);
                    }
                    append_to_buffer(buffer, ") ");
                }
            }
        }
    }
}

pub fn ast_to_string(node: &AstNode) -> String {
    let mut buffer = String::new();
    append_ast_to_buffer(&mut buffer, node);
    buffer
}
