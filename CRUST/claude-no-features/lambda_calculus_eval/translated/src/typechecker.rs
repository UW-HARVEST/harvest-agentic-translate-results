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
            let t = create_type(&type_, "", expr);
            // env extension would happen here if we had mutable env
            let _ = env;
            t
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &expr.node {
                let func_type = typecheck(
                    app.function.as_ref().expect("application has no function"),
                    env,
                );
                let arg_type = typecheck(
                    app.argument.as_ref().expect("application has no argument"),
                    env,
                );
                assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
                return func_type;
            }
            create_type("", "", expr)
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

pub fn get_type_from_expr(expr: &common::AstNode) -> String {
    match expr.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &expr.node {
                v.type_.clone()
            } else {
                String::new()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &expr.node {
                le.type_.clone()
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

pub fn expr_type_equal(t: &Type, expr: &common::AstNode) -> bool {
    // Mimic C: t->expr != expr is pointer comparison.
    // In Rust, we'll consider them equal if the type and structure align.
    // First check structural equality of expressions.
    if !ast_struct_equal(&t.expr, expr) {
        return false;
    }

    let type_str = get_type_from_expr(expr);
    if type_str.is_empty() {
        eprintln!("ERROR: Null pointer encountered: type was empty");
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

fn ast_struct_equal(a: &AstNode, b: &AstNode) -> bool {
    if a.type_ != b.type_ {
        return false;
    }
    match (&a.node, &b.node) {
        (AstNodeUnion::Variable(va), AstNodeUnion::Variable(vb)) => {
            va.name == vb.name && va.type_ == vb.type_
        }
        (AstNodeUnion::LambdaExpr(la), AstNodeUnion::LambdaExpr(lb)) => {
            if la.parameter != lb.parameter || la.type_ != lb.type_ {
                return false;
            }
            match (&la.body, &lb.body) {
                (Some(ba), Some(bb)) => ast_struct_equal(ba, bb),
                (None, None) => true,
                _ => false,
            }
        }
        (AstNodeUnion::Application(aa), AstNodeUnion::Application(ab)) => {
            let func_eq = match (&aa.function, &ab.function) {
                (Some(fa), Some(fb)) => ast_struct_equal(fa, fb),
                (None, None) => true,
                _ => false,
            };
            let arg_eq = match (&aa.argument, &ab.argument) {
                (Some(ga), Some(gb)) => ast_struct_equal(ga, gb),
                (None, None) => true,
                _ => false,
            };
            func_eq && arg_eq
        }
        _ => false,
    }
}

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let prev = env.take();
    *env = Some(Box::new(TypeEnv {
        type_,
        next: prev,
    }));
}

pub fn lookup_type(env: &TypeEnv, expr: &common::AstNode) -> Type {
    let mut cur: Option<&TypeEnv> = Some(env);
    while let Some(e) = cur {
        if expr_type_equal(&e.type_, expr) {
            return Type {
                expr: crate::reducer::deepcopy(&e.type_.expr),
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
