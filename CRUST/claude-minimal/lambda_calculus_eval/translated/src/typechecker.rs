use crate::common::{self, AstNode, AstNodeType, AstNodeUnion};

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

pub fn typecheck(expr: &common::AstNode, env: Option<&TypeEnv>) -> Type {
    match expr.type_ {
        AstNodeType::VAR => {
            let type_ = get_type_from_expr(expr);
            create_type(&type_, "", expr)
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref a) = expr.node {
                let func_type = match a.function {
                    Some(ref f) => typecheck(f, env),
                    None => create_type("", "", &AstNode::default()),
                };
                let arg_type = match a.argument {
                    Some(ref ar) => typecheck(ar, env),
                    None => create_type("", "", &AstNode::default()),
                };
                assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
                func_type
            } else {
                create_type("", "", expr)
            }
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
    let return_eql = a.return_type == b.return_type;
    type_eql && return_eql
}

pub fn get_type_from_expr(expr: &common::AstNode) -> String {
    match expr.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(ref v) = expr.node {
                v.type_.clone()
            } else {
                String::new()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref l) = expr.node {
                l.type_.clone()
            } else {
                String::new()
            }
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

pub fn create_type(type_: &str, return_type: &str, expr: &common::AstNode) -> Type {
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

pub fn expr_type_equal(t: &Type, expr: &common::AstNode) -> bool {
    // The C version compares pointer equality (t->expr != expr).
    // We use structural equality via comparing names in the variable case
    // and field comparisons.
    let type_str = get_type_from_expr(expr);
    let parsed_type = parse_function_type(&type_str);
    if t.type_ != parsed_type.type_ {
        return false;
    }
    if parsed_type.return_type.is_empty() {
        return t.return_type.is_empty();
    }
    t.return_type == parsed_type.return_type
}

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let new_env = Box::new(TypeEnv {
        type_,
        next: env.take(),
    });
    *env = Some(new_env);
}

pub fn lookup_type(env: &TypeEnv, expr: &common::AstNode) -> Type {
    let mut cur: Option<&TypeEnv> = Some(env);
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
