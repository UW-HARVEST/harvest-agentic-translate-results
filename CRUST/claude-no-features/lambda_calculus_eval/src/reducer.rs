use crate::common::{self, AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable};
use crate::config;
use crate::hash_table;
use std::cell::Cell;

pub const SIZE: usize = 122;

thread_local! {
    static REDUCTION_ORDER: Cell<u8> = Cell::new(0); // 0 = APPLICATIVE, 1 = NORMAL
}

fn is_applicative() -> bool {
    REDUCTION_ORDER.with(|c| c.get() == 0)
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    let v = match t {
        config::reduction_order_t::APPLICATIVE => 0,
        config::reduction_order_t::NORMAL => 1,
    };
    REDUCTION_ORDER.with(|c| c.set(v));
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
            let def_name = if let AstNodeUnion::Variable(v) = &n.node {
                v.name.clone()
            } else {
                return;
            };
            let copy = if let Some(expanded_def) = table.search(&def_name) {
                deepcopy(expanded_def)
            } else {
                eprintln!(
                    "ERROR: Null pointer encountered: definition '{}' not found",
                    def_name
                );
                std::process::exit(1);
            };
            n.type_ = copy.type_;
            n.node = copy.node;
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

pub fn reduce_ast(table: &mut hash_table::HashTable, n: &AstNode) -> AstNode {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(le) = &n.node {
                let body = le.body.as_ref().expect("lambda has no body");
                let new_body = if is_applicative() {
                    reduce_ast(table, body)
                } else {
                    deepcopy(body)
                };
                return AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: le.parameter.clone(),
                        type_: le.type_.clone(),
                        body: Some(Box::new(new_body)),
                    }),
                };
            }
            deepcopy(n)
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &n.node {
                let f = app.function.as_ref().expect("application has no function");
                let a = app.argument.as_ref().expect("application has no argument");

                let new_f = reduce_ast(table, f);
                let new_a = if is_applicative() {
                    reduce_ast(table, a)
                } else {
                    deepcopy(a)
                };

                if new_f.type_ == AstNodeType::LAMBDA_EXPR {
                    if let AstNodeUnion::LambdaExpr(le) = &new_f.node {
                        let param = le.parameter.clone();
                        let body = le.body.as_ref().expect("lambda has no body");
                        let reduced = substitute(body, &param, &new_a);
                        if is_applicative() {
                            return reduced;
                        }
                        return reduce_ast(table, &reduced);
                    }
                }
                return AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(new_f)),
                        argument: Some(Box::new(new_a)),
                    }),
                };
            }
            deepcopy(n)
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
            if let AstNodeUnion::LambdaExpr(le) = &expression.node {
                let body = le.body.as_ref().expect("lambda has no body");
                let new_body = substitute(body, variable, replacement);
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
            deepcopy(expression)
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &expression.node {
                let f = app.function.as_ref().expect("application has no function");
                let a = app.argument.as_ref().expect("application has no argument");
                let new_f = substitute(f, variable, replacement);
                let new_a = substitute(a, variable, replacement);
                return AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(new_f)),
                        argument: Some(Box::new(new_a)),
                    }),
                };
            }
            deepcopy(expression)
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
            if let AstNodeUnion::LambdaExpr(le) = &n.node {
                let body = le.body.as_ref().map(|b| b.as_ref());
                match body {
                    Some(b) => deepcopy_lambda_expr(&le.parameter, b, &le.type_),
                    None => AstNode {
                        type_: AstNodeType::LAMBDA_EXPR,
                        node: AstNodeUnion::LambdaExpr(LambdaExpression {
                            parameter: le.parameter.clone(),
                            type_: le.type_.clone(),
                            body: None,
                        }),
                    },
                }
            } else {
                AstNode::default()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &n.node {
                match (app.function.as_ref(), app.argument.as_ref()) {
                    (Some(f), Some(a)) => deepcopy_application(f, a),
                    _ => AstNode {
                        type_: AstNodeType::APPLICATION,
                        node: AstNodeUnion::Application(Application {
                            function: app.function.as_ref().map(|f| Box::new(deepcopy(f))),
                            argument: app.argument.as_ref().map(|a| Box::new(deepcopy(a))),
                        }),
                    },
                }
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
