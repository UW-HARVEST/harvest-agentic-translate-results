use std::fmt;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE_MODE: AtomicBool = AtomicBool::new(false);

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

impl Clone for LambdaExpression {
    fn clone(&self) -> Self {
        Self {
            parameter: self.parameter.clone(),
            type_: self.type_.clone(),
            body: self.body.as_ref().map(|body| Box::new((**body).clone())),
        }
    }
}

impl Clone for Application {
    fn clone(&self) -> Self {
        Self {
            function: self.function.as_ref().map(|node| Box::new((**node).clone())),
            argument: self.argument.as_ref().map(|node| Box::new((**node).clone())),
        }
    }
}

impl Clone for Variable {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            type_: self.type_.clone(),
        }
    }
}

impl Clone for AstNodeUnion {
    fn clone(&self) -> Self {
        match self {
            Self::LambdaExpr(expr) => Self::LambdaExpr(expr.clone()),
            Self::Application(app) => Self::Application(app.clone()),
            Self::Variable(var) => Self::Variable(var.clone()),
        }
    }
}

impl Clone for AstNode {
    fn clone(&self) -> Self {
        Self {
            type_: match self.type_ {
                AstNodeType::LAMBDA_EXPR => AstNodeType::LAMBDA_EXPR,
                AstNodeType::APPLICATION => AstNodeType::APPLICATION,
                AstNodeType::VAR => AstNodeType::VAR,
                AstNodeType::DEFINITION => AstNodeType::DEFINITION,
            },
            node: self.node.clone(),
        }
    }
}

impl Default for AstNode {
    fn default() -> Self {
        Self {
            type_: AstNodeType::VAR,
            node: AstNodeUnion::Variable(Variable {
                name: String::new(),
                type_: String::new(),
            }),
        }
    }
}

pub(crate) fn is_placeholder_ast(node: &AstNode) -> bool {
    matches!(
        &node.node,
        AstNodeUnion::Variable(variable)
            if matches!(node.type_, AstNodeType::VAR)
                && variable.name.is_empty()
                && variable.type_.is_empty()
    )
}

pub fn set_verbose(verbose: bool) {
    VERBOSE_MODE.store(verbose, Ordering::Relaxed);
}

pub fn print_ast_verbose(n: &AstNode) {
    if !VERBOSE_MODE.load(Ordering::Relaxed) {
        return;
    }
    println!("{}", ast_to_string(n));
}

pub fn print_verbose(format: &str, args: fmt::Arguments) {
    if !VERBOSE_MODE.load(Ordering::Relaxed) {
        return;
    }

    print!("\n");
    if format.is_empty() {
        print!("{args}");
    } else if let Some(rendered) = args.as_str() {
        print!("{format}{rendered}");
    } else {
        print!("{format}");
        print!("{args}");
    }
}

pub fn error(msg: &str, file: &str, line: i32, func: &str) {
    eprintln!("ERROR: {msg} at {file}:{line} in {func}()");
    process::exit(1);
}

pub fn format(fmt: &str, args: fmt::Arguments) -> String {
    let mut rendered_args = String::new();
    let _ = fmt::write(&mut rendered_args, args);

    if fmt.is_empty() {
        rendered_args
    } else if rendered_args.is_empty() {
        fmt.to_string()
    } else {
        let mut out = String::with_capacity(fmt.len() + rendered_args.len());
        out.push_str(fmt);
        out.push_str(&rendered_args);
        out
    }
}

pub fn append_to_buffer(buffer: &mut String, str: &str) {
    buffer.push_str(str);
}

pub fn append_ast_to_buffer(buffer: &mut String, node: &AstNode) {
    match &node.node {
        AstNodeUnion::LambdaExpr(lambda) if matches!(node.type_, AstNodeType::LAMBDA_EXPR) => {
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
        AstNodeUnion::Application(application)
            if matches!(node.type_, AstNodeType::APPLICATION) =>
        {
            append_to_buffer(buffer, "(");
            if let Some(function) = &application.function {
                append_ast_to_buffer(buffer, function);
            }
            if let Some(argument) = &application.argument {
                append_ast_to_buffer(buffer, argument);
            }
            append_to_buffer(buffer, ") ");
        }
        AstNodeUnion::Variable(variable) if matches!(node.type_, AstNodeType::VAR) => {
            append_to_buffer(buffer, "(");
            append_to_buffer(buffer, &variable.name);
            if !variable.type_.is_empty() {
                append_to_buffer(buffer, " : ");
                append_to_buffer(buffer, &variable.type_);
            }
            append_to_buffer(buffer, ") ");
        }
        AstNodeUnion::Variable(variable) if matches!(node.type_, AstNodeType::DEFINITION) => {
            append_to_buffer(buffer, "(");
            append_to_buffer(buffer, &variable.name);
            append_to_buffer(buffer, ") ");
        }
        _ => append_to_buffer(buffer, "(UNKNOWN) "),
    }
}

pub fn ast_to_string(node: &AstNode) -> String {
    let mut buffer = String::with_capacity(1024);
    append_ast_to_buffer(&mut buffer, node);
    buffer
}
