use crate::common::{self, AstNode, AstNodeType, AstNodeUnion, LambdaExpression, Application, Variable};
use crate::{config, hash_table};
use std::sync::atomic::{AtomicU8, Ordering};

pub const SIZE: usize = 122;

const ORDER_APPLICATIVE: u8 = 0;
const ORDER_NORMAL: u8 = 1;

static REDUCTION_ORDER: AtomicU8 = AtomicU8::new(ORDER_APPLICATIVE);

fn current_order() -> u8 {
    REDUCTION_ORDER.load(Ordering::SeqCst)
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    let v = match t {
        config::reduction_order_t::APPLICATIVE => ORDER_APPLICATIVE,
        config::reduction_order_t::NORMAL => ORDER_NORMAL,
    };
    REDUCTION_ORDER.store(v, Ordering::SeqCst);
}

pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}

pub fn reduce(table: &mut hash_table::HashTable, n: &AstNode) -> AstNode {
    let mut n_copy = deepcopy(n);
    expand_definitions(table, &mut n_copy);
    reduce_ast(table, &n_copy)
}

pub fn expand_definitions(table: &mut hash_table::HashTable, n: &mut AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref mut l) = n.node {
                if let Some(ref mut body) = l.body {
                    expand_definitions(table, body);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref mut app) = n.node {
                if let Some(ref mut f) = app.function {
                    expand_definitions(table, f);
                }
                if let Some(ref mut a) = app.argument {
                    expand_definitions(table, a);
                }
            }
        }
        AstNodeType::DEFINITION => {
            let def_name = if let AstNodeUnion::Variable(ref v) = n.node {
                v.name.clone()
            } else {
                return;
            };
            let expanded = match table.search(&def_name) {
                Some(d) => deepcopy(d),
                None => {
                    eprintln!("ERROR: Null pointer encountered (definition not found): {}", def_name);
                    std::process::exit(1);
                }
            };
            *n = expanded;
        }
        _ => {}
    }
}

pub fn replace(n: &mut AstNode, old: &str, new_name: &str) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref mut l) = n.node {
                if l.parameter == old {
                    l.parameter = new_name.to_string();
                }
                if let Some(ref mut body) = l.body {
                    replace(body, old, new_name);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref mut app) = n.node {
                if let Some(ref mut f) = app.function {
                    replace(f, old, new_name);
                }
                if let Some(ref mut a) = app.argument {
                    replace(a, old, new_name);
                }
            }
        }
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(ref mut v) = n.node {
                if v.name == old {
                    v.name = new_name.to_string();
                }
            }
        }
        AstNodeType::DEFINITION => {}
    }
}

pub fn reduce_ast(table: &mut hash_table::HashTable, n: &AstNode) -> AstNode {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref l) = n.node {
                let new_body = if let Some(ref body) = l.body {
                    if current_order() == ORDER_APPLICATIVE {
                        reduce_ast(table, body)
                    } else {
                        deepcopy(body)
                    }
                } else {
                    AstNode::default()
                };
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: l.parameter.clone(),
                        type_: l.type_.clone(),
                        body: Some(Box::new(new_body)),
                    }),
                }
            } else {
                deepcopy(n)
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = n.node {
                let function = if let Some(ref f) = app.function {
                    reduce_ast(table, f)
                } else {
                    return deepcopy(n);
                };

                let argument = if let Some(ref a) = app.argument {
                    if current_order() == ORDER_APPLICATIVE {
                        reduce_ast(table, a)
                    } else {
                        deepcopy(a)
                    }
                } else {
                    return deepcopy(n);
                };

                if function.type_ == AstNodeType::LAMBDA_EXPR {
                    if let AstNodeUnion::LambdaExpr(ref l) = function.node {
                        let param = l.parameter.clone();
                        if let Some(ref body) = l.body {
                            let reduced = substitute(body, &param, &argument);
                            if current_order() == ORDER_APPLICATIVE {
                                return reduced;
                            }
                            return reduce_ast(table, &reduced);
                        }
                    }
                }

                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(function)),
                        argument: Some(Box::new(argument)),
                    }),
                }
            } else {
                deepcopy(n)
            }
        }
        _ => deepcopy(n),
    }
}

pub fn substitute(expression: &AstNode, variable: &str, replacement: &AstNode) -> AstNode {
    match expression.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(ref v) = expression.node {
                if v.name == variable {
                    return deepcopy(replacement);
                }
            }
            deepcopy(expression)
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref l) = expression.node {
                let new_body = if let Some(ref body) = l.body {
                    Some(Box::new(substitute(body, variable, replacement)))
                } else {
                    None
                };
                // C uses pointer comparison, which is typically always different.
                // Always return the lambda with substituted body.
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: l.parameter.clone(),
                        type_: l.type_.clone(),
                        body: new_body,
                    }),
                }
            } else {
                deepcopy(expression)
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = expression.node {
                let new_func = app.function.as_ref().map(|f| Box::new(substitute(f, variable, replacement)));
                let new_arg = app.argument.as_ref().map(|a| Box::new(substitute(a, variable, replacement)));
                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: new_func,
                        argument: new_arg,
                    }),
                }
            } else {
                deepcopy(expression)
            }
        }
        AstNodeType::DEFINITION => deepcopy(expression),
    }
}

pub fn deepcopy(n: &AstNode) -> AstNode {
    match n.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(ref v) = n.node {
                deepcopy_var(&v.name, &v.type_)
            } else {
                AstNode::default()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref l) = n.node {
                let body_copy = if let Some(ref body) = l.body {
                    Some(Box::new(deepcopy(body)))
                } else {
                    None
                };
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: l.parameter.clone(),
                        type_: l.type_.clone(),
                        body: body_copy,
                    }),
                }
            } else {
                AstNode::default()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = n.node {
                let f_copy = app.function.as_ref().map(|f| Box::new(deepcopy(f)));
                let a_copy = app.argument.as_ref().map(|a| Box::new(deepcopy(a)));
                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: f_copy,
                        argument: a_copy,
                    }),
                }
            } else {
                AstNode::default()
            }
        }
        AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(ref v) = n.node {
                AstNode {
                    type_: AstNodeType::DEFINITION,
                    node: AstNodeUnion::Variable(Variable {
                        name: v.name.clone(),
                        type_: v.type_.clone(),
                    }),
                }
            } else {
                AstNode::default()
            }
        }
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

// Suppress unused-import warning while maintaining the import for future use.
#[allow(dead_code)]
fn _suppress_common_use(_: &common::AstNode) {}
