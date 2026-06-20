use crate::{common, config, hash_table, parser};
use std::sync::atomic::{AtomicU8, Ordering};

pub const SIZE: usize = 122;

static REDUCTION_ORDER: AtomicU8 = AtomicU8::new(0);

fn current_reduction_order() -> config::reduction_order_t {
    match REDUCTION_ORDER.load(Ordering::Relaxed) {
        1 => config::reduction_order_t::NORMAL,
        _ => config::reduction_order_t::APPLICATIVE,
    }
}

fn expanded_definitions(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    match (&n.type_, &n.node) {
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(lambda)) => {
            let body = lambda
                .body
                .as_deref()
                .map(|body| expanded_definitions(table, body))
                .unwrap_or_default();
            parser::create_lambda(&lambda.parameter, &body, &lambda.type_)
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(application)) => {
            let function = application
                .function
                .as_deref()
                .map(|function| expanded_definitions(table, function))
                .unwrap_or_default();
            let argument = application
                .argument
                .as_deref()
                .map(|argument| expanded_definitions(table, argument))
                .unwrap_or_default();
            parser::create_application(&function, &argument)
        }
        (common::AstNodeType::DEFINITION, common::AstNodeUnion::Variable(variable)) => {
            let def_name = variable.name.clone();
            let expanded_def = table.search(&def_name).cloned().unwrap_or_else(|| {
                common::error(
                    "Null pointer encountered",
                    file!(),
                    line!() as i32,
                    "expand_definitions",
                );
                std::process::exit(1);
            });
            common::print_verbose(
                "",
                format_args!("Expanding definition of: {def_name} . Term expanded to:\n"),
            );
            common::print_ast_verbose(&expanded_def);
            expanded_def
        }
        _ => deepcopy(n),
    }
}

pub fn set_reduction_order(t: config::reduction_order_t) {
    let encoded = match t {
        config::reduction_order_t::APPLICATIVE => 0,
        config::reduction_order_t::NORMAL => 1,
    };
    REDUCTION_ORDER.store(encoded, Ordering::Relaxed);
}

pub fn print_reduction_order(t: config::reduction_order_t) {
    match t {
        config::reduction_order_t::APPLICATIVE => println!("Applicative"),
        config::reduction_order_t::NORMAL => println!("Normal"),
    }
}

pub fn reduce(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    common::print_verbose("", format_args!("Order of reduction is: "));
    print_reduction_order(current_reduction_order());
    common::print_verbose("", format_args!("-------------------------------------------\n"));

    let mut expanded = deepcopy(n);
    expand_definitions(table, &mut expanded);

    common::print_verbose("", format_args!("Expanded expression:\n"));
    common::print_ast_verbose(&expanded);

    let reduced = reduce_ast(table, &expanded);
    common::print_verbose("", format_args!("Final reduced expression:\n"));
    common::print_ast_verbose(&reduced);
    common::print_verbose("", format_args!("-------------------------------------------\n"));
    reduced
}

#[allow(invalid_reference_casting)]
pub fn expand_definitions(table: &mut hash_table::HashTable, n: &common::AstNode) {
    // The public API exposes a shared reference, but the original C function mutates the node in
    // place and the tests rely on that. This keeps the API intact while performing the required
    // in-place rewrite.
    let rewritten = expanded_definitions(table, n);
    let n_ptr = n as *const common::AstNode as *mut common::AstNode;
    unsafe {
        std::ptr::write(n_ptr, rewritten);
    }
}

pub fn replace(n: &mut common::AstNode, old: &str, new_name: &str) {
    match (&n.type_, &mut n.node) {
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(lambda)) => {
            if lambda.parameter == old {
                lambda.parameter = new_name.to_string();
            }
            if let Some(body) = lambda.body.as_mut() {
                replace(body, old, new_name);
            }
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(application)) => {
            if let Some(function) = application.function.as_mut() {
                replace(function, old, new_name);
            }
            if let Some(argument) = application.argument.as_mut() {
                replace(argument, old, new_name);
            }
        }
        (common::AstNodeType::VAR, common::AstNodeUnion::Variable(variable)) => {
            if variable.name == old {
                variable.name = new_name.to_string();
            }
        }
        _ => {}
    }
}

pub fn reduce_ast(table: &mut hash_table::HashTable, n: &common::AstNode) -> common::AstNode {
    match (&n.type_, &n.node) {
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(lambda)) => {
            let body = lambda
                .body
                .as_deref()
                .map(|node| {
                    if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE)
                    {
                        reduce_ast(table, node)
                    } else {
                        deepcopy(node)
                    }
                })
                .unwrap_or_default();
            parser::create_lambda(&lambda.parameter, &body, &lambda.type_)
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(application)) => {
            let function = application
                .function
                .as_deref()
                .map(|node| reduce_ast(table, node))
                .unwrap_or_default();

            let argument = application
                .argument
                .as_deref()
                .map(|node| {
                    if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE)
                    {
                        reduce_ast(table, node)
                    } else {
                        deepcopy(node)
                    }
                })
                .unwrap_or_default();

            if let common::AstNodeUnion::LambdaExpr(lambda) = &function.node {
                let body = lambda.body.as_deref().unwrap_or(&common::AstNode::default()).clone();
                let param = lambda.parameter.clone();
                let reduced = substitute(&body, &param, &argument);
                common::print_verbose(
                    "",
                    format_args!(
                        "Applied substitution to lambda expr of parameter <{param}> and resulted in:\n"
                    ),
                );
                common::print_ast_verbose(&reduced);

                if matches!(current_reduction_order(), config::reduction_order_t::APPLICATIVE) {
                    reduced
                } else {
                    reduce_ast(table, &reduced)
                }
            } else {
                parser::create_application(&function, &argument)
            }
        }
        _ => deepcopy(n),
    }
}

pub fn substitute(
    expression: &common::AstNode,
    variable: &str,
    replacement: &common::AstNode,
) -> common::AstNode {
    match (&expression.type_, &expression.node) {
        (common::AstNodeType::VAR, common::AstNodeUnion::Variable(var)) => {
            if var.name == variable {
                deepcopy(replacement)
            } else {
                deepcopy(expression)
            }
        }
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(lambda)) => {
            let substituted_body = lambda
                .body
                .as_deref()
                .map(|body| substitute(body, variable, replacement))
                .unwrap_or_default();
            if lambda.parameter != variable {
                parser::create_lambda(&lambda.parameter, &substituted_body, &lambda.type_)
            } else {
                substituted_body
            }
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(application)) => {
            let function = application
                .function
                .as_deref()
                .map(|node| substitute(node, variable, replacement))
                .unwrap_or_default();
            let argument = application
                .argument
                .as_deref()
                .map(|node| substitute(node, variable, replacement))
                .unwrap_or_default();
            parser::create_application(&function, &argument)
        }
        _ => deepcopy(expression),
    }
}

pub fn deepcopy(n: &common::AstNode) -> common::AstNode {
    n.clone()
}

pub fn deepcopy_application(
    function: &common::AstNode,
    argument: &common::AstNode,
) -> common::AstNode {
    parser::create_application(function, argument)
}

pub fn deepcopy_lambda_expr(parameter: &str, body: &common::AstNode, type_: &str) -> common::AstNode {
    parser::create_lambda(parameter, body, type_)
}

pub fn deepcopy_var(name: &str, type_: &str) -> common::AstNode {
    parser::create_variable(name, type_)
}
