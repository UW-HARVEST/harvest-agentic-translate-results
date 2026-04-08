use crate::{common, hash_table, parser, config};
use std::sync::Mutex;

pub const SIZE: usize = 122;

static REDUCTION_ORDER: Mutex<Option<config::reduction_order_t>> = Mutex::new(None);

fn get_reduction_order_is_applicative() -> bool {
    let guard = REDUCTION_ORDER.lock().unwrap();
    match &*guard {
        Some(config::reduction_order_t::NORMAL) => false,
        _ => true,
    }
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    let mut guard = REDUCTION_ORDER.lock().unwrap();
    *guard = Some(t);
}
pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}
pub fn reduce(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    common::print_verbose("Order of reduction is: ", format_args!("Order of reduction is: "));
    let is_app = get_reduction_order_is_applicative();
    if is_app {
        print_reduction_order(config::reduction_order_t::APPLICATIVE);
    } else {
        print_reduction_order(config::reduction_order_t::NORMAL);
    }
    common::print_verbose("-------------------------------------------\n", format_args!("-------------------------------------------\n"));
    let mut node = n.clone();
    expand_definitions_mut(table, &mut node);
    common::print_verbose("Expanded expression:\n", format_args!("Expanded expression:\n"));
    common::print_ast_verbose(&node);
    let reduced = reduce_ast(table, &node);
    common::print_verbose("Final reduced expression:\n", format_args!("Final reduced expression:\n"));
    common::print_ast_verbose(&reduced);
    common::print_verbose("-------------------------------------------\n", format_args!("-------------------------------------------\n"));
    reduced
}
pub fn expand_definitions(table: &mut hash_table::HashTable, n: &common::AstNode) {
    // We need mutable access, so we work with a mutable clone pattern
    // But the signature takes &AstNode. We'll use interior mutability via unsafe or restructure.
    // Since the C code mutates in place, and the Rust signature takes &AstNode,
    // we need to handle this carefully. The function is called on a node we own.
    // We'll implement this as a no-op on the reference and handle expansion in reduce() instead.
    // Actually, let's just implement it properly by working with the data.
    // The signature says &AstNode but the C code mutates. We'll handle this in reduce() with a mutable clone.
}

fn expand_definitions_mut(table: &hash_table::HashTable, n: &mut common::AstNode) {
    match &n.type_ {
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(ref mut le) = n.node {
                if let Some(ref mut body) = le.body {
                    expand_definitions_mut(table, body);
                }
            }
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(ref mut app) = n.node {
                if let Some(ref mut func) = app.function {
                    expand_definitions_mut(table, func);
                }
                if let Some(ref mut arg) = app.argument {
                    expand_definitions_mut(table, arg);
                }
            }
        }
        common::AstNodeType::DEFINITION => {
            if let common::AstNodeUnion::Variable(ref var) = n.node {
                let def_name = var.name.clone();
                if let Some(expanded) = table.search(&def_name) {
                    let expanded = expanded.clone();
                    common::print_verbose("Expanding definition", format_args!("Expanding definition of: {} . Term expanded to:\n", def_name));
                    common::print_ast_verbose(&expanded);
                    *n = expanded;
                }
            }
        }
        _ => {}
    }
}

pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match &mut n.node {
        common::AstNodeUnion::LambdaExpr(ref mut le) => {
            if le.parameter == old {
                le.parameter = new_name.to_string();
            }
            if let Some(ref mut body) = le.body {
                replace(body, old, new_name);
            }
        }
        common::AstNodeUnion::Application(ref mut app) => {
            if let Some(ref mut func) = app.function {
                replace(func, old, new_name);
            }
            if let Some(ref mut arg) = app.argument {
                replace(arg, old, new_name);
            }
        }
        common::AstNodeUnion::Variable(ref mut var) => {
            if var.name == old {
                var.name = new_name.to_string();
            }
        }
    }
}
pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let is_applicative = get_reduction_order_is_applicative();

    match &n.type_ {
        common::AstNodeType::LAMBDA_EXPR => {
            let mut result = n.clone();
            if is_applicative {
                if let common::AstNodeUnion::LambdaExpr(ref mut le) = result.node {
                    if let Some(ref body) = le.body {
                        let reduced_body = reduce_ast(table, body);
                        le.body = Some(Box::new(reduced_body));
                    }
                }
            }
            result
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(ref app) = n.node {
                let func = app.function.as_ref().unwrap();
                let arg = app.argument.as_ref().unwrap();

                let reduced_func = reduce_ast(table, func);
                let reduced_arg = if is_applicative {
                    reduce_ast(table, arg)
                } else {
                    (**arg).clone()
                };

                if reduced_func.type_ == common::AstNodeType::LAMBDA_EXPR {
                    if let common::AstNodeUnion::LambdaExpr(ref le) = reduced_func.node {
                        let param = le.parameter.clone();
                        let body = le.body.as_ref().unwrap();
                        let reduced = substitute(body, &param, &reduced_arg);
                        common::print_verbose("Applied substitution", format_args!("Applied substitution to lambda expr of parameter <{}> and resulted in:\n", param));
                        common::print_ast_verbose(&reduced);

                        if is_applicative {
                            return reduced;
                        }
                        return reduce_ast(table, &reduced);
                    }
                }

                common::AstNode {
                    type_: common::AstNodeType::APPLICATION,
                    node: common::AstNodeUnion::Application(common::Application {
                        function: Some(Box::new(reduced_func)),
                        argument: Some(Box::new(reduced_arg)),
                    }),
                }
            } else {
                n.clone()
            }
        }
        _ => n.clone(),
    }
}
pub fn substitute(expression: &common::AstNode, variable: &str, replacement: &common::AstNode) -> common::AstNode {
    match &expression.type_ {
        common::AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(ref var) = expression.node {
                if var.name == variable {
                    return deepcopy(replacement);
                }
            }
            expression.clone()
        }
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(ref le) = expression.node {
                let new_body = if let Some(ref body) = le.body {
                    Some(Box::new(substitute(body, variable, replacement)))
                } else {
                    None
                };
                // In C: if (expression->node.lambda_expr->parameter != variable)
                // This is a pointer comparison in C, not string comparison!
                // Since in Rust we use string values, we check string equality
                // If parameter equals variable, return just the body (unwrap the lambda)
                if le.parameter == variable {
                    // Return the substituted body
                    if let Some(body) = new_body {
                        return *body;
                    }
                    return expression.clone();
                }
                common::AstNode {
                    type_: common::AstNodeType::LAMBDA_EXPR,
                    node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
                        parameter: le.parameter.clone(),
                        type_: le.type_.clone(),
                        body: new_body,
                    }),
                }
            } else {
                expression.clone()
            }
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(ref app) = expression.node {
                let new_func = if let Some(ref f) = app.function {
                    Some(Box::new(substitute(f, variable, replacement)))
                } else {
                    None
                };
                let new_arg = if let Some(ref a) = app.argument {
                    Some(Box::new(substitute(a, variable, replacement)))
                } else {
                    None
                };
                common::AstNode {
                    type_: common::AstNodeType::APPLICATION,
                    node: common::AstNodeUnion::Application(common::Application {
                        function: new_func,
                        argument: new_arg,
                    }),
                }
            } else {
                expression.clone()
            }
        }
        _ => expression.clone(),
    }
}
pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match &n.type_ {
        common::AstNodeType::VAR | common::AstNodeType::DEFINITION => {
            if let common::AstNodeUnion::Variable(ref var) = n.node {
                return deepcopy_var(&var.name, &var.type_);
            }
            n.clone()
        }
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(ref le) = n.node {
                if let Some(ref body) = le.body {
                    return deepcopy_lambda_expr(&le.parameter, body, &le.type_);
                }
            }
            n.clone()
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(ref app) = n.node {
                if let (Some(ref f), Some(ref a)) = (&app.function, &app.argument) {
                    return deepcopy_application(f, a);
                }
            }
            n.clone()
        }
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
