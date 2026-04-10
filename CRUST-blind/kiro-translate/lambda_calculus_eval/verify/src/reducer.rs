use crate::{common, hash_table, parser, config};
use crate::common::{AstNode, AstNodeType, AstNodeUnion, LambdaExpression, Application, Variable};
use std::sync::Mutex;

pub const SIZE: usize = 122;

static REDUCTION_ORDER: Mutex<config::reduction_order_t> = Mutex::new(config::reduction_order_t::APPLICATIVE);

fn is_applicative() -> bool {
    matches!(*REDUCTION_ORDER.lock().unwrap(), config::reduction_order_t::APPLICATIVE)
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    *REDUCTION_ORDER.lock().unwrap() = t;
}
pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}
pub fn reduce(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let order = if is_applicative() { config::reduction_order_t::APPLICATIVE } else { config::reduction_order_t::NORMAL };
    common::print_verbose("Order of reduction is: ", format_args!("Order of reduction is: "));
    print_reduction_order(order);
    common::print_verbose("-------------------------------------------\n", format_args!("-------------------------------------------\n"));
    let mut expanded = deepcopy(n);
    expand_definitions_mut(table, &mut expanded);
    common::print_verbose("Expanded expression:\n", format_args!("Expanded expression:\n"));
    common::print_ast_verbose(&expanded);
    let reduced = reduce_ast(table, &expanded);
    common::print_verbose("Final reduced expression:\n", format_args!("Final reduced expression:\n"));
    common::print_ast_verbose(&reduced);
    common::print_verbose("-------------------------------------------\n", format_args!("-------------------------------------------\n"));
    reduced
}

fn expand_definitions_mut(table: &mut hash_table::HashTable, n: &mut AstNode) {
    match &mut n.node {
        AstNodeUnion::LambdaExpr(le) => {
            if let Some(body) = &mut le.body {
                expand_definitions_mut(table, body);
            }
        }
        AstNodeUnion::Application(app) => {
            if let Some(f) = &mut app.function {
                expand_definitions_mut(table, f);
            }
            if let Some(a) = &mut app.argument {
                expand_definitions_mut(table, a);
            }
        }
        AstNodeUnion::Variable(var) => {
            if n.type_ == AstNodeType::DEFINITION {
                let def_name = var.name.clone();
                if let Some(expanded_def) = table.search(&def_name) {
                    common::print_verbose("Expanding definition", format_args!("Expanding definition of: {} . Term expanded to:\n", def_name));
                    common::print_ast_verbose(expanded_def);
                    let copy = deepcopy(expanded_def);
                    n.type_ = copy.type_;
                    n.node = copy.node;
                }
            }
        }
    }
}

