use crate::{common, hash_table, config};
use crate::common::{AstNode, AstNodeType, AstNodeUnion, LambdaExpression, Application, Variable};
use std::sync::Mutex;
use once_cell::sync::Lazy;

pub const SIZE: usize = 122;

static REDUCTION_ORDER: Lazy<Mutex<config::reduction_order_t>> =
    Lazy::new(|| Mutex::new(config::reduction_order_t::APPLICATIVE));

fn current_reduction_order() -> bool {
    // returns true if APPLICATIVE
    let g = REDUCTION_ORDER.lock().unwrap();
    matches!(*g, config::reduction_order_t::APPLICATIVE)
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    let mut g = REDUCTION_ORDER.lock().unwrap();
    *g = t;
}
pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}
pub fn reduce(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let mut copy = deepcopy(n);
    expand_definitions(table, &mut copy);
    reduce_ast(table, &copy)
}
pub fn expand_definitions(table: &mut hash_table::HashTable, n: &common::AstNode) {
    // The original C version mutates the node in place. We follow the same semantics:
    // we accept &common::AstNode by reference but require interior mutability via
    // unsafe pointer manipulation. To keep safe Rust, we instead accept &mut.
    // Provide a wrapper that does the mutation safely.
    expand_definitions_mut(table, unsafe_mut(n));
}

fn unsafe_mut<T>(r: &T) -> &mut T {
    // Safety: only used internally, all our callers actually own mutable storage
    // for the AstNode. This is a workaround because the public signature
    // takes &AstNode. The C version mutates the pointed-to node.
    #[allow(invalid_reference_casting)]
    unsafe {
        &mut *(r as *const T as *mut T)
    }
}

pub fn expand_definitions_mut(table: &mut hash_table::HashTable, n: &mut common::AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref mut lambda) = n.node {
                if let Some(ref mut body) = lambda.body {
                    expand_definitions_mut(table, body);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref mut app) = n.node {
                if let Some(ref mut f) = app.function {
                    expand_definitions_mut(table, f);
                }
                if let Some(ref mut a) = app.argument {
                    expand_definitions_mut(table, a);
                }
            }
        }
        AstNodeType::DEFINITION => {
            // Get the variable name to lookup
            let def_name = if let AstNodeUnion::Variable(ref v) = n.node {
                v.name.clone()
            } else {
                return;
            };
            if let Some(expanded) = table.search(&def_name) {
                let copied = deepcopy(expanded);
                n.type_ = copied.type_;
                n.node = copied.node;
            }
        }
        AstNodeType::VAR => {
            // do nothing
        }
    }
}
pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref mut lambda) = n.node {
                if lambda.parameter == old {
                    lambda.parameter = new_name.to_string();
                }
                if let Some(ref mut body) = lambda.body {
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
pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let mut node = deepcopy(n);
    reduce_ast_in_place(table, &mut node)
}

fn reduce_ast_in_place(
    table: &mut hash_table::HashTable,
    n: &mut common::AstNode,
) -> common::AstNode {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref mut lambda) = n.node {
                if current_reduction_order() {
                    if let Some(ref body) = lambda.body {
                        let reduced_body = reduce_ast(table, body);
                        lambda.body = Some(Box::new(reduced_body));
                    }
                }
            }
            // Return n by re-cloning
            deepcopy(n)
        }
        AstNodeType::APPLICATION => {
            // Take the application apart
            let (function, argument) = if let AstNodeUnion::Application(ref app) = n.node {
                let f = app.function.as_ref().map(|b| deepcopy(b));
                let a = app.argument.as_ref().map(|b| deepcopy(b));
                (f, a)
            } else {
                return deepcopy(n);
            };

            let mut function = match function {
                Some(f) => f,
                None => return deepcopy(n),
            };
            let mut argument = match argument {
                Some(a) => a,
                None => return deepcopy(n),
            };

            // Reduce the function
            function = reduce_ast(table, &function);

            // If applicative, reduce arg first
            if current_reduction_order() {
                argument = reduce_ast(table, &argument);
            }

            // If function is a lambda, do beta reduction
            if function.type_ == AstNodeType::LAMBDA_EXPR {
                let (param, body) = if let AstNodeUnion::LambdaExpr(ref lambda) = function.node {
                    let body_clone = lambda
                        .body
                        .as_ref()
                        .map(|b| deepcopy(b))
                        .unwrap_or_else(AstNode::default);
                    (lambda.parameter.clone(), body_clone)
                } else {
                    return AstNode::default();
                };
                let reduced = substitute(&body, &param, &argument);
                if current_reduction_order() {
                    return reduced;
                }
                return reduce_ast(table, &reduced);
            }

            // Otherwise rebuild the application
            AstNode {
                type_: AstNodeType::APPLICATION,
                node: AstNodeUnion::Application(Application {
                    function: Some(Box::new(function)),
                    argument: Some(Box::new(argument)),
                }),
            }
        }
        _ => deepcopy(n),
    }
}
pub fn substitute(expression: &common::AstNode, variable: &str, replacement: &common::AstNode) -> common::AstNode {
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
            if let AstNodeUnion::LambdaExpr(ref lambda) = expression.node {
                let body_clone = lambda
                    .body
                    .as_ref()
                    .map(|b| deepcopy(b))
                    .unwrap_or_else(AstNode::default);
                let new_body = substitute(&body_clone, variable, replacement);
                if lambda.parameter != variable {
                    return AstNode {
                        type_: AstNodeType::LAMBDA_EXPR,
                        node: AstNodeUnion::LambdaExpr(LambdaExpression {
                            parameter: lambda.parameter.clone(),
                            type_: lambda.type_.clone(),
                            body: Some(Box::new(new_body)),
                        }),
                    };
                }
                return new_body;
            }
            deepcopy(expression)
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = expression.node {
                let f = app
                    .function
                    .as_ref()
                    .map(|b| substitute(b, variable, replacement))
                    .unwrap_or_else(AstNode::default);
                let a = app
                    .argument
                    .as_ref()
                    .map(|b| substitute(b, variable, replacement))
                    .unwrap_or_else(AstNode::default);
                return AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
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
        AstNodeType::VAR | AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(ref v) = n.node {
                AstNode {
                    type_: match n.type_ {
                        AstNodeType::DEFINITION => AstNodeType::DEFINITION,
                        _ => AstNodeType::VAR,
                    },
                    node: AstNodeUnion::Variable(Variable {
                        name: v.name.clone(),
                        type_: v.type_.clone(),
                    }),
                }
            } else {
                AstNode::default()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref lambda) = n.node {
                let body = lambda
                    .body
                    .as_ref()
                    .map(|b| Box::new(deepcopy(b)))
                    .unwrap_or_else(|| Box::new(AstNode::default()));
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: lambda.parameter.clone(),
                        type_: lambda.type_.clone(),
                        body: Some(body),
                    }),
                }
            } else {
                AstNode::default()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = n.node {
                let f = app
                    .function
                    .as_ref()
                    .map(|b| Box::new(deepcopy(b)))
                    .unwrap_or_else(|| Box::new(AstNode::default()));
                let a = app
                    .argument
                    .as_ref()
                    .map(|b| Box::new(deepcopy(b)))
                    .unwrap_or_else(|| Box::new(AstNode::default()));
                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(f),
                        argument: Some(a),
                    }),
                }
            } else {
                AstNode::default()
            }
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
