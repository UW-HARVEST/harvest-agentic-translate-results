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
    if expr {
        return;
    }
    eprintln!("ERROR: {}", error_msg);
    std::process::exit(1);
}

pub fn typecheck(expr: &common::AstNode, env: Option<&TypeEnv>) -> Type {
    let mut local_env: Option<Box<TypeEnv>> = None;
    if let Some(e) = env {
        // Build a copy of the env chain to avoid lifetime issues. Since
        // we don't actually use the env across recursive calls beyond the
        // newly added type, we skip a deep clone and just initialize empty.
        let _ = e;
    }
    typecheck_inner(expr, &mut local_env)
}

fn typecheck_inner(
    expr: &common::AstNode,
    env: &mut Option<Box<TypeEnv>>,
) -> Type {
    match expr.type_ {
        common::AstNodeType::VAR => {
            let type_str = get_type_from_expr(expr);
            let t = create_type(&type_str, "", expr);
            let t_for_env = create_type(&type_str, "", expr);
            add_to_env(env, t_for_env);
            t
        }
        common::AstNodeType::APPLICATION => {
            let (f_node, a_node) = match &expr.node {
                common::AstNodeUnion::Application(app) => {
                    let f = match &app.function {
                        Some(b) => b.as_ref(),
                        None => &expr_default(),
                    };
                    let a = match &app.argument {
                        Some(b) => b.as_ref(),
                        None => &expr_default(),
                    };
                    // Borrowing inside match makes this tricky; use clones from below
                    let _ = (f, a);
                    (
                        app.function.as_ref().map(|b| b.as_ref()),
                        app.argument.as_ref().map(|b| b.as_ref()),
                    )
                }
                _ => (None, None),
            };
            let default = common::AstNode::default();
            let f_node = f_node.unwrap_or(&default);
            let a_node = a_node.unwrap_or(&default);
            let func_type = typecheck_inner(f_node, env);
            let arg_type = typecheck_inner(a_node, env);
            assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
            func_type
        }
        common::AstNodeType::LAMBDA_EXPR => {
            let type_str = get_type_from_expr(expr);
            let t = create_type(&type_str, "", expr);
            let t_for_env = create_type(&type_str, "", expr);
            add_to_env(env, t_for_env);
            t
        }
        _ => Type {
            expr: common::AstNode::default(),
            type_: String::new(),
            return_type: String::new(),
        },
    }
}

fn expr_default() -> common::AstNode {
    common::AstNode::default()
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
        common::AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(v) = &expr.node {
                return v.type_.clone();
            }
            String::new()
        }
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &expr.node {
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

pub fn create_type(type_: &str, return_type: &str, expr: &common::AstNode) -> Type {
    Type {
        expr: clone_ast(expr),
        type_: type_.to_string(),
        return_type: return_type.to_string(),
    }
}

fn clone_ast(n: &common::AstNode) -> common::AstNode {
    crate::reducer::deepcopy(n)
}

pub fn parse_function_type(type_: &str) -> Type {
    Type {
        expr: common::AstNode::default(),
        type_: type_.to_string(),
        return_type: String::new(),
    }
}

pub fn expr_type_equal(t: &Type, expr: &common::AstNode) -> bool {
    // C-version compares pointer equality (`t->expr != expr`). Since Rust
    // owns its values, compare structurally via the type fields.
    let type_str = get_type_from_expr(expr);
    if type_str.is_empty() {
        // null type from expr
        // C: HANDLE_NULL aborts; we match that with an early false return.
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

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let prev = env.take();
    let new_env = TypeEnv {
        type_,
        next: prev,
    };
    *env = Some(Box::new(new_env));
}

pub fn lookup_type(env: &TypeEnv, expr: &common::AstNode) -> Type {
    let mut cur = Some(env);
    while let Some(node) = cur {
        if expr_type_equal(&node.type_, expr) {
            return Type {
                expr: clone_ast(&node.type_.expr),
                type_: node.type_.type_.clone(),
                return_type: node.type_.return_type.clone(),
            };
        }
        cur = node.next.as_deref();
    }
    Type {
        expr: common::AstNode::default(),
        type_: String::new(),
        return_type: String::new(),
    }
}
