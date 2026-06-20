use crate::{common, hash_table, parser, config};
use std::sync::{Mutex, OnceLock};
pub const SIZE: usize = 122;

fn reduction_order_state() -> &'static Mutex<config::reduction_order_t> {
    static REDUCTION_ORDER: OnceLock<Mutex<config::reduction_order_t>> = OnceLock::new();
    REDUCTION_ORDER.get_or_init(|| Mutex::new(config::reduction_order_t::APPLICATIVE))
}

fn clone_reduction_order(t: &config::reduction_order_t) -> config::reduction_order_t {
    match t {
        config::reduction_order_t::APPLICATIVE => config::reduction_order_t::APPLICATIVE,
        config::reduction_order_t::NORMAL => config::reduction_order_t::NORMAL,
    }
}

fn current_reduction_order() -> config::reduction_order_t {
    clone_reduction_order(&reduction_order_state().lock().expect("reduction order poisoned"))
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    *reduction_order_state().lock().expect("reduction order poisoned") = t;
}
pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => println!("Applicative"),
        config::reduction_order_t::NORMAL => println!("Normal"),
    }
}
pub fn reduce(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    common::print_verbose("", format_args!("Order of reduction is: "));
    print_reduction_order(current_reduction_order());
    common::print_verbose("", format_args!("-------------------------------------------\n"));

    let mut expanded = n.clone();
    expand_definitions_mut(table, &mut expanded);
    common::print_verbose("", format_args!("Expanded expression:\n"));
    common::print_ast_verbose(&expanded);

    let reduced = reduce_ast(table, &expanded);
    common::print_verbose("", format_args!("Final reduced expression:\n"));
    common::print_ast_verbose(&reduced);
    common::print_verbose("", format_args!("-------------------------------------------\n"));
    reduced
}
pub fn expand_definitions(table: &mut hash_table::HashTable, n: &common::AstNode) {
    let mut cloned = n.clone();
    expand_definitions_mut(table, &mut cloned);
}