pub fn expand_definitions(table: &mut hash_table::HashTable, n: &common::AstNode) {
    // Can't mutate through &AstNode; expansion is handled internally in reduce()
}
pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match &mut n.node {
        AstNodeUnion::LambdaExpr(le) => {
            if le.parameter == old {
                le.parameter = new_name.to_string();
            }
            if let Some(body) = &mut le.body {
                replace(body, old, new_name);
            }
        }
        AstNodeUnion::Application(app) => {
            if let Some(f) = &mut app.function {
                replace(f, old, new_name);
            }
            if let Some(a) = &mut app.argument {
                replace(a, old, new_name);
            }
        }
        AstNodeUnion::Variable(var) => {
            if var.name == old {
                var.name = new_name.to_string();
            }
        }
    }
}
pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    match &n.node {
        AstNodeUnion::LambdaExpr(le) => {
            if is_applicative() {
                if let Some(body) = &le.body {
                    let reduced_body = reduce_ast(table, body);
                    return AstNode {
                        type_: AstNodeType::LAMBDA_EXPR,
                        node: AstNodeUnion::LambdaExpr(LambdaExpression {
                            parameter: le.parameter.clone(),
                            type_: le.type_.clone(),
                            body: Some(Box::new(reduced_body)),
                        }),
                    };
                }
            }
            deepcopy(n)
        }
        AstNodeUnion::Application(app) => {
            let function = app.function.as_ref().unwrap();
            let argument = app.argument.as_ref().unwrap();

            let reduced_func = reduce_ast(table, function);
            let reduced_arg = if is_applicative() {
                reduce_ast(table, argument)
            } else {
                deepcopy(argument)
            };

            if reduced_func.type_ == AstNodeType::LAMBDA_EXPR {
                if let AstNodeUnion::LambdaExpr(ref le) = reduced_func.node {
                    let param = le.parameter.clone();
                    let body = le.body.as_ref().unwrap();
                    let reduced = substitute(body, &param, &reduced_arg);
                    common::print_verbose("Applied substitution", format_args!("Applied substitution to lambda expr of parameter <{}> and resulted in:\n", param));
                    common::print_ast_verbose(&reduced);

                    if is_applicative() {
                        return reduced;
                    }
                    return reduce_ast(table, &reduced);
                }
            }
            AstNode {
                type_: AstNodeType::APPLICATION,
                node: AstNodeUnion::Application(Application {
                    function: Some(Box::new(reduced_func)),
                    argument: Some(Box::new(reduced_arg)),
                }),
            }
        }
        _ => deepcopy(n),
    }
}
pub fn substitute(expression: &common::AstNode, variable: &str, replacement: &common::AstNode) -> common::AstNode {
    match &expression.node {
        AstNodeUnion::Variable(var) => {
            if var.name == variable {
                return deepcopy(replacement);
            }
            deepcopy(expression)
        }
        AstNodeUnion::LambdaExpr(le) => {
            let new_body = if let Some(body) = &le.body {
                Some(Box::new(substitute(body, variable, replacement)))
            } else {
                None
            };
            if le.parameter == variable {
                // Return just the body (unwrap the lambda)
                if let Some(b) = new_body {
                    return *b;
                }
                return AstNode::default();
            }
            AstNode {
                type_: AstNodeType::LAMBDA_EXPR,
                node: AstNodeUnion::LambdaExpr(LambdaExpression {
                    parameter: le.parameter.clone(),
                    type_: le.type_.clone(),
                    body: new_body,
                }),
            }
        }
        AstNodeUnion::Application(app) => {
            let new_func = if let Some(f) = &app.function {
                Some(Box::new(substitute(f, variable, replacement)))
            } else {
                None
            };
            let new_arg = if let Some(a) = &app.argument {
                Some(Box::new(substitute(a, variable, replacement)))
            } else {
                None
            };
            AstNode {
                type_: AstNodeType::APPLICATION,
                node: AstNodeUnion::Application(Application {
                    function: new_func,
                    argument: new_arg,
                }),
            }
        }
    }
}
pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match &n.node {
        AstNodeUnion::Variable(var) => deepcopy_var(&var.name, &var.type_),
        AstNodeUnion::LambdaExpr(le) => {
            if let Some(body) = &le.body {
                deepcopy_lambda_expr(&le.parameter, body, &le.type_)
            } else {
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: le.parameter.clone(),
                        type_: le.type_.clone(),
                        body: None,
                    }),
                }
            }
        }
        AstNodeUnion::Application(app) => {
            let f = app.function.as_ref().unwrap();
            let a = app.argument.as_ref().unwrap();
            deepcopy_application(f, a)
        }
    }
}
pub fn deepcopy_application(function: &common::AstNode, argument: &common::AstNode) -> common::AstNode {
    AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(deepcopy(function))),
            argument: Some(Box::new(deepcopy(argument))),
        }),
    }
}
pub fn deepcopy_lambda_expr(parameter: &str, body: &common::AstNode, type_: &str) -> common::AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: parameter.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(deepcopy(body))),
        }),
    }
}
pub fn deepcopy_var(name: &str, type_: &str) -> common::AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}
