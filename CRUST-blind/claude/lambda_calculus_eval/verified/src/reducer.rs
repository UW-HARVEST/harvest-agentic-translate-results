use crate::{common, config, hash_table};
use std::cell::RefCell;

pub const SIZE: usize = 122;

thread_local! {
    static REDUCTION_ORDER: RefCell<config::reduction_order_t> =
        RefCell::new(config::reduction_order_t::APPLICATIVE);
}

fn current_order() -> config::reduction_order_t {
    REDUCTION_ORDER.with(|c| match *c.borrow() {
        config::reduction_order_t::APPLICATIVE => {
            config::reduction_order_t::APPLICATIVE
        }
        config::reduction_order_t::NORMAL => config::reduction_order_t::NORMAL,
    })
}

fn is_applicative() -> bool {
    REDUCTION_ORDER.with(|c| {
        matches!(*c.borrow(), config::reduction_order_t::APPLICATIVE)
    })
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    REDUCTION_ORDER.with(|c| {
        *c.borrow_mut() = t;
    });
}

pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}

pub fn reduce(
    table: &mut hash_table::HashTable,
    n: &common::AstNode,
) -> common::AstNode {
    let mut copy = deepcopy(n);
    expand_definitions(table, &mut copy);
    reduce_ast(table, &copy)
}

pub fn expand_definitions(
    table: &mut hash_table::HashTable,
    n: &mut common::AstNode,
) {
    match &mut n.node {
        common::AstNodeUnion::LambdaExpr(lambda) => {
            if let Some(body) = &mut lambda.body {
                expand_definitions(table, body);
            }
        }
        common::AstNodeUnion::Application(app) => {
            if let Some(f) = &mut app.function {
                expand_definitions(table, f);
            }
            if let Some(a) = &mut app.argument {
                expand_definitions(table, a);
            }
        }
        common::AstNodeUnion::Variable(_) => {
            if matches!(n.type_, common::AstNodeType::DEFINITION) {
                let def_name = match &n.node {
                    common::AstNodeUnion::Variable(v) => v.name.clone(),
                    _ => String::new(),
                };
                if let Some(expanded_def) = table.search(&def_name) {
                    let copied = deepcopy(expanded_def);
                    *n = copied;
                } else {
                    let msg = format!(
                        "ERROR: Could not expand definition for {}",
                        def_name
                    );
                    common::error(
                        &msg,
                        file!(),
                        line!() as i32,
                        "expand_definitions",
                    );
                }
            }
        }
    }
}

pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match &mut n.node {
        common::AstNodeUnion::LambdaExpr(lambda) => {
            if lambda.parameter == old {
                lambda.parameter = new_name.to_string();
            }
            if let Some(body) = &mut lambda.body {
                replace(body, old, new_name);
            }
        }
        common::AstNodeUnion::Application(app) => {
            if let Some(f) = &mut app.function {
                replace(f, old, new_name);
            }
            if let Some(a) = &mut app.argument {
                replace(a, old, new_name);
            }
        }
        common::AstNodeUnion::Variable(var) => {
            if matches!(n.type_, common::AstNodeType::VAR) && var.name == old {
                var.name = new_name.to_string();
            }
        }
    }
}

pub fn reduce_ast(
    table: &mut hash_table::HashTable,
    n: &common::AstNode,
) -> common::AstNode {
    match &n.node {
        common::AstNodeUnion::LambdaExpr(lambda) => {
            // Recursively reduce the body if applicative
            let new_body = if is_applicative() {
                if let Some(body) = &lambda.body {
                    Some(Box::new(reduce_ast(table, body)))
                } else {
                    None
                }
            } else if let Some(body) = &lambda.body {
                Some(Box::new(deepcopy(body)))
            } else {
                None
            };
            common::AstNode {
                type_: common::AstNodeType::LAMBDA_EXPR,
                node: common::AstNodeUnion::LambdaExpr(
                    common::LambdaExpression {
                        parameter: lambda.parameter.clone(),
                        type_: lambda.type_.clone(),
                        body: new_body,
                    },
                ),
            }
        }
        common::AstNodeUnion::Application(app) => {
            let function_default = common::AstNode::default();
            let argument_default = common::AstNode::default();
            let function_ref = app
                .function
                .as_deref()
                .unwrap_or(&function_default);
            let argument_ref = app
                .argument
                .as_deref()
                .unwrap_or(&argument_default);

            let reduced_function = reduce_ast(table, function_ref);

            let reduced_argument = if is_applicative() {
                reduce_ast(table, argument_ref)
            } else {
                deepcopy(argument_ref)
            };

            if matches!(reduced_function.type_, common::AstNodeType::LAMBDA_EXPR) {
                if let common::AstNodeUnion::LambdaExpr(lambda) =
                    &reduced_function.node
                {
                    let param = lambda.parameter.clone();
                    let body_default = common::AstNode::default();
                    let body_ref =
                        lambda.body.as_deref().unwrap_or(&body_default);
                    let reduced =
                        substitute(body_ref, &param, &reduced_argument);
                    if is_applicative() {
                        return reduced;
                    }
                    return reduce_ast(table, &reduced);
                }
            }
            common::AstNode {
                type_: common::AstNodeType::APPLICATION,
                node: common::AstNodeUnion::Application(common::Application {
                    function: Some(Box::new(reduced_function)),
                    argument: Some(Box::new(reduced_argument)),
                }),
            }
        }
        common::AstNodeUnion::Variable(_) => deepcopy(n),
    }
}

