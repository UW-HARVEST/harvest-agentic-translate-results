use crate::common::{self, AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable};
use crate::config;
use crate::hash_table;
#[allow(unused_imports)]
use crate::parser;
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

pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => print!("Applicative"),
        config::reduction_order_t::NORMAL => print!("Normal"),
    }
    println!();
}

fn is_applicative() -> bool {
    REDUCTION_ORDER.load(Ordering::SeqCst) == 0
}

pub fn reduce(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let mut copy = deepcopy(n);
    expand_definitions(table, &mut copy);
    reduce_ast(table, &copy)
}

pub fn expand_definitions(table: &mut hash_table::HashTable, n: &mut common::AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref mut l) = n.node {
                if let Some(ref mut body) = l.body {
                    expand_definitions(table, body);
                }
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref mut a) = n.node {
                if let Some(ref mut f) = a.function {
                    expand_definitions(table, f);
                }
                if let Some(ref mut arg) = a.argument {
                    expand_definitions(table, arg);
                }
            }
        }
        AstNodeType::DEFINITION => {
            let def_name = if let AstNodeUnion::Variable(ref v) = n.node {
                v.name.clone()
            } else {
                return;
            };
            if let Some(expanded_def) = table.search(&def_name) {
                let cloned = expanded_def.clone();
                n.type_ = cloned.type_;
                n.node = cloned.node;
            } else {
                eprintln!("ERROR: Null pointer encountered: definition {} not found", def_name);
                std::process::exit(1);
            }
        }
        _ => {}
    }
}

pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
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
            if let AstNodeUnion::Application(ref mut a) = n.node {
                if let Some(ref mut f) = a.function {
                    replace(f, old, new_name);
                }
                if let Some(ref mut arg) = a.argument {
                    replace(arg, old, new_name);
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
        _ => {}
    }
}

fn reduce_ast_in_place(table: &mut hash_table::HashTable, n: &mut common::AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if is_applicative() {
                if let AstNodeUnion::LambdaExpr(ref mut l) = n.node {
                    if let Some(ref mut body) = l.body {
                        reduce_ast_in_place(table, body);
                    }
                }
            }
        }
        AstNodeType::APPLICATION => {
            // First reduce the function
            let (function_is_lambda, lambda_param, lambda_body, argument_clone) = {
                if let AstNodeUnion::Application(ref mut a) = n.node {
                    if let Some(ref mut f) = a.function {
                        reduce_ast_in_place(table, f);
                    }
                    if is_applicative() {
                        if let Some(ref mut arg) = a.argument {
                            reduce_ast_in_place(table, arg);
                        }
                    }

                    let func_is_lambda = match a.function {
                        Some(ref f) => f.type_ == AstNodeType::LAMBDA_EXPR,
                        None => false,
                    };

                    if func_is_lambda {
                        let (param, body) = if let Some(ref f) = a.function {
                            if let AstNodeUnion::LambdaExpr(ref l) = f.node {
                                let body = l.body.as_ref().map(|b| (**b).clone());
                                (l.parameter.clone(), body)
                            } else {
                                (String::new(), None)
                            }
                        } else {
                            (String::new(), None)
                        };
                        let arg_clone = a.argument.as_ref().map(|x| (**x).clone());
                        (true, param, body, arg_clone)
                    } else {
                        (false, String::new(), None, None)
                    }
                } else {
                    (false, String::new(), None, None)
                }
            };

            if function_is_lambda {
                if let (Some(body), Some(arg)) = (lambda_body, argument_clone) {
                    let reduced = substitute(&body, &lambda_param, &arg);
                    if is_applicative() {
                        *n = reduced;
                    } else {
                        let mut r = reduced;
                        reduce_ast_in_place(table, &mut r);
                        *n = r;
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    let mut copy = deepcopy(n);
    reduce_ast_in_place(table, &mut copy);
    copy
}

pub fn substitute(
    expression: &common::AstNode,
    variable: &str,
    replacement: &common::AstNode,
) -> common::AstNode {
    match expression.type_ {
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(ref v) = expression.node {
                if v.name == variable {
                    return deepcopy(replacement);
                }
            }
            expression.clone()
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref l) = expression.node {
                let new_body = if let Some(ref b) = l.body {
                    substitute(b, variable, replacement)
                } else {
                    AstNode::default()
                };
                if l.parameter != variable {
                    AstNode {
                        type_: AstNodeType::LAMBDA_EXPR,
                        node: AstNodeUnion::LambdaExpr(LambdaExpression {
                            parameter: l.parameter.clone(),
                            type_: l.type_.clone(),
                            body: Some(Box::new(new_body)),
                        }),
                    }
                } else {
                    new_body
                }
            } else {
                expression.clone()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref a) = expression.node {
                let new_func = match a.function {
                    Some(ref f) => substitute(f, variable, replacement),
                    None => AstNode::default(),
                };
                let new_arg = match a.argument {
                    Some(ref arg) => substitute(arg, variable, replacement),
                    None => AstNode::default(),
                };
                AstNode {
                    type_: AstNodeType::APPLICATION,
                    node: AstNodeUnion::Application(Application {
                        function: Some(Box::new(new_func)),
                        argument: Some(Box::new(new_arg)),
                    }),
                }
            } else {
                expression.clone()
            }
        }
        _ => expression.clone(),
    }
}

pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match n.type_ {
        AstNodeType::VAR | AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(ref v) = n.node {
                let mut copy = deepcopy_var(&v.name, &v.type_);
                copy.type_ = n.type_.clone();
                copy
            } else {
                n.clone()
            }
        }
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref l) = n.node {
                let body = match l.body {
                    Some(ref b) => b.as_ref().clone(),
                    None => AstNode::default(),
                };
                deepcopy_lambda_expr(&l.parameter, &body, &l.type_)
            } else {
                n.clone()
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref a) = n.node {
                let func = match a.function {
                    Some(ref f) => f.as_ref().clone(),
                    None => AstNode::default(),
                };
                let arg = match a.argument {
                    Some(ref ar) => ar.as_ref().clone(),
                    None => AstNode::default(),
                };
                deepcopy_application(&func, &arg)
            } else {
                n.clone()
            }
        }
    }
}

pub fn deepcopy_application(
    function: &common::AstNode,
    argument: &common::AstNode,
) -> common::AstNode {
    AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
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
