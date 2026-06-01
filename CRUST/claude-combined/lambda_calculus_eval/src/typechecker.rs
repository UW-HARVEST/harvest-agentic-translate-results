use crate::common;
use crate::common::{AstNode, AstNodeType, AstNodeUnion};

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
        AstNodeType::VAR | AstNodeType::DEFINITION => {
            let type_str = get_type_from_expr(expr);
            create_type(&type_str, "", expr)
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &expr.node {
                let func_type = match &app.function {
                    Some(f) => typecheck(f, env),
                    None => return create_type("", "", expr),
                };
                let arg_type = match &app.argument {
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
        AstNodeType::VAR | AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(v) = &expr.node {
                return v.type_.clone();
            }
            String::new()
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &expr.node {
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
    match n.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &n.node {
                AstNode {
                    type_: AstNodeType::VAR,
                    node: AstNodeUnion::Variable(common::Variable {
                        name: v.name.clone(),
                        type_: v.type_.clone(),
                    }),
                }
            } else {
                AstNode::default()
            }
        }
        AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(v) = &n.node {
                AstNode {
                    type_: AstNodeType::DEFINITION,
                    node: AstNodeUnion::Variable(common::Variable {
                        name: v.name.clone(),
                        type_: v.type_.clone(),
                    }),
                }
            } else {
                AstNode::default()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &n.node {
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(common::LambdaExpression {
                        parameter: le.parameter.clone(),
                        type_: le.type_.clone(),
                        body: le.body.as_ref().map(|b| Box::new(clone_ast(b))),
                    }),
                }
            } else {
                AstNode::default()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &n.node {
                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(common::Application {
                        function: app.function.as_ref().map(|f| Box::new(clone_ast(f))),
                        argument: app.argument.as_ref().map(|a| Box::new(clone_ast(a))),
                    }),
                }
            } else {
                AstNode::default()
            }
        }
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
    let new_env = Box::new(TypeEnv {
        type_,
        next: prev,
    });
    *env = Some(new_env);
}
pub fn lookup_type(env: &TypeEnv, expr: &common::AstNode) -> Type {
    let mut current: Option<&TypeEnv> = Some(env);
    while let Some(e) = current {
        if expr_type_equal(&e.type_, expr) {
            return Type {
                expr: clone_ast(&e.type_.expr),
                type_: e.type_.type_.clone(),
                return_type: e.type_.return_type.clone(),
            };
        }
        current = e.next.as_deref();
    }
    Type {
        expr: AstNode::default(),
        type_: String::new(),
        return_type: String::new(),
    }
}
