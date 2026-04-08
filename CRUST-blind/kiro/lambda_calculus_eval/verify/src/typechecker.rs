use crate::common;

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
    if expr { return; }
    common::error(error_msg, file!(), line!() as i32, "assert_");
}
pub fn typecheck(expr: &common::AstNode, env: Option<&TypeEnv>) -> Type {
    let mut owned_env: Option<Box<TypeEnv>> = None;
    // Rebuild env as owned chain for add_to_env
    // We'll work with a simple approach: just pass through

    match &expr.type_ {
        common::AstNodeType::VAR => {
            let type_str = get_type_from_expr(expr);
            let t = create_type(&type_str, "", expr);
            t
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(ref app) = expr.node {
                let func = app.function.as_ref().unwrap();
                let arg = app.argument.as_ref().unwrap();
                let func_type = typecheck(func, env);
                let arg_type = typecheck(arg, env);
                assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
                func_type
            } else {
                create_type("", "", expr)
            }
        }
        common::AstNodeType::LAMBDA_EXPR => {
            let type_str = get_type_from_expr(expr);
            let t = create_type(&type_str, "", expr);
            t
        }
        _ => create_type("", "", expr),
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
pub fn get_type_from_expr(expr: &common::AstNode) -> String {
    match &expr.node {
        common::AstNodeUnion::Variable(ref var) => var.type_.clone(),
        common::AstNodeUnion::LambdaExpr(ref le) => le.type_.clone(),
        _ => String::new(),
    }
}
pub fn p_print_type(t: &Type) {
    if t.type_.is_empty() && t.return_type.is_empty() {
        println!("(type null)");
        return;
    }
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
        expr: common::AstNode::default(),
        type_: type_.to_string(),
        return_type: String::new(),
    }
}
pub fn expr_type_equal(t: &Type, expr: &common::AstNode) -> bool {
    // In C this compares pointers: t->expr != expr
    // In Rust we compare by structure using ast_to_string as a proxy
    let t_str = common::ast_to_string(&t.expr);
    let e_str = common::ast_to_string(expr);
    if t_str != e_str {
        return false;
    }

    let type_str = get_type_from_expr(expr);
    if type_str.is_empty() {
        return false;
    }

    let parsed_type = parse_function_type(&type_str);

    if t.type_ != parsed_type.type_ {
        return false;
    }

    if parsed_type.return_type.is_empty() {
        if t.return_type.is_empty() { return true; }
        return false;
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
    let mut current: Option<&TypeEnv> = Some(env);
    while let Some(e) = current {
        if expr_type_equal(&e.type_, expr) {
            return create_type(&e.type_.type_, &e.type_.return_type, &e.type_.expr);
        }
        current = e.next.as_deref();
    }
    // Return a default type if not found - matching C's NULL return
    Type {
        expr: common::AstNode::default(),
        type_: String::new(),
        return_type: String::new(),
    }
}
