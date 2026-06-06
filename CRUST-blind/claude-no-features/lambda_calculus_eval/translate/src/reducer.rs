use crate::{common, hash_table, config};
use std::sync::atomic::{AtomicU8, Ordering};

pub const SIZE: usize = 122;

// 0 = APPLICATIVE, 1 = NORMAL
static REDUCTION_ORDER: AtomicU8 = AtomicU8::new(0);

pub fn set_reduction_order(t: config::reduction_order_t) {
    let v = match t {
        config::reduction_order_t::APPLICATIVE => 0,
        config::reduction_order_t::NORMAL => 1,
    };
    REDUCTION_ORDER.store(v, Ordering::SeqCst);
}

fn current_order() -> config::reduction_order_t {
    match REDUCTION_ORDER.load(Ordering::SeqCst) {
        1 => config::reduction_order_t::NORMAL,
        _ => config::reduction_order_t::APPLICATIVE,
    }
}

pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}

pub fn reduce(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let mut working = deepcopy(n);
    expand_definitions(table, &mut working);
    let reduced = reduce_ast(table, &working);
    reduced
}

pub fn expand_definitions(table: &mut hash_table::HashTable, n: &mut common::AstNode) {
    match n.type_ {
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &mut n.node {
                if let Some(body) = &mut le.body {
                    expand_definitions(table, body);
                }
            }
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &mut n.node {
                if let Some(f) = &mut app.function {
                    expand_definitions(table, f);
                }
                if let Some(a) = &mut app.argument {
                    expand_definitions(table, a);
                }
            }
        }
        common::AstNodeType::DEFINITION => {
            let def_name = if let common::AstNodeUnion::Variable(v) = &n.node {
                v.name.clone()
            } else {
                return;
            };
            let expanded = match table.search(&def_name) {
                Some(e) => deepcopy(e),
                None => {
                    eprintln!("ERROR: Null pointer encountered (no definition for {})", def_name);
                    std::process::exit(1);
                }
            };
            *n = expanded;
        }
        _ => {}
    }
}

pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match n.type_ {
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &mut n.node {
                if le.parameter == old {
                    le.parameter = new_name.to_string();
                }
                if let Some(body) = &mut le.body {
                    replace(body, old, new_name);
                }
            }
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &mut n.node {
                if let Some(f) = &mut app.function {
                    replace(f, old, new_name);
                }
                if let Some(a) = &mut app.argument {
                    replace(a, old, new_name);
                }
            }
        }
        common::AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(v) = &mut n.node {
                if v.name == old {
                    v.name = new_name.to_string();
                }
            }
        }
        _ => {}
    }
}

pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let order = current_order();
    match n.type_ {
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &n.node {
                let new_body = match &le.body {
                    Some(body) => {
                        if matches!(order, config::reduction_order_t::APPLICATIVE) {
                            Some(Box::new(reduce_ast(table, body)))
                        } else {
                            Some(Box::new(deepcopy(body)))
                        }
                    }
                    None => None,
                };
                return common::AstNode {
                    type_: common::AstNodeType::LAMBDA_EXPR,
                    node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
                        parameter: le.parameter.clone(),
                        type_: le.type_.clone(),
                        body: new_body,
                    }),
                };
            }
            deepcopy(n)
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &n.node {
                let function = match &app.function {
                    Some(f) => f,
                    None => return deepcopy(n),
                };
                let argument = match &app.argument {
                    Some(a) => a,
                    None => return deepcopy(n),
                };
                let reduced_function = reduce_ast(table, function);
                let reduced_argument = if matches!(order, config::reduction_order_t::APPLICATIVE) {
                    reduce_ast(table, argument)
                } else {
                    deepcopy(argument)
                };

                if reduced_function.type_ == common::AstNodeType::LAMBDA_EXPR {
                    if let common::AstNodeUnion::LambdaExpr(le) = &reduced_function.node {
                        let body = match &le.body {
                            Some(b) => b,
                            None => return common::AstNode {
                                type_: common::AstNodeType::APPLICATION,
                                node: common::AstNodeUnion::Application(common::Application {
                                    function: Some(Box::new(reduced_function)),
                                    argument: Some(Box::new(reduced_argument)),
                                }),
                            },
                        };
                        let reduced = substitute(body, &le.parameter, &reduced_argument);
                        if matches!(order, config::reduction_order_t::APPLICATIVE) {
                            return reduced;
                        }
                        return reduce_ast(table, &reduced);
                    }
                }
                return common::AstNode {
                    type_: common::AstNodeType::APPLICATION,
                    node: common::AstNodeUnion::Application(common::Application {
                        function: Some(Box::new(reduced_function)),
                        argument: Some(Box::new(reduced_argument)),
                    }),
                };
            }
            deepcopy(n)
        }
        _ => deepcopy(n),
    }
}

pub fn substitute(
    expression: &common::AstNode,
    variable: &str,
    replacement: &common::AstNode,
) -> common::AstNode {
    match expression.type_ {
        common::AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(v) = &expression.node {
                if v.name == variable {
                    return deepcopy(replacement);
                }
            }
            deepcopy(expression)
        }
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &expression.node {
                let new_body = match &le.body {
                    Some(body) => substitute(body, variable, replacement),
                    None => return deepcopy(expression),
                };
                if le.parameter != variable {
                    return common::AstNode {
                        type_: common::AstNodeType::LAMBDA_EXPR,
                        node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
                            parameter: le.parameter.clone(),
                            type_: le.type_.clone(),
                            body: Some(Box::new(new_body)),
                        }),
                    };
                }
                return new_body;
            }
            deepcopy(expression)
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &expression.node {
                let f = match &app.function {
                    Some(f) => substitute(f, variable, replacement),
                    None => return deepcopy(expression),
                };
                let a = match &app.argument {
                    Some(a) => substitute(a, variable, replacement),
                    None => return deepcopy(expression),
                };
                return common::AstNode {
                    type_: common::AstNodeType::APPLICATION,
                    node: common::AstNodeUnion::Application(common::Application {
                        function: Some(Box::new(f)),
                        argument: Some(Box::new(a)),
                    }),
                };
            }
            deepcopy(expression)
        }
        _ => deepcopy(expression),
    }
}

pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match n.type_ {
        common::AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(v) = &n.node {
                return deepcopy_var(&v.name, &v.type_);
            }
            common::AstNode::default()
        }
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &n.node {
                let body_node = match &le.body {
                    Some(b) => b.as_ref(),
                    None => &common::AstNode::default(),
                };
                return deepcopy_lambda_expr(&le.parameter, body_node, &le.type_);
            }
            common::AstNode::default()
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &n.node {
                let f_node = match &app.function {
                    Some(b) => b.as_ref(),
                    None => &common::AstNode::default(),
                };
                let a_node = match &app.argument {
                    Some(b) => b.as_ref(),
                    None => &common::AstNode::default(),
                };
                return deepcopy_application(f_node, a_node);
            }
            common::AstNode::default()
        }
        common::AstNodeType::DEFINITION => {
            if let common::AstNodeUnion::Variable(v) = &n.node {
                return common::AstNode {
                    type_: common::AstNodeType::DEFINITION,
                    node: common::AstNodeUnion::Variable(common::Variable {
                        name: v.name.clone(),
                        type_: v.type_.clone(),
                    }),
                };
            }
            common::AstNode::default()
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
