use std::cell::Cell;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum tokens_t {
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

thread_local! {
    static VERBOSE_MODE: Cell<bool> = Cell::new(false);
}

pub fn set_verbose(verbose: bool) {
    VERBOSE_MODE.with(|v| v.set(verbose));
}

pub(crate) fn is_verbose() -> bool {
    VERBOSE_MODE.with(|v| v.get())
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
    // Mirror the C version which calls vprintf(format, args). We attempt
    // to first render the supplied arguments; if the caller did not provide
    // any (Arguments::new_const("") is default) we just print the format.
    let rendered = std::fmt::format(args);
    if rendered.is_empty() {
        print!("{}", format);
    } else {
        print!("{}", rendered);
    }
}

pub fn error(msg: &str, file: &str, line: i32, func: &str) {
    eprintln!("ERROR: {} at {}:{} in {}()", msg, file, line, func);
    std::process::exit(1);
}

pub fn format(fmt: &str, args: fmt::Arguments) -> String {
    let rendered = std::fmt::format(args);
    if rendered.is_empty() {
        fmt.to_string()
    } else {
        rendered
    }
}

pub fn append_to_buffer(buffer: &mut String, str: &str) {
    buffer.push_str(str);
}

pub fn append_ast_to_buffer(buffer: &mut String, node: &AstNode) {
    match &node.node {
        AstNodeUnion::LambdaExpr(lambda) => {
            append_to_buffer(buffer, "(@");
            append_to_buffer(buffer, &lambda.parameter);
            append_to_buffer(buffer, " : ");
            append_to_buffer(buffer, &lambda.type_);
            append_to_buffer(buffer, " .");
            if let Some(body) = &lambda.body {
                append_ast_to_buffer(buffer, body);
            }
            append_to_buffer(buffer, ") ");
        }
        AstNodeUnion::Application(app) => {
            // Application
            append_to_buffer(buffer, "(");
            if let Some(f) = &app.function {
                append_ast_to_buffer(buffer, f);
            }
            if let Some(a) = &app.argument {
                append_ast_to_buffer(buffer, a);
            }
            append_to_buffer(buffer, ") ");
        }
        AstNodeUnion::Variable(var) => {
            match node.type_ {
                AstNodeType::DEFINITION => {
                    append_to_buffer(buffer, "(");
                    append_to_buffer(buffer, &var.name);
                    append_to_buffer(buffer, ") ");
                }
                _ => {
                    append_to_buffer(buffer, "(");
                    append_to_buffer(buffer, &var.name);
                    if !var.type_.is_empty() {
                        append_to_buffer(buffer, " : ");
                        append_to_buffer(buffer, &var.type_);
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
