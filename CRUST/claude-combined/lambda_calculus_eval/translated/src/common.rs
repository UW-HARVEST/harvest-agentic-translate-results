use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

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
    VERBOSE.store(verbose, Ordering::SeqCst);
}
pub fn print_ast_verbose(n: &AstNode) {
    if !VERBOSE.load(Ordering::SeqCst) {
        return;
    }
    let s = ast_to_string(n);
    println!("{}", s);
}
pub fn print_verbose(format: &str, _args: fmt::Arguments) {
    if !VERBOSE.load(Ordering::SeqCst) {
        return;
    }
    println!();
    print!("{}", format);
}
pub fn error(msg: &str, file: &str, line: i32, func: &str) {
    eprintln!("ERROR: {} at {}:{} in {}()", msg, file, line, func);
    std::process::exit(1);
}
pub fn format(fmt: &str, _args: fmt::Arguments) -> String {
    fmt.to_string()
}
pub fn append_to_buffer(buffer: &mut String, str: &str) {
    buffer.push_str(str);
}
pub fn append_ast_to_buffer(buffer: &mut String, node: &AstNode) {
    match node.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &node.node {
                buffer.push_str("(@");
                buffer.push_str(&le.parameter);
                buffer.push_str(" : ");
                buffer.push_str(&le.type_);
                buffer.push_str(" .");
                if let Some(body) = &le.body {
                    append_ast_to_buffer(buffer, body);
                }
                buffer.push_str(") ");
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &node.node {
                buffer.push('(');
                if let Some(f) = &app.function {
                    append_ast_to_buffer(buffer, f);
                }
                if let Some(a) = &app.argument {
                    append_ast_to_buffer(buffer, a);
                }
                buffer.push_str(") ");
            }
        }
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &node.node {
                buffer.push('(');
                buffer.push_str(&v.name);
                if !v.type_.is_empty() {
                    buffer.push_str(" : ");
                    buffer.push_str(&v.type_);
                }
                buffer.push_str(") ");
            }
        }
        AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(v) = &node.node {
                buffer.push('(');
                buffer.push_str(&v.name);
                buffer.push_str(") ");
            }
        }
    }
}
pub fn ast_to_string(node: &AstNode) -> String {
    let mut buffer = String::new();
    append_ast_to_buffer(&mut buffer, node);
    buffer
}
