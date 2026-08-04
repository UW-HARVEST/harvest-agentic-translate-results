use crate::{common, config, hash_table};
use std::sync::atomic::{AtomicU8, Ordering};

pub const SIZE: usize = 122;

// 0 = APPLICATIVE, 1 = NORMAL
static REDUCTION_ORDER: AtomicU8 = AtomicU8::new(0);

pub fn set_reduction_order(t: config::reduction_order_t) {
    let v = match t {
        config::reduction_order_t::APPLICATIVE => 0u8,
        config::reduction_order_t::NORMAL => 1u8,
    };
    REDUCTION_ORDER.store(v, Ordering::Relaxed);
}

fn current_reduction_order() -> config::reduction_order_t {
    match REDUCTION_ORDER.load(Ordering::Relaxed) {
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
    common::print_verbose("Order of reduction is: ", format_args!(""));
    print_reduction_order(current_reduction_order());
    common::print_verbose(
        "-------------------------------------------",
        format_args!(""),
    );
    let mut expanded = n.clone();
    expand_definitions(table, &mut expanded);
    common::print_verbose("Expanded expression:", format_args!(""));
    common::print_ast_verbose(&expanded);
    let reduced = reduce_ast(table, &expanded);
    common::print_verbose("Final reduced expression:", format_args!(""));
    common::print_ast_verbose(&reduced);
    common::print_verbose(
        "-------------------------------------------",
        format_args!(""),
    );
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
            let expanded_def = match table.search(&def_name) {
                Some(node) => node.clone(),
                None => {
                    common::error(
                        "Null pointer encountered",
                        file!(),
                        line!() as i32,
                        "expand_definitions",
                    );
                    return;
                }
            };
            common::print_verbose(
                "Expanding definition of: . Term expanded to:",
                format_args!(""),
            );
            common::print_ast_verbose(&expanded_def);
            *n = expanded_def;
        }
        common::AstNodeType::VAR => {}
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
        common::AstNodeType::DEFINITION => {}
    }
}

pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let order = current_reduction_order();
    match n.type_ {
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &n.node {
                let mut new_le = le.clone();
                if order == config::reduction_order_t::APPLICATIVE {
                    if let Some(body) = &le.body {
                        let reduced_body = reduce_ast(table, body);
                        new_le.body = Some(Box::new(reduced_body));
                    }
                }
                common::AstNode {
                    type_: common::AstNodeType::LAMBDA_EXPR,
                    node: common::AstNodeUnion::LambdaExpr(new_le),
                }
            } else {
                n.clone()
            }
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &n.node {
                let function = match &app.function {
                    Some(f) => reduce_ast(table, f),
                    None => return n.clone(),
                };
                let argument = match &app.argument {
                    Some(a) => {
                        if order == config::reduction_order_t::APPLICATIVE {
                            reduce_ast(table, a)
                        } else {
                            (**a).clone()
                        }
                    }
                    None => return n.clone(),
                };

                if function.type_ == common::AstNodeType::LAMBDA_EXPR {
                    if let common::AstNodeUnion::LambdaExpr(le) = &function.node {
                        let param = le.parameter.clone();
                        let body = match &le.body {
                            Some(b) => (**b).clone(),
                            None => return n.clone(),
                        };
                        let reduced = substitute(&body, &param, &argument);
                        common::print_verbose(
                            "Applied substitution to lambda expr of parameter and resulted in:",
                            format_args!(""),
                        );
                        common::print_ast_verbose(&reduced);

                        if order == config::reduction_order_t::APPLICATIVE {
                            return reduced;
                        }
                        return reduce_ast(table, &reduced);
                    }
                }

                common::AstNode {
                    type_: common::AstNodeType::APPLICATION,
                    node: common::AstNodeUnion::Application(common::Application {
                        function: Some(Box::new(function)),
                        argument: Some(Box::new(argument)),
                    }),
                }
            } else {
                n.clone()
            }
        }
        _ => n.clone(),
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
            expression.clone()
        }
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &expression.node {
                let new_body = match &le.body {
                    Some(b) => substitute(b, variable, replacement),
                    None => return expression.clone(),
                };
                if le.parameter != variable {
                    let mut new_le = le.clone();
                    new_le.body = Some(Box::new(new_body));
                    return common::AstNode {
                        type_: common::AstNodeType::LAMBDA_EXPR,
                        node: common::AstNodeUnion::LambdaExpr(new_le),
                    };
                }
                new_body
            } else {
                expression.clone()
            }
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &expression.node {
                let new_function = match &app.function {
                    Some(f) => substitute(f, variable, replacement),
                    None => return expression.clone(),
                };
                let new_argument = match &app.argument {
                    Some(a) => substitute(a, variable, replacement),
                    None => return expression.clone(),
                };
                common::AstNode {
                    type_: common::AstNodeType::APPLICATION,
                    node: common::AstNodeUnion::Application(common::Application {
                        function: Some(Box::new(new_function)),
                        argument: Some(Box::new(new_argument)),
                    }),
                }
            } else {
                expression.clone()
            }
        }
        common::AstNodeType::DEFINITION => expression.clone(),
    }
}

pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match n.type_ {
        common::AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(v) = &n.node {
                return deepcopy_var(&v.name, &v.type_);
            }
            n.clone()
        }
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &n.node {
                let body_node = match &le.body {
                    Some(b) => (**b).clone(),
                    None => common::AstNode::default(),
                };
                return deepcopy_lambda_expr(&le.parameter, &body_node, &le.type_);
            }
            n.clone()
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &n.node {
                let function_node = match &app.function {
                    Some(f) => (**f).clone(),
                    None => common::AstNode::default(),
                };
                let argument_node = match &app.argument {
                    Some(a) => (**a).clone(),
                    None => common::AstNode::default(),
                };
                return deepcopy_application(&function_node, &argument_node);
            }
            n.clone()
        }
        common::AstNodeType::DEFINITION => n.clone(),
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
