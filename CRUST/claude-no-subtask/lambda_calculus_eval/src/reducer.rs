use crate::common::{self, AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable};
use crate::config;
use crate::hash_table::HashTable;
use std::cell::RefCell;

pub const SIZE: usize = 122;

thread_local! {
    static REDUCTION_ORDER: RefCell<config::reduction_order_t> = RefCell::new(config::reduction_order_t::APPLICATIVE);
}

fn current_reduction_order() -> config::reduction_order_t {
    REDUCTION_ORDER.with(|r| match *r.borrow() {
        config::reduction_order_t::APPLICATIVE => config::reduction_order_t::APPLICATIVE,
        config::reduction_order_t::NORMAL => config::reduction_order_t::NORMAL,
    })
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    REDUCTION_ORDER.with(|r| *r.borrow_mut() = t);
}

pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}

pub fn reduce(table: &mut HashTable, n: &AstNode) -> AstNode {
    let mut node = n.clone();
    expand_definitions(table, &mut node);
    reduce_ast(table, &node)
}

pub fn expand_definitions(table: &mut HashTable, n: &mut AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &mut n.node {
                if let Some(body) = le.body.as_mut() {
                    expand_definitions(table, body);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &mut n.node {
                if let Some(f) = app.function.as_mut() {
                    expand_definitions(table, f);
                }
                if let Some(a) = app.argument.as_mut() {
                    expand_definitions(table, a);
                }
            }
        }
        AstNodeType::DEFINITION => {
            let def_name = match &n.node {
                AstNodeUnion::Variable(v) => v.name.clone(),
                _ => return,
            };
            let expanded = match table.search(&def_name) {
                Some(e) => e.clone(),
                None => {
                    eprintln!("ERROR: Null pointer encountered in expand_definitions for {}", def_name);
                    std::process::exit(1);
                }
            };
            n.type_ = expanded.type_.clone();
            n.node = expanded.node.clone();
        }
        AstNodeType::VAR => {}
    }
}

pub fn replace(n: &mut AstNode, old: &str, new_name: &str) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &mut n.node {
                if le.parameter == old {
                    le.parameter = new_name.to_string();
                }
                if let Some(body) = le.body.as_mut() {
                    replace(body, old, new_name);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &mut n.node {
                if let Some(f) = app.function.as_mut() {
                    replace(f, old, new_name);
                }
                if let Some(a) = app.argument.as_mut() {
                    replace(a, old, new_name);
                }
            }
        }
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &mut n.node {
                if v.name == old {
                    v.name = new_name.to_string();
                }
            }
        }
        AstNodeType::DEFINITION => {}
    }
}

pub fn reduce_ast(table: &mut HashTable, n: &AstNode) -> AstNode {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            let mut new_node = n.clone();
            if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE) {
                if let AstNodeUnion::LambdaExpr(le) = &mut new_node.node {
                    if let Some(body) = le.body.as_ref() {
                        let reduced_body = reduce_ast(table, body);
                        le.body = Some(Box::new(reduced_body));
                    }
                }
            }
            new_node
        }
        AstNodeType::APPLICATION => {
            // Get function and argument
            let (function, argument) = match &n.node {
                AstNodeUnion::Application(app) => {
                    let f = app.function.as_deref().cloned().unwrap_or_default();
                    let a = app.argument.as_deref().cloned().unwrap_or_default();
                    (f, a)
                }
                _ => return n.clone(),
            };

            let reduced_function = reduce_ast(table, &function);
            let reduced_argument = if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE) {
                reduce_ast(table, &argument)
            } else {
                argument.clone()
            };

            if reduced_function.type_ == AstNodeType::LAMBDA_EXPR {
                let (param, body) = match &reduced_function.node {
                    AstNodeUnion::LambdaExpr(le) => {
                        let body = le.body.as_deref().cloned().unwrap_or_default();
                        (le.parameter.clone(), body)
                    }
                    _ => unreachable!(),
                };
                let reduced = substitute(&body, &param, &reduced_argument);
                if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE) {
                    return reduced;
                }
                return reduce_ast(table, &reduced);
            }

            // Otherwise rebuild application
            AstNode {
                type_: AstNodeType::APPLICATION,
                node: AstNodeUnion::Application(Application {
                    function: Some(Box::new(reduced_function)),
                    argument: Some(Box::new(reduced_argument)),
                }),
            }
        }
        _ => n.clone(),
    }
}

pub fn substitute(expression: &AstNode, variable: &str, replacement: &AstNode) -> AstNode {
    match expression.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &expression.node {
                if v.name == variable {
                    return deepcopy(replacement);
                }
            }
            expression.clone()
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &expression.node {
                let body = le.body.as_deref().cloned().unwrap_or_default();
                let new_body = substitute(&body, variable, replacement);
                if le.parameter != variable {
                    return AstNode {
                        type_: AstNodeType::LAMBDA_EXPR,
                        node: AstNodeUnion::LambdaExpr(LambdaExpression {
                            parameter: le.parameter.clone(),
                            type_: le.type_.clone(),
                            body: Some(Box::new(new_body)),
                        }),
                    };
                }
                return new_body;
            }
            expression.clone()
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &expression.node {
                let f = app.function.as_deref().cloned().unwrap_or_default();
                let a = app.argument.as_deref().cloned().unwrap_or_default();
                let new_f = substitute(&f, variable, replacement);
                let new_a = substitute(&a, variable, replacement);
                return AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(new_f)),
                        argument: Some(Box::new(new_a)),
                    }),
                };
            }
            expression.clone()
        }
        _ => expression.clone(),
    }
}

pub fn deepcopy(n: &AstNode) -> AstNode {
    match n.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &n.node {
                return deepcopy_var(&v.name, &v.type_);
            }
            n.clone()
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &n.node {
                let body = le.body.as_deref().cloned().unwrap_or_default();
                return deepcopy_lambda_expr(&le.parameter, &body, &le.type_);
            }
            n.clone()
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &n.node {
                let f = app.function.as_deref().cloned().unwrap_or_default();
                let a = app.argument.as_deref().cloned().unwrap_or_default();
                return deepcopy_application(&f, &a);
            }
            n.clone()
        }
        AstNodeType::DEFINITION => n.clone(),
    }
}

pub fn deepcopy_application(function: &AstNode, argument: &AstNode) -> AstNode {
    AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(deepcopy(function))),
            argument: Some(Box::new(deepcopy(argument))),
        }),
    }
}

pub fn deepcopy_lambda_expr(parameter: &str, body: &AstNode, type_: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: parameter.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(deepcopy(body))),
        }),
    }
}

pub fn deepcopy_var(name: &str, type_: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}

#[allow(dead_code)]
fn _suppress() {
    let _: common::tokens_t = common::tokens_t::ERROR;
}
