use crate::common;
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

pub fn typecheck(expr: &common::AstNode, env: Option<&TypeEnv>) -> Type {
    match expr.type_ {
        common::AstNodeType::VAR => {
            let type_str = get_type_from_expr(expr);
            create_type(&type_str, "", expr)
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &expr.node {
                let function_default = common::AstNode::default();
                let argument_default = common::AstNode::default();
                let func_ref =
                    app.function.as_deref().unwrap_or(&function_default);
                let arg_ref =
                    app.argument.as_deref().unwrap_or(&argument_default);
                let func_type = typecheck(func_ref, env);
                let arg_type = typecheck(arg_ref, env);
                assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
                func_type
            } else {
                create_type("", "", expr)
            }
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
    let return_eql = a.return_type == b.return_type;
    type_eql && return_eql
}

pub fn get_type_from_expr(expr: &common::AstNode) -> String {
    match &expr.node {
        common::AstNodeUnion::Variable(v) => v.type_.clone(),
        common::AstNodeUnion::LambdaExpr(l) => l.type_.clone(),
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

pub fn create_type(
    type_: &str,
    return_type: &str,
    expr: &common::AstNode,
) -> Type {
    Type {
        type_: type_.to_string(),
        return_type: return_type.to_string(),
        expr: reducer::deepcopy(expr),
    }
}

pub fn parse_function_type(type_: &str) -> Type {
    Type {
        type_: type_.to_string(),
        return_type: String::new(),
        expr: common::AstNode::default(),
    }
}

pub fn expr_type_equal(t: &Type, expr: &common::AstNode) -> bool {
    // The C version compares pointer identity for `t->expr != expr`. Since we
    // can't compare pointers in safe Rust, we compare structurally on the
    // string representation.
    if common::ast_to_string(&t.expr) != common::ast_to_string(expr) {
        return false;
    }

    let type_str = get_type_from_expr(expr);
    if type_str.is_empty() {
        // The C version aborts on null type; replicate the abort.
        common::error(
            "ERROR: Null pointer encountered",
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

    t.return_type == parsed_type.return_type
}

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let prev = env.take();
    let new_env = Box::new(TypeEnv {
        type_,
        next: prev,
    });
    *env = Some(new_env);
}

pub fn lookup_type(env: &TypeEnv, expr: &common::AstNode) -> Type {
    let mut current: Option<&TypeEnv> = Some(env);
    while let Some(node) = current {
        if expr_type_equal(&node.type_, expr) {
            return create_type(
                &node.type_.type_,
                &node.type_.return_type,
                &node.type_.expr,
            );
        }
        current = node.next.as_deref();
    }
    create_type("", "", expr)
}
