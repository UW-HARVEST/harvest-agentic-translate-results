use crate::common;
use crate::common::{AstNode, AstNodeType, AstNodeUnion};
use crate::reducer::deepcopy;

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
    match &expr.type_ {
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
        AstNodeUnion::Variable(v) => v.type_.clone(),
        AstNodeUnion::LambdaExpr(le) => le.type_.clone(),
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
        expr: deepcopy(expr),
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
    let type_str = get_type_from_expr(expr);
    if type_str.is_empty() { return false; }
    let parsed_type = parse_function_type(&type_str);
    if t.type_ != parsed_type.type_ { return false; }
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
    let mut current = Some(env);
    while let Some(e) = current {
        if expr_type_equal(&e.type_, expr) {
            return create_type(&e.type_.type_, &e.type_.return_type, &e.type_.expr);
        }
        current = e.next.as_deref();
    }
    // Return a default type if not found
    Type {
        expr: AstNode::default(),
        type_: String::new(),
        return_type: String::new(),
    }
}