pub fn substitute(
    expression: &common::AstNode,
    variable: &str,
    replacement: &common::AstNode,
) -> common::AstNode {
    match &expression.node {
        common::AstNodeUnion::Variable(var) => {
            if matches!(expression.type_, common::AstNodeType::VAR)
                && var.name == variable
            {
                deepcopy(replacement)
            } else {
                deepcopy(expression)
            }
        }
        common::AstNodeUnion::LambdaExpr(lambda) => {
            let body_default = common::AstNode::default();
            let body_ref = lambda.body.as_deref().unwrap_or(&body_default);
            let new_body = substitute(body_ref, variable, replacement);
            if lambda.parameter != variable {
                common::AstNode {
                    type_: common::AstNodeType::LAMBDA_EXPR,
                    node: common::AstNodeUnion::LambdaExpr(
                        common::LambdaExpression {
                            parameter: lambda.parameter.clone(),
                            type_: lambda.type_.clone(),
                            body: Some(Box::new(new_body)),
                        },
                    ),
                }
            } else {
                // Strip the lambda - the parameter is being substituted away.
                new_body
            }
        }
        common::AstNodeUnion::Application(app) => {
            let function_default = common::AstNode::default();
            let argument_default = common::AstNode::default();
            let function_ref =
                app.function.as_deref().unwrap_or(&function_default);
            let argument_ref =
                app.argument.as_deref().unwrap_or(&argument_default);
            let new_function = substitute(function_ref, variable, replacement);
            let new_argument = substitute(argument_ref, variable, replacement);
            common::AstNode {
                type_: common::AstNodeType::APPLICATION,
                node: common::AstNodeUnion::Application(common::Application {
                    function: Some(Box::new(new_function)),
                    argument: Some(Box::new(new_argument)),
                }),
            }
        }
    }
}

pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match &n.node {
        common::AstNodeUnion::Variable(var) => {
            let mut copy = deepcopy_var(&var.name, &var.type_);
            // Preserve DEFINITION marker if present.
            if matches!(n.type_, common::AstNodeType::DEFINITION) {
                copy.type_ = common::AstNodeType::DEFINITION;
            }
            copy
        }
        common::AstNodeUnion::LambdaExpr(lambda) => {
            let body_default = common::AstNode::default();
            let body_ref = lambda.body.as_deref().unwrap_or(&body_default);
            deepcopy_lambda_expr(&lambda.parameter, body_ref, &lambda.type_)
        }
        common::AstNodeUnion::Application(app) => {
            let function_default = common::AstNode::default();
            let argument_default = common::AstNode::default();
            let function_ref =
                app.function.as_deref().unwrap_or(&function_default);
            let argument_ref =
                app.argument.as_deref().unwrap_or(&argument_default);
            deepcopy_application(function_ref, argument_ref)
        }
    }
}

pub fn deepcopy_application(
    function: &common::AstNode,
    argument: &common::AstNode,
) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::APPLICATION,
        node: common::AstNodeUnion::Application(common::Application {
            function: Some(Box::new(deepcopy(function))),
            argument: Some(Box::new(deepcopy(argument))),
        }),
    }
}

pub fn deepcopy_lambda_expr(
    parameter: &str,
    body: &common::AstNode,
    type_: &str,
) -> common::AstNode {
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

// `current_order` is unused publicly but useful for callers; suppress warnings.
#[allow(dead_code)]
fn _use_current_order() -> config::reduction_order_t {
    current_order()
}
