use crate::{common, hash_table, config};
use std::sync::Mutex;

use common::{AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable};

pub const SIZE: usize = 122;

#[derive(Clone, Copy)]
enum Order {
    Applicative,
    Normal,
}

static REDUCTION_ORDER: Mutex<Order> = Mutex::new(Order::Applicative);

pub fn set_reduction_order(t: config::reduction_order_t) {
    let new_order = match t {
        config::reduction_order_t::APPLICATIVE => Order::Applicative,
        config::reduction_order_t::NORMAL => Order::Normal,
    };
    *REDUCTION_ORDER.lock().unwrap() = new_order;
}

fn current_order() -> Order {
    *REDUCTION_ORDER.lock().unwrap()
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
    // The C version mutates the node in place. The requested signature takes
    // &AstNode, so we use a raw pointer to perform the in-place mutation that
    // the C semantics require. This is safe in this context because callers
    // own the AstNode behind the reference.
    let raw = n as *const common::AstNode as *mut common::AstNode;
    unsafe {
        expand_definitions_inner(table, raw);
    }
}

unsafe fn expand_definitions_inner(table: &hash_table::HashTable, n_ptr: *mut common::AstNode) {
    let n = &mut *n_ptr;
    expand_definitions_mut(table, n);
}

// Mutating variant used internally.
pub(crate) fn expand_definitions_mut(table: &hash_table::HashTable, n: &mut common::AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref mut le) = n.node {
                if let Some(ref mut body) = le.body {
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
            let def_name = if let AstNodeUnion::Variable(ref v) = n.node {
                v.name.clone()
            } else {
                String::new()
            };
            if let Some(expanded) = table.search(&def_name) {
                let copy = deepcopy(expanded);
                n.type_ = copy.type_;
                n.node = copy.node;
            }
        }
        AstNodeType::VAR => {}
    }
}

pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref mut le) = n.node {
                if le.parameter == old {
                    le.parameter = new_name.to_string();
                }
                if let Some(ref mut body) = le.body {
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
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref le) = n.node {
                let parameter = le.parameter.clone();
                let type_ = le.type_.clone();
                let body_ref = le.body.as_deref();
                let new_body = match body_ref {
                    Some(b) => {
                        if matches!(current_order(), Order::Applicative) {
                            reduce_ast(table, b)
                        } else {
                            deepcopy(b)
                        }
                    }
                    None => common::AstNode::default(),
                };
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter,
                        type_,
                        body: Some(Box::new(new_body)),
                    }),
                }
            } else {
                deepcopy(n)
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = n.node {
                let function_ref = app.function.as_deref();
                let argument_ref = app.argument.as_deref();
                let reduced_function = match function_ref {
                    Some(f) => reduce_ast(table, f),
                    None => return deepcopy(n),
                };
                let reduced_argument = match argument_ref {
                    Some(a) => {
                        if matches!(current_order(), Order::Applicative) {
                            reduce_ast(table, a)
                        } else {
                            deepcopy(a)
                        }
                    }
                    None => return deepcopy(n),
                };

                if reduced_function.type_ == AstNodeType::LAMBDA_EXPR {
                    if let AstNodeUnion::LambdaExpr(ref le) = reduced_function.node {
                        let param = le.parameter.clone();
                        let body_ref = le.body.as_deref();
                        let body_copy = match body_ref {
                            Some(b) => deepcopy(b),
                            None => common::AstNode::default(),
                        };
                        let reduced = substitute(&body_copy, &param, &reduced_argument);

                        if matches!(current_order(), Order::Applicative) {
                            return reduced;
                        }
                        return reduce_ast(table, &reduced);
                    }
                }

                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(reduced_function)),
                        argument: Some(Box::new(reduced_argument)),
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
            if let AstNodeUnion::Variable(ref v) = expression.node {
                if v.name == variable {
                    return deepcopy(replacement);
                }
            }
            deepcopy(expression)
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref le) = expression.node {
                let new_body = match le.body.as_deref() {
                    Some(b) => substitute(b, variable, replacement),
                    None => common::AstNode::default(),
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
            if let AstNodeUnion::Application(ref app) = expression.node {
                let new_f = match app.function.as_deref() {
                    Some(f) => substitute(f, variable, replacement),
                    None => common::AstNode::default(),
                };
                let new_a = match app.argument.as_deref() {
                    Some(a) => substitute(a, variable, replacement),
                    None => common::AstNode::default(),
                };
                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(new_f)),
                        argument: Some(Box::new(new_a)),
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
            if let AstNodeUnion::Variable(ref v) = n.node {
                deepcopy_var(&v.name, &v.type_)
            } else {
                common::AstNode::default()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref le) = n.node {
                let body_node = match le.body.as_deref() {
                    Some(b) => b.clone_node(),
                    None => common::AstNode::default(),
                };
                deepcopy_lambda_expr(&le.parameter, &body_node, &le.type_)
            } else {
                common::AstNode::default()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = n.node {
                let f = match app.function.as_deref() {
                    Some(f) => f.clone_node(),
                    None => common::AstNode::default(),
                };
                let a = match app.argument.as_deref() {
                    Some(a) => a.clone_node(),
                    None => common::AstNode::default(),
                };
                deepcopy_application(&f, &a)
            } else {
                common::AstNode::default()
            }
        }
        AstNodeType::DEFINITION => {
            // copy as a definition variable
            if let AstNodeUnion::Variable(ref v) = n.node {
                AstNode {
                    type_: AstNodeType::DEFINITION,
                    node: AstNodeUnion::Variable(Variable {
                        name: v.name.clone(),
                        type_: v.type_.clone(),
                    }),
                }
            } else {
                common::AstNode::default()
            }
        }
    }
}

trait CloneNode {
    fn clone_node(&self) -> common::AstNode;
}
impl CloneNode for common::AstNode {
    fn clone_node(&self) -> common::AstNode {
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
