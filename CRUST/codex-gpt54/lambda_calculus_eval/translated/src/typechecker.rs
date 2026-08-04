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

impl Clone for Type {
    fn clone(&self) -> Self {
        Self {
            expr: self.expr.clone(),
            type_: self.type_.clone(),
            return_type: self.return_type.clone(),
        }
    }
}

impl Clone for TypeEnv {
    fn clone(&self) -> Self {
        Self {
            type_: self.type_.clone(),
            next: self.next.as_ref().map(|next| Box::new((**next).clone())),
        }
    }
}

fn clone_env(env: Option<&TypeEnv>) -> Option<Box<TypeEnv>> {
    env.map(|value| Box::new(value.clone()))
}

fn same_expr(a: &common::AstNode, b: &common::AstNode) -> bool {
    common::ast_to_string(a) == common::ast_to_string(b)
}

pub fn assert_(expr: bool, error_msg: &str) {
    if !expr {
        common::error(error_msg, file!(), line!() as i32, "assert_");
    }
}

pub fn typecheck(expr: &common::AstNode, env: Option<&TypeEnv>) -> Type {
    match expr.type_ {
        common::AstNodeType::VAR => {
            let type_ = get_type_from_expr(expr);
            let t = create_type(&type_, "", expr);
            let mut local_env = clone_env(env);
            add_to_env(&mut local_env, t.clone());
            t
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(application) = &expr.node {
                let function = application
                    .function
                    .as_deref()
                    .unwrap_or(&common::AstNode::default())
                    .clone();
                let argument = application
                    .argument
                    .as_deref()
                    .unwrap_or(&common::AstNode::default())
                    .clone();
                let func_type = typecheck(&function, env);
                let arg_type = typecheck(&argument, env);
                assert_(type_equal(&func_type, &arg_type), "Type mismatch.");
                func_type
            } else {
                create_type("", "", expr)
            }
        }
        common::AstNodeType::LAMBDA_EXPR => {
            let type_ = get_type_from_expr(expr);
            let t = create_type(&type_, "", expr);
            let mut local_env = clone_env(env);
            add_to_env(&mut local_env, t.clone());
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
        (common::AstNodeType::VAR, common::AstNodeUnion::Variable(variable)) => {
            variable.type_.clone()
        }
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(lambda)) => {
            lambda.type_.clone()
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
    if !same_expr(&t.expr, expr) {
        return false;
    }

    let type_ = get_type_from_expr(expr);
    let parsed_type = parse_function_type(&type_);
    if t.type_ != parsed_type.type_ {
        return false;
    }

    if parsed_type.return_type.is_empty() {
        return t.return_type.is_empty();
    }

    t.return_type == parsed_type.return_type
}

pub fn add_to_env(env: &mut Option<Box<TypeEnv>>, type_: Type) {
    let next = env.take();
    *env = Some(Box::new(TypeEnv { type_, next }));
}

pub fn lookup_type(env: &TypeEnv, expr: &common::AstNode) -> Type {
    let mut current: Option<&TypeEnv> = Some(env);
    while let Some(node) = current {
        if expr_type_equal(&node.type_, expr) {
            return node.type_.clone();
        }
        current = node.next.as_deref();
    }
    create_type("", "", expr)
}
