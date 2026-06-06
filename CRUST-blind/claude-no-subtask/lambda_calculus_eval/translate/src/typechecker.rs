use crate::common::{self, AstNode, AstNodeType, AstNodeUnion};
use crate::reducer;

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

pub fn typecheck(expr: &AstNode, env: Option<&TypeEnv>) -> Type {
    // Move env into a local Option<Box<TypeEnv>> so we can extend it as we
    // recurse, mirroring the linked-list mutation pattern from C.
    let mut env_box: Option<Box<TypeEnv>> = env.map(|e| Box::new(deepcopy_env(e)));
    typecheck_inner(expr, &mut env_box)
}

fn deepcopy_env(env: &TypeEnv) -> TypeEnv {
    TypeEnv {
        type_: Type {
            expr: reducer::deepcopy(&env.type_.expr),
            type_: env.type_.type_.clone(),
            return_type: env.type_.return_type.clone(),
        },
        next: env.next.as_deref().map(|n| Box::new(deepcopy_env(n))),
    }
}

fn typecheck_inner(expr: &AstNode, env: &mut Option<Box<TypeEnv>>) -> Type {
    match expr.type_ {
        AstNodeType::VAR => {
            let type_str = get_type_from_expr(expr);
            let t = create_type(&type_str, "", expr);
            add_to_env(env, clone_type(&t));
            t
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &expr.node {
                let func_type = match app.function.as_deref() {
                    Some(f) => typecheck_inner(f, env),
                    None => return create_type("", "", expr),
                };
                let arg_type = match app.argument.as_deref() {
                    Some(a) => typecheck_inner(a, env),
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
            let t = create_type(&type_str, "", expr);
            add_to_env(env, clone_type(&t));
            t
        }
        AstNodeType::DEFINITION => create_type("", "", expr),
    }
}

fn clone_type(t: &Type) -> Type {
    Type {
        expr: reducer::deepcopy(&t.expr),
        type_: t.type_.clone(),
        return_type: t.return_type.clone(),
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
            if let AstNodeUnion::Variable(v) = &expr.node {
                v.type_.clone()
            } else {
                String::new()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(lam) = &expr.node {
                lam.type_.clone()
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
        expr: reducer::deepcopy(expr),
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
    // Compare expression identity by structural equivalence.
    if !ast_structural_eq(&t.expr, expr) {
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

fn ast_structural_eq(a: &AstNode, b: &AstNode) -> bool {
    if a.type_ != b.type_ {
        return false;
    }
    match (&a.node, &b.node) {
        (AstNodeUnion::Variable(va), AstNodeUnion::Variable(vb)) => {
            va.name == vb.name && va.type_ == vb.type_
        }
        (AstNodeUnion::LambdaExpr(la), AstNodeUnion::LambdaExpr(lb)) => {
            la.parameter == lb.parameter
                && la.type_ == lb.type_
                && match (la.body.as_deref(), lb.body.as_deref()) {
                    (None, None) => true,
                    (Some(x), Some(y)) => ast_structural_eq(x, y),
                    _ => false,
                }
        }
        (AstNodeUnion::Application(aa), AstNodeUnion::Application(ab)) => {
            let f_eq = match (aa.function.as_deref(), ab.function.as_deref()) {
                (None, None) => true,
                (Some(x), Some(y)) => ast_structural_eq(x, y),
                _ => false,
            };
            let a_eq = match (aa.argument.as_deref(), ab.argument.as_deref()) {
                (None, None) => true,
                (Some(x), Some(y)) => ast_structural_eq(x, y),
                _ => false,
            };
            f_eq && a_eq
        }
        _ => false,
    }
}

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let old = env.take();
    *env = Some(Box::new(TypeEnv {
        type_,
        next: old,
    }));
}

pub fn lookup_type(env: &TypeEnv, expr: &AstNode) -> Type {
    let mut current: Option<&TypeEnv> = Some(env);
    while let Some(e) = current {
        if expr_type_equal(&e.type_, expr) {
            return clone_type(&e.type_);
        }
        current = e.next.as_deref();
    }
    Type {
        expr: AstNode::default(),
        type_: String::new(),
        return_type: String::new(),
    }
}
