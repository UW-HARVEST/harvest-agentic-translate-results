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

pub fn typecheck(expr: &AstNode, env: Option<&TypeEnv>) -> Type {
    match expr.type_ {
        AstNodeType::VAR => {
            let type_ = get_type_from_expr(expr);
            create_type(&type_, "", expr)
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = expr.node {
                let func_type = if let Some(ref f) = app.function {
                    typecheck(f, env)
                } else {
                    return create_type("", "", expr);
                };
                let arg_type = if let Some(ref a) = app.argument {
                    typecheck(a, env)
                } else {
                    return create_type("", "", expr);
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
        AstNodeType::DEFINITION => create_type("", "", expr),
    }
}

pub fn type_equal(a: &Type, b: &Type) -> bool {
    let type_eql = a.type_ == b.type_;

    let return_eql = if a.return_type.is_empty() && b.return_type.is_empty() {
        true
    } else if a.return_type.is_empty() || b.return_type.is_empty() {
        false
    } else {
        a.return_type == b.return_type
    };

    type_eql && return_eql
}

pub fn get_type_from_expr(expr: &AstNode) -> String {
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

pub fn create_type(type_: &str, return_type: &str, expr: &AstNode) -> Type {
    Type {
        expr: crate::reducer::deepcopy(expr),
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
    // C compares pointer addresses; here we compare structurally by name/type
    let same_expr = match (&t.expr.type_, &expr.type_) {
        (AstNodeType::VAR, AstNodeType::VAR) => {
            if let (AstNodeUnion::Variable(ref a), AstNodeUnion::Variable(ref b)) =
                (&t.expr.node, &expr.node)
            {
                a.name == b.name && a.type_ == b.type_
            } else {
                false
            }
        }
        (AstNodeType::LAMBDA_EXPR, AstNodeType::LAMBDA_EXPR) => {
            if let (AstNodeUnion::LambdaExpr(ref a), AstNodeUnion::LambdaExpr(ref b)) =
                (&t.expr.node, &expr.node)
            {
                a.parameter == b.parameter && a.type_ == b.type_
            } else {
                false
            }
        }
        _ => false,
    };
    if !same_expr {
        return false;
    }

    let type_str = get_type_from_expr(expr);
    if type_str.is_empty() {
        eprintln!("ERROR: Null pointer encountered in expr_type_equal");
        std::process::exit(1);
    }

    let parsed_type = parse_function_type(&type_str);

    if t.type_ != parsed_type.type_ {
        return false;
    }

    if parsed_type.return_type.is_empty() {
        return t.return_type.is_empty();
    }

    if t.return_type != parsed_type.return_type {
        return false;
    }

    true
}

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let new_env = TypeEnv {
        type_,
        next: env.take(),
    };
    *env = Some(Box::new(new_env));
}

pub fn lookup_type(env: &TypeEnv, expr: &AstNode) -> Type {
    let mut current = Some(env);
    while let Some(e) = current {
        if expr_type_equal(&e.type_, expr) {
            return create_type(&e.type_.type_, &e.type_.return_type, &e.type_.expr);
        }
        current = e.next.as_deref();
    }
    create_type("", "", expr)
}
