use crate::common::{self, AstNode, AstNodeType, AstNodeUnion};

#[derive(Debug)]
pub struct Type {
    pub expr: common::AstNode,
    pub type_: String,
    pub return_type: String,
}

pub struct TypeEnv {
    pub type_: Type,
    pub next: Option<Box<TypeEnv>>,
}

pub fn assert_(expr: bool, error_msg: &str) {
    if expr {
        return;
    }
    eprintln!("ERROR: {}", error_msg);
    std::process::exit(1);
}

pub fn typecheck(expr: &AstNode, env: Option<&TypeEnv>) -> Type {
    match expr.type_ {
        AstNodeType::VAR => {
            let type_ = get_type_from_expr(expr);
            let t = create_type(&type_, "", expr);
            // env update is conceptually owned; ignore here as in C the local env reassign is a no-op for caller
            let _ = env;
            t
        }
        AstNodeType::APPLICATION => {
            let (f, a) = match &expr.node {
                AstNodeUnion::Application(app) => {
                    let f = app.function.as_deref().cloned().unwrap_or_default();
                    let a = app.argument.as_deref().cloned().unwrap_or_default();
                    (f, a)
                }
                _ => return create_type("", "", expr),
            };
            let func_type = typecheck(&f, env);
            let arg_type = typecheck(&a, env);
            assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
            func_type
        }
        AstNodeType::LAMBDA_EXPR => {
            let type_ = get_type_from_expr(expr);
            create_type(&type_, "", expr)
        }
        _ => create_type("", "", expr),
    }
}

pub fn type_equal(a: &Type, b: &Type) -> bool {
    let type_eql = a.type_ == b.type_;
    // We use empty strings to represent NULL return types (matching the C
    // tests which pass NULL/empty string).
    let return_eql = a.return_type == b.return_type;
    type_eql && return_eql
}

pub fn get_type_from_expr(expr: &AstNode) -> String {
    match expr.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &expr.node {
                return v.type_.clone();
            }
            String::new()
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &expr.node {
                return le.type_.clone();
            }
            String::new()
        }
        _ => String::new(),
    }
}

pub fn p_print_type(t: &Type) {
    if !t.type_.is_empty() {
        println!("Type: {}", t.type_);
    }
    if !t.return_type.is_empty() {
        println!("Return type: {}", t.return_type);
    }
}

pub fn create_type(type_: &str, return_type: &str, expr: &AstNode) -> Type {
    Type {
        expr: expr.clone(),
        type_: type_.to_string(),
        return_type: return_type.to_string(),
    }
}

pub fn parse_function_type(type_: &str) -> Type {
    Type {
        expr: AstNode::default(),
        type_: type_.to_string(),
        return_type: String::new(),
    }
}

pub fn expr_type_equal(t: &Type, expr: &AstNode) -> bool {
    let type_str = get_type_from_expr(expr);
    let parsed_type = parse_function_type(&type_str);

    if t.type_ != parsed_type.type_ {
        return false;
    }
    if parsed_type.return_type.is_empty() {
        if t.return_type.is_empty() {
            return true;
        }
        return false;
    }
    if t.return_type != parsed_type.return_type {
        return false;
    }
    true
}

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let prev = env.take();
    *env = Some(Box::new(TypeEnv {
        type_,
        next: prev,
    }));
}

pub fn lookup_type(env: &TypeEnv, expr: &AstNode) -> Type {
    let mut cur = Some(env);
    while let Some(e) = cur {
        if expr_type_equal(&e.type_, expr) {
            return Type {
                expr: e.type_.expr.clone(),
                type_: e.type_.type_.clone(),
                return_type: e.type_.return_type.clone(),
            };
        }
        cur = e.next.as_deref();
    }
    Type {
        expr: AstNode::default(),
        type_: String::new(),
        return_type: String::new(),
    }
}
