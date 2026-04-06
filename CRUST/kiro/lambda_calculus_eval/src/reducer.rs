use crate::{common, hash_table, parser, config};
use crate::common::{AstNode, AstNodeType, AstNodeUnion, LambdaExpression, Application, Variable};
use std::sync::atomic::{AtomicU8, Ordering};

pub const SIZE: usize = 122;

// 0 = APPLICATIVE, 1 = NORMAL
static REDUCTION_ORDER: AtomicU8 = AtomicU8::new(0);

fn is_applicative() -> bool {
    REDUCTION_ORDER.load(Ordering::SeqCst) == 0
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => REDUCTION_ORDER.store(0, Ordering::SeqCst),
        config::reduction_order_t::NORMAL => REDUCTION_ORDER.store(1, Ordering::SeqCst),
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
    let mut copy = deepcopy(n);
    common::print_verbose("Order of reduction is: ", format_args!(""));
    if is_applicative() {
        print_reduction_order(config::reduction_order_t::APPLICATIVE);
    } else {
        print_reduction_order(config::reduction_order_t::NORMAL);
    }
    common::print_verbose("-------------------------------------------\n", format_args!("-------------------------------------------\n"));
    expand_definitions(table, &mut copy);
    common::print_verbose("Expanded expression:\n", format_args!("Expanded expression:\n"));
    common::print_ast_verbose(&copy);
    let reduced = reduce_ast(table, &copy);
    common::print_verbose("Final reduced expression:\n", format_args!("Final reduced expression:\n"));
    common::print_ast_verbose(&reduced);
    common::print_verbose("-------------------------------------------\n", format_args!("-------------------------------------------\n"));
    reduced
}
pub fn expand_definitions(table: &mut hash_table::HashTable, n: &common::AstNode) {
    // The C code mutates n in-place. The test passes &mut which coerces to &.
    // We use a raw pointer to perform the mutation as the C code does.
    let n_ptr = n as *const common::AstNode as *mut common::AstNode;
    unsafe {
        match &(*n_ptr).type_ {
            AstNodeType::LAMBDA_EXPR => {
                if let AstNodeUnion::LambdaExpr(ref le) = (*n_ptr).node {
                    if let Some(ref body) = le.body {
                        expand_definitions(table, body);
                    }
                }
            }
            AstNodeType::APPLICATION => {
                if let AstNodeUnion::Application(ref app) = (*n_ptr).node {
                    if let Some(ref f) = app.function {
                        expand_definitions(table, f);
                    }
                    if let Some(ref a) = app.argument {
                        expand_definitions(table, a);
                    }
                }
            }
            AstNodeType::DEFINITION => {
                if let AstNodeUnion::Variable(ref var) = (*n_ptr).node {
                    let def_name = var.name.clone();
                    if let Some(expanded) = table.search(&def_name) {
                        let copy = deepcopy(expanded);
                        common::print_verbose("Expanding definition of: ", format_args!("Expanding definition of: {} . Term expanded to:\n", def_name));
                        common::print_ast_verbose(&copy);
                        (*n_ptr).type_ = copy.type_;
                        (*n_ptr).node = copy.node;
                    }
                }
            }
            _ => {}
        }
    }
}
pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match &mut n.node {
        AstNodeUnion::LambdaExpr(le) => {
            if le.parameter == old {
                le.parameter = new_name.to_string();
            }
            if let Some(ref mut body) = le.body {
                replace(body, old, new_name);
            }
        }
        AstNodeUnion::Application(app) => {
            if let Some(ref mut f) = app.function {
                replace(f, old, new_name);
            }
            if let Some(ref mut a) = app.argument {
                replace(a, old, new_name);
            }
        }
        AstNodeUnion::Variable(var) => {
            if var.name == old {
                var.name = new_name.to_string();
            }
        }
    }
}
pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    match &n.node {
        AstNodeUnion::LambdaExpr(le) => {
            let mut result = deepcopy(n);
            if is_applicative() {
                if let AstNodeUnion::LambdaExpr(ref mut rle) = result.node {
                    if let Some(ref body) = rle.body {
                        let reduced_body = reduce_ast(table, body);
                        rle.body = Some(Box::new(reduced_body));
                    }
                }
            }
            result
        }
        AstNodeUnion::Application(app) => {
            let reduced_func = if let Some(ref f) = app.function {
                reduce_ast(table, f)
            } else {
                return deepcopy(n);
            };

            let reduced_arg = if is_applicative() {
                if let Some(ref a) = app.argument {
                    reduce_ast(table, a)
                } else {
                    AstNode::default()
                }
            } else {
                if let Some(ref a) = app.argument {
                    deepcopy(a)
                } else {
                    AstNode::default()
                }
            };

            if reduced_func.type_ == AstNodeType::LAMBDA_EXPR {
                if let AstNodeUnion::LambdaExpr(ref le) = reduced_func.node {
                    let param = le.parameter.clone();
                    if let Some(ref body) = le.body {
                        let reduced = substitute(body, &param, &reduced_arg);
                        common::print_verbose("Applied substitution", format_args!("Applied substitution to lambda expr of parameter <{}> and resulted in:\n", param));
                        common::print_ast_verbose(&reduced);
                        if is_applicative() {
                            return reduced;
                        }
                        return reduce_ast(table, &reduced);
                    }
                }
            }
            AstNode {
                type_: AstNodeType::APPLICATION,
                node: AstNodeUnion::Application(Application {
                    function: Some(Box::new(reduced_func)),
                    argument: Some(Box::new(reduced_arg)),
                }),
            }
        }
        _ => deepcopy(n),
    }
}
pub fn substitute(expression: &common::AstNode, variable: &str, replacement: &common::AstNode) -> common::AstNode {
    match &expression.node {
        AstNodeUnion::Variable(var) => {
            if var.name == variable {
                return deepcopy(replacement);
            }
            deepcopy(expression)
        }
        AstNodeUnion::LambdaExpr(le) => {
            let new_body = if let Some(ref body) = le.body {
                Some(Box::new(substitute(body, variable, replacement)))
            } else {
                None
            };
            if le.parameter != variable {
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: le.parameter.clone(),
                        type_: le.type_.clone(),
                        body: new_body,
                    }),
                }
            } else {
                // parameter == variable, return the body (unwrap the lambda)
                match new_body {
                    Some(b) => *b,
                    None => AstNode::default(),
                }
            }
        }
        AstNodeUnion::Application(app) => {
            let new_func = if let Some(ref f) = app.function {
                Some(Box::new(substitute(f, variable, replacement)))
            } else {
                None
            };
            let new_arg = if let Some(ref a) = app.argument {
                Some(Box::new(substitute(a, variable, replacement)))
            } else {
                None
            };
            AstNode {
                type_: AstNodeType::APPLICATION,
                node: AstNodeUnion::Application(Application {
                    function: new_func,
                    argument: new_arg,
                }),
            }
        }
    }
}
pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    match &n.node {
        AstNodeUnion::Variable(v) => deepcopy_var(&v.name, &v.type_),
        AstNodeUnion::LambdaExpr(le) => {
            if let Some(ref body) = le.body {
                deepcopy_lambda_expr(&le.parameter, body, &le.type_)
            } else {
                AstNode {
                    type_: AstNodeType::LAMBDA_EXPR,
                    node: AstNodeUnion::LambdaExpr(LambdaExpression {
                        parameter: le.parameter.clone(),
                        type_: le.type_.clone(),
                        body: None,
                    }),
                }
            }
        }
        AstNodeUnion::Application(app) => {
            let default_f = AstNode::default();
            let default_a = AstNode::default();
            let f = app.function.as_ref().map(|f| f.as_ref()).unwrap_or(&default_f);
            let a = app.argument.as_ref().map(|a| a.as_ref()).unwrap_or(&default_a);
            deepcopy_application(f, a)
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