fn expand_definitions_mut(table: &mut hash_table::HashTable, n: &mut common::AstNode) {
    match (&n.type_, &mut n.node) {
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(expr)) => {
            if let Some(body) = expr.body.as_mut() {
                expand_definitions_mut(table, body);
            }
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(app)) => {
            if let Some(function) = app.function.as_mut() {
                expand_definitions_mut(table, function);
            }
            if let Some(argument) = app.argument.as_mut() {
                expand_definitions_mut(table, argument);
            }
        }
        (common::AstNodeType::DEFINITION, common::AstNodeUnion::Variable(var)) => {
            let def_name = var.name.clone();
            let expanded_def = table.search(&def_name).cloned().unwrap_or_else(|| {
                common::error("Null pointer encountered", file!(), line!() as i32, "expand_definitions");
                unreachable!()
            });
            common::print_verbose(
                "",
                format_args!("Expanding definition of: {} . Term expanded to:\n", def_name),
            );
            common::print_ast_verbose(&expanded_def);
            *n = expanded_def;
        }
        _ => {}
    }
}
pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match (&n.type_, &mut n.node) {
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(expr)) => {
            if expr.parameter == old {
                expr.parameter = new_name.to_string();
            }
            if let Some(body) = expr.body.as_mut() {
                replace(body, old, new_name);
            }
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(app)) => {
            if let Some(function) = app.function.as_mut() {
                replace(function, old, new_name);
            }
            if let Some(argument) = app.argument.as_mut() {
                replace(argument, old, new_name);
            }
        }
        (common::AstNodeType::VAR, common::AstNodeUnion::Variable(var)) => {
            if var.name == old {
                var.name = new_name.to_string();
            }
        }
        _ => {}
    }
}
pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    match (&n.type_, &n.node) {
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(expr)) => {
            let body = expr
                .body
                .as_ref()
                .map(|body| {
                    if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE) {
                        reduce_ast(table, body)
                    } else {
                        (**body).clone()
                    }
                });

            common::AstNode {
                type_: common::AstNodeType::LAMBDA_EXPR,
                node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
                    parameter: expr.parameter.clone(),
                    type_: expr.type_.clone(),
                    body: body.map(Box::new),
                }),
            }
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(app)) => {
            let function = app
                .function
                .as_ref()
                .map(|function| reduce_ast(table, function))
                .unwrap_or_default();
            let argument = app
                .argument
                .as_ref()
                .map(|argument| {
                    if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE) {
                        reduce_ast(table, argument)
                    } else {
                        (**argument).clone()
                    }
                })
                .unwrap_or_default();

            if function.type_ == common::AstNodeType::LAMBDA_EXPR {
                if let common::AstNodeUnion::LambdaExpr(lambda) = &function.node {
                    let reduced = substitute(
                        lambda
                            .body
                            .as_ref()
                            .map(|body| body.as_ref())
                            .unwrap_or(&common::AstNode::default()),
                        &lambda.parameter,
                        &argument,
                    );
                    common::print_verbose(
                        "",
                        format_args!(
                            "Applied substitution to lambda expr of parameter <{}> and resulted in:\n",
                            lambda.parameter
                        ),
                    );
                    common::print_ast_verbose(&reduced);
                    if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE) {
                        reduced
                    } else {
                        reduce_ast(table, &reduced)
                    }
                } else {
                    parser::create_application(&function, &argument)
                }
            } else {
                parser::create_application(&function, &argument)
            }
        }
        _ => n.clone(),
    }
}
pub fn substitute(expression: &common::AstNode, variable: &str, replacement: &common::AstNode) -> common::AstNode {
    match (&expression.type_, &expression.node) {
        (common::AstNodeType::VAR, common::AstNodeUnion::Variable(var)) => {
            if var.name == variable {
                deepcopy(replacement)
            } else {
                expression.clone()
            }
        }
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(expr)) => {
            let substituted_body = expr
                .body
                .as_ref()
                .map(|body| substitute(body, variable, replacement))
                .unwrap_or_default();
            if expr.parameter != variable {
                common::AstNode {
                    type_: common::AstNodeType::LAMBDA_EXPR,
                    node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
                        parameter: expr.parameter.clone(),
                        type_: expr.type_.clone(),
                        body: Some(Box::new(substituted_body)),
                    }),
                }
            } else {
                substituted_body
            }
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(app)) => {
            let function = app
                .function
                .as_ref()
                .map(|function| substitute(function, variable, replacement))
                .unwrap_or_default();
            let argument = app
                .argument
                .as_ref()
                .map(|argument| substitute(argument, variable, replacement))
                .unwrap_or_default();
            parser::create_application(&function, &argument)
        }
        _ => expression.clone(),
    }
}
pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match (&n.type_, &n.node) {
        (common::AstNodeType::VAR, common::AstNodeUnion::Variable(var)) => {
            deepcopy_var(&var.name, &var.type_)
        }
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(expr)) => {
            deepcopy_lambda_expr(
                &expr.parameter,
                expr.body.as_ref().map(|body| body.as_ref()).unwrap_or(&common::AstNode::default()),
                &expr.type_,
            )
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(app)) => deepcopy_application(
            app.function.as_ref().map(|function| function.as_ref()).unwrap_or(&common::AstNode::default()),
            app.argument.as_ref().map(|argument| argument.as_ref()).unwrap_or(&common::AstNode::default()),
        ),
        _ => n.clone(),
    }
}
pub fn deepcopy_application(function: &common::AstNode, argument: &common::AstNode) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::APPLICATION,
        node: common::AstNodeUnion::Application(common::Application {
            function: Some(Box::new(deepcopy(function))),
            argument: Some(Box::new(deepcopy(argument))),
        }),
    }
}
pub fn deepcopy_lambda_expr(parameter: &str, body: &common::AstNode, type_: &str) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::LAMBDA_EXPR,
        node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
            parameter: parameter.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(deepcopy(body))),
        }),
    }
}
pub fn deepcopy_var(name: &str, type_: &str) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}
