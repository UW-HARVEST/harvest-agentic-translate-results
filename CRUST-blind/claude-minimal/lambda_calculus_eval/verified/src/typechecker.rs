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
    common::error(error_msg, file!(), line!() as i32, "assert_");
}

pub fn typecheck(expr: &common::AstNode, env: Option<&TypeEnv>) -> Type {
    match expr.type_ {
        common::AstNodeType::VAR => {
            let type_str = get_type_from_expr(expr);
            create_type(&type_str, "", expr)
        }
        common::AstNodeType::APPLICATION => {
            let (function, argument) = if let common::AstNodeUnion::Application(app) = &expr.node {
                (
                    app.function.as_ref().map(|b| (**b).clone()),
                    app.argument.as_ref().map(|b| (**b).clone()),
                )
            } else {
                return create_type("", "", expr);
            };
            let function = match function {
                Some(f) => f,
                None => return create_type("", "", expr),
            };
            let argument = match argument {
                Some(a) => a,
                None => return create_type("", "", expr),
            };
            let func_type = typecheck(&function, env);
            let arg_type = typecheck(&argument, env);

            assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
            func_type
        }
        common::AstNodeType::LAMBDA_EXPR => {
            let type_str = get_type_from_expr(expr);
            create_type(&type_str, "", expr)
        }
        common::AstNodeType::DEFINITION => create_type("", "", expr),
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
    // Compare expr identity using debug formatting/structural equality
    // The C version compares pointer equality. For Rust we approximate by
    // comparing serialized forms.
    if common::ast_to_string(&t.expr) != common::ast_to_string(expr) {
        return false;
    }

    let type_str = get_type_from_expr(expr);
    if type_str.is_empty() {
        common::error(
            "Null pointer encountered",
            file!(),
            line!() as i32,
            "expr_type_equal",
        );
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
    let next = env.take();
    *env = Some(Box::new(TypeEnv { type_, next }));
}

pub fn lookup_type(env: &TypeEnv, expr: &common::AstNode) -> Type {
    let mut cur: Option<&TypeEnv> = Some(env);
    while let Some(e) = cur {
        if expr_type_equal(&e.type_, expr) {
            return create_type(&e.type_.type_, &e.type_.return_type, &e.type_.expr);
        }
        cur = e.next.as_deref();
    }
    create_type("", "", expr)
}
