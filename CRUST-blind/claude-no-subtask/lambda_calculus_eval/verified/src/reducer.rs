use crate::common::{self, AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable};
use crate::{config, hash_table};
use std::sync::atomic::{AtomicU8, Ordering};

pub const SIZE: usize = 122;

const APPLICATIVE: u8 = 0;
const NORMAL: u8 = 1;

static REDUCTION_ORDER: AtomicU8 = AtomicU8::new(APPLICATIVE);

pub fn set_reduction_order(t: config::reduction_order_t) {
    let v = match t {
        config::reduction_order_t::APPLICATIVE => APPLICATIVE,
        config::reduction_order_t::NORMAL => NORMAL,
    };
    REDUCTION_ORDER.store(v, Ordering::SeqCst);
}

fn current_order() -> u8 {
    REDUCTION_ORDER.load(Ordering::SeqCst)
}

pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}

pub fn reduce(table: &mut hash_table::HashTable, n: &AstNode) -> AstNode {
    let mut copy = deepcopy(n);
    expand_definitions(table, &mut copy);
    reduce_ast(table, &copy)
}

pub fn expand_definitions(table: &mut hash_table::HashTable, n: &mut AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(lam) = &mut n.node {
                if let Some(body) = lam.body.as_deref_mut() {
                    expand_definitions(table, body);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &mut n.node {
                if let Some(f) = app.function.as_deref_mut() {
                    expand_definitions(table, f);
                }
                if let Some(a) = app.argument.as_deref_mut() {
                    expand_definitions(table, a);
                }
            }
        }
        AstNodeType::DEFINITION => {
            let def_name = if let AstNodeUnion::Variable(v) = &n.node {
                v.name.clone()
            } else {
                return;
            };
            let expanded = match table.search(&def_name) {
                Some(d) => deepcopy(d),
                None => {
                    common::error(
                        "Null pointer encountered (definition lookup)",
                        file!(),
                        line!() as i32,
                        "expand_definitions",
                    );
                    return;
                }
            };
            *n = expanded;
        }
        AstNodeType::VAR => {}
    }
}

pub fn replace(n: &mut AstNode, old: &str, new_name: &str) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(lam) = &mut n.node {
                if lam.parameter == old {
                    lam.parameter = new_name.to_string();
                }
                if let Some(body) = lam.body.as_deref_mut() {
                    replace(body, old, new_name);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &mut n.node {
                if let Some(f) = app.function.as_deref_mut() {
                    replace(f, old, new_name);
                }
                if let Some(a) = app.argument.as_deref_mut() {
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

pub fn reduce_ast(table: &mut hash_table::HashTable, n: &AstNode) -> AstNode {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(lam) = &n.node {
                let new_body: Option<Box<AstNode>> = if current_order() == APPLICATIVE {
                    if let Some(body) = lam.body.as_deref() {
                        Some(Box::new(reduce_ast(table, body)))
                    } else {
                        None
                    }
                } else {
                    lam.body.as_deref().map(|b| Box::new(deepcopy(b)))
                };
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: lam.parameter.clone(),
                        type_: lam.type_.clone(),
                        body: new_body,
                    }),
                }
            } else {
                deepcopy(n)
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &n.node {
                let function_node = match app.function.as_deref() {
                    Some(f) => f,
                    None => return deepcopy(n),
                };
                let argument_node = match app.argument.as_deref() {
                    Some(a) => a,
                    None => return deepcopy(n),
                };

                let reduced_func = reduce_ast(table, function_node);
                let reduced_arg = if current_order() == APPLICATIVE {
                    reduce_ast(table, argument_node)
                } else {
                    deepcopy(argument_node)
                };

                if reduced_func.type_ == AstNodeType::LAMBDA_EXPR {
                    if let AstNodeUnion::LambdaExpr(lam) = &reduced_func.node {
                        let body = match lam.body.as_deref() {
                            Some(b) => b,
                            None => return deepcopy(n),
                        };
                        let param = lam.parameter.clone();
                        let reduced = substitute(body, &param, &reduced_arg);

                        if current_order() == APPLICATIVE {
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
            if let AstNodeUnion::Variable(v) = &expression.node {
                if v.name == variable {
                    return deepcopy(replacement);
                }
            }
            deepcopy(expression)
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(lam) = &expression.node {
                let new_body = if let Some(body) = lam.body.as_deref() {
                    substitute(body, variable, replacement)
                } else {
                    return deepcopy(expression);
                };

                if lam.parameter != variable {
                    AstNode {
                        type_: AstNodeType::LAMBDA_EXPR,
                        node: AstNodeUnion::LambdaExpr(LambdaExpression {
                            parameter: lam.parameter.clone(),
                            type_: lam.type_.clone(),
                            body: Some(Box::new(new_body)),
                        }),
                    }
                } else {
                    // The C code returns expression->node.lambda_expr->body;
                    // i.e. it strips the lambda when the parameter equals the
                    // variable being substituted.
                    new_body
                }
            } else {
                deepcopy(expression)
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &expression.node {
                let f_new = match app.function.as_deref() {
                    Some(f) => substitute(f, variable, replacement),
                    None => return deepcopy(expression),
                };
                let a_new = match app.argument.as_deref() {
                    Some(a) => substitute(a, variable, replacement),
                    None => return deepcopy(expression),
                };
                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(f_new)),
                        argument: Some(Box::new(a_new)),
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
            if let AstNodeUnion::Variable(v) = &n.node {
                deepcopy_var(&v.name, &v.type_)
            } else {
                AstNode::default()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(lam) = &n.node {
                let body = lam
                    .body
                    .as_deref()
                    .map(|b| deepcopy(b))
                    .unwrap_or_else(AstNode::default);
                deepcopy_lambda_expr(&lam.parameter, &body, &lam.type_)
            } else {
                AstNode::default()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &n.node {
                let f = app
                    .function
                    .as_deref()
                    .map(|x| deepcopy(x))
                    .unwrap_or_else(AstNode::default);
                let a = app
                    .argument
                    .as_deref()
                    .map(|x| deepcopy(x))
                    .unwrap_or_else(AstNode::default);
                deepcopy_application(&f, &a)
            } else {
                AstNode::default()
            }
        }
        AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(v) = &n.node {
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
