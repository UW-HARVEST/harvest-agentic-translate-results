use crate::{common, hash_table, config};
use crate::common::{AstNode, AstNodeType, AstNodeUnion, LambdaExpression, Application, Variable};
use std::sync::atomic::{AtomicI32, Ordering};

pub const SIZE: usize = 122;

static REDUCTION_ORDER: AtomicI32 = AtomicI32::new(0); // 0 = APPLICATIVE, 1 = NORMAL

pub fn set_reduction_order(t: config::reduction_order_t) {
    let v = match t {
        config::reduction_order_t::APPLICATIVE => 0,
        config::reduction_order_t::NORMAL => 1,
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

fn current_reduction_order() -> config::reduction_order_t {
    if REDUCTION_ORDER.load(Ordering::SeqCst) == 0 {
        config::reduction_order_t::APPLICATIVE
    } else {
        config::reduction_order_t::NORMAL
    }
}

pub fn reduce(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let mut copy = deepcopy(n);
    expand_definitions(table, &mut copy);
    reduce_ast(table, &copy)
}

pub fn expand_definitions(table: &mut hash_table::HashTable, n: &mut common::AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &mut n.node {
                if let Some(body) = &mut le.body {
                    expand_definitions(table, body);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &mut n.node {
                if let Some(f) = &mut app.function {
                    expand_definitions(table, f);
                }
                if let Some(a) = &mut app.argument {
                    expand_definitions(table, a);
                }
            }
        }
        AstNodeType::DEFINITION => {
            // Get name from the variable
            let def_name: String = if let AstNodeUnion::Variable(v) = &n.node {
                v.name.clone()
            } else {
                return;
            };
            let expanded_def = match table.search(&def_name) {
                Some(d) => deepcopy(d),
                None => return,
            };
            n.type_ = expanded_def.type_;
            n.node = expanded_def.node;
        }
        AstNodeType::VAR => {}
    }
}

pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &mut n.node {
                if le.parameter == old {
                    le.parameter = new_name.to_string();
                }
                if let Some(body) = &mut le.body {
                    replace(body, old, new_name);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &mut n.node {
                if let Some(f) = &mut app.function {
                    replace(f, old, new_name);
                }
                if let Some(a) = &mut app.argument {
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

pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &n.node {
                let order = current_reduction_order();
                let new_body = if matches!(order, config::reduction_order_t::APPLICATIVE) {
                    if let Some(body) = &le.body {
                        Some(Box::new(reduce_ast(table, body)))
                    } else {
                        None
                    }
                } else if let Some(body) = &le.body {
                    Some(Box::new(deepcopy(body)))
                } else {
                    None
                };

                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: le.parameter.clone(),
                        type_: le.type_.clone(),
                        body: new_body,
                    }),
                }
            } else {
                deepcopy(n)
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &n.node {
                let function = match &app.function {
                    Some(f) => reduce_ast(table, f),
                    None => return deepcopy(n),
                };

                let order = current_reduction_order();
                let argument = if matches!(order, config::reduction_order_t::APPLICATIVE) {
                    match &app.argument {
                        Some(a) => reduce_ast(table, a),
                        None => return deepcopy(n),
                    }
                } else {
                    match &app.argument {
                        Some(a) => deepcopy(a),
                        None => return deepcopy(n),
                    }
                };

                if function.type_ == AstNodeType::LAMBDA_EXPR {
                    if let AstNodeUnion::LambdaExpr(le) = &function.node {
                        let param = le.parameter.clone();
                        let body = match &le.body {
                            Some(b) => deepcopy(b),
                            None => return deepcopy(n),
                        };
                        let reduced = substitute(&body, &param, &argument);
                        if matches!(order, config::reduction_order_t::APPLICATIVE) {
                            return reduced;
                        }
                        return reduce_ast(table, &reduced);
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

pub fn substitute(expression: &common::AstNode, variable: &str, replacement: &common::AstNode) -> common::AstNode {
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
            if let AstNodeUnion::LambdaExpr(le) = &expression.node {
                let new_body = match &le.body {
                    Some(b) => substitute(b, variable, replacement),
                    None => return deepcopy(expression),
                };
                if le.parameter != variable {
                    AstNode {
                        type_: AstNodeType::LAMBDA_EXPR,
                        node: AstNodeUnion::LambdaExpr(LambdaExpression {
                            parameter: le.parameter.clone(),
                            type_: le.type_.clone(),
                            body: Some(Box::new(new_body)),
                        }),
                    }
                } else {
                    new_body
                }
            } else {
                deepcopy(expression)
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &expression.node {
                let new_func = match &app.function {
                    Some(f) => substitute(f, variable, replacement),
                    None => return deepcopy(expression),
                };
                let new_arg = match &app.argument {
                    Some(a) => substitute(a, variable, replacement),
                    None => return deepcopy(expression),
                };
                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(new_func)),
                        argument: Some(Box::new(new_arg)),
                    }),
                }
            } else {
                deepcopy(expression)
            }
        }
        AstNodeType::DEFINITION => deepcopy(expression),
    }
}

pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match n.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &n.node {
                deepcopy_var(&v.name, &v.type_)
            } else {
                AstNode::default()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &n.node {
                let body_node = match &le.body {
                    Some(b) => b.as_ref().clone_node(),
                    None => AstNode::default(),
                };
                deepcopy_lambda_expr(&le.parameter, &body_node, &le.type_)
            } else {
                AstNode::default()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &n.node {
                let func_node = match &app.function {
                    Some(f) => f.as_ref().clone_node(),
                    None => AstNode::default(),
                };
                let arg_node = match &app.argument {
                    Some(a) => a.as_ref().clone_node(),
                    None => AstNode::default(),
                };
                deepcopy_application(&func_node, &arg_node)
            } else {
                AstNode::default()
            }
        }
        AstNodeType::DEFINITION => {
            // Same as VAR essentially - has variable underneath
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

trait CloneNode {
    fn clone_node(&self) -> AstNode;
}

impl CloneNode for AstNode {
    fn clone_node(&self) -> AstNode {
        deepcopy(self)
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
