use crate::common;
use crate::reducer::deepcopy;

use common::AstNodeType;

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
    common::error(error_msg, file!(), line!() as i32, "assert_");
}

pub fn typecheck(expr: &common::AstNode, env: Option<&TypeEnv>) -> Type {
    let _ = env;
    match expr.type_ {
        AstNodeType::VAR => {
            let type_str = get_type_from_expr(expr);
            create_type(&type_str, "", expr)
        }
        AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(ref app) = expr.node {
                let func_type = match app.function.as_deref() {
                    Some(f) => typecheck(f, env),
                    None => return create_type("", "", expr),
                };
                let arg_type = match app.argument.as_deref() {
                    Some(a) => typecheck(a, env),
                    None => return create_type("", "", expr),
                };
                assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
                func_type
            } else {
                create_type("", "", expr)
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            let type_str = get_type_from_expr(expr);
            create_type(&type_str, "", expr)
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

pub fn get_type_from_expr(expr: &common::AstNode) -> String {
    match expr.type_ {
        AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(ref v) = expr.node {
                return v.type_.clone();
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(ref le) = expr.node {
                return le.type_.clone();
            }
        }
        _ => {}
    }
    String::new()
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
        expr: deepcopy(expr),
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
    // Structural equivalence: t.expr matches expr by content, and the type
    // matches the type carried in the AST node.
    if !ast_eq(&t.expr, expr) {
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
        return t.return_type.is_empty();
    }
    if t.return_type != parsed_type.return_type {
        return false;
    }
    true
}

fn ast_eq(a: &common::AstNode, b: &common::AstNode) -> bool {
    if a.type_ != b.type_ {
        return false;
    }
    match (&a.node, &b.node) {
        (common::AstNodeUnion::Variable(av), common::AstNodeUnion::Variable(bv)) => {
            av.name == bv.name && av.type_ == bv.type_
        }
        (common::AstNodeUnion::LambdaExpr(al), common::AstNodeUnion::LambdaExpr(bl)) => {
            al.parameter == bl.parameter
                && al.type_ == bl.type_
                && match (al.body.as_deref(), bl.body.as_deref()) {
                    (None, None) => true,
                    (Some(ax), Some(bx)) => ast_eq(ax, bx),
                    _ => false,
                }
        }
        (common::AstNodeUnion::Application(aa), common::AstNodeUnion::Application(ba)) => {
            let f = match (aa.function.as_deref(), ba.function.as_deref()) {
                (None, None) => true,
                (Some(ax), Some(bx)) => ast_eq(ax, bx),
                _ => false,
            };
            let g = match (aa.argument.as_deref(), ba.argument.as_deref()) {
                (None, None) => true,
                (Some(ax), Some(bx)) => ast_eq(ax, bx),
                _ => false,
            };
            f && g
        }
        _ => false,
    }
}

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let old = env.take();
    let new_env = TypeEnv {
        type_,
        next: old,
    };
    *env = Some(Box::new(new_env));
}

pub fn lookup_type(env: &TypeEnv, expr: &common::AstNode) -> Type {
    let mut current: Option<&TypeEnv> = Some(env);
    while let Some(e) = current {
        if expr_type_equal(&e.type_, expr) {
            return Type {
                expr: deepcopy(&e.type_.expr),
                type_: e.type_.type_.clone(),
                return_type: e.type_.return_type.clone(),
            };
        }
        current = e.next.as_deref();
    }
    // Return empty Type as substitute for NULL
    Type {
        expr: common::AstNode::default(),
        type_: String::new(),
        return_type: String::new(),
    }
}
