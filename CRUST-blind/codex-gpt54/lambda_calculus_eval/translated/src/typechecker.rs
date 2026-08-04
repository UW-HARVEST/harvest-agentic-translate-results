use crate::{common};
pub struct Type {
pub expr: common::AstNode,
pub type_: String,
pub return_type: String,
}
pub struct TypeEnv {
pub type_: Type,
pub next: Option<Box<TypeEnv>>,
}

impl Clone for Type {
    fn clone(&self) -> Self {
        Self {
            expr: self.expr.clone(),
            type_: self.type_.clone(),
            return_type: self.return_type.clone(),
        }
    }
}

pub fn assert_(expr: bool, error_msg: &str) {
    if !expr {
        common::error(error_msg, file!(), line!() as i32, "assert_");
    }
}
pub fn typecheck(expr: &common::AstNode, env: Option<&TypeEnv>) -> Type {
    let mut env_head: Option<Box<TypeEnv>> = None;
    match (&expr.type_, &expr.node) {
        (common::AstNodeType::VAR, _) => {
            let type_ = get_type_from_expr(expr);
            let t = create_type(&type_, "", expr);
            add_to_env(&mut env_head, t.clone());
            t
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(app)) => {
            let func_type = typecheck(
                app.function
                    .as_ref()
                    .map(|function| function.as_ref())
                    .unwrap_or(expr),
                env,
            );
            let arg_type = typecheck(
                app.argument
                    .as_ref()
                    .map(|argument| argument.as_ref())
                    .unwrap_or(expr),
                env,
            );
            assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
            func_type
        }
        (common::AstNodeType::LAMBDA_EXPR, _) => {
            let type_ = get_type_from_expr(expr);
            let t = create_type(&type_, "", expr);
            add_to_env(&mut env_head, t.clone());
            t
        }
        _ => create_type("", "", expr),
    }
}
pub fn type_equal(a: &Type, b: &Type) -> bool {
    a.type_ == b.type_ && a.return_type == b.return_type
}
pub fn get_type_from_expr(expr: &common::AstNode) -> String {
    match (&expr.type_, &expr.node) {
        (common::AstNodeType::VAR, common::AstNodeUnion::Variable(var))
        | (common::AstNodeType::DEFINITION, common::AstNodeUnion::Variable(var)) => var.type_.clone(),
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(expr)) => expr.type_.clone(),
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
    let parsed_type = parse_function_type(&get_type_from_expr(expr));
    t.expr.type_ == expr.type_
        && common::ast_to_string(&t.expr) == common::ast_to_string(expr)
        && t.type_ == parsed_type.type_
        && t.return_type == parsed_type.return_type
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
    while let Some(scope) = current {
        if expr_type_equal(&scope.type_, expr) {
            return scope.type_.clone();
        }
        current = scope.next.as_deref();
    }
    create_type("", "", expr)
}
