use crate::{common, hash_table, io as crate_io, reducer};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicI64, Ordering};

static ALPHA_N: AtomicI64 = AtomicI64::new(1);

pub fn parse_token(token: char) -> common::tokens_t {
    if token == '(' {
        common::tokens_t::L_PAREN
    } else if token == ')' {
        common::tokens_t::R_PAREN
    } else if token == '@' {
        common::tokens_t::LAMBDA
    } else if token == '.' {
        common::tokens_t::DOT
    } else if is_variable(token) {
        common::tokens_t::VARIABLE
    } else if token == ' ' {
        common::tokens_t::WHITESPACE
    } else if token == '\n' {
        common::tokens_t::NEWLINE
    } else if token == '=' {
        common::tokens_t::EQ
    } else if token == '"' {
        common::tokens_t::QUOTE
    } else if token == ':' {
        common::tokens_t::COLON
    } else {
        common::tokens_t::ERROR
    }
}

pub fn p_print_token(token: common::tokens_t) {
    match token {
        common::tokens_t::L_PAREN => print!("( "),
        common::tokens_t::R_PAREN => print!(") "),
        common::tokens_t::LAMBDA => print!("@ "),
        common::tokens_t::DOT => print!(". "),
        common::tokens_t::VARIABLE => print!("VARIABLE "),
        common::tokens_t::WHITESPACE => print!("WHITESPACE "),
        common::tokens_t::NEWLINE => print!("NEWLINE "),
        common::tokens_t::EQ => print!("= "),
        _ => print!("ERROR "),
    }
}

pub fn p_print_astNode_type(n: &common::AstNode) {
    match n.type_ {
        common::AstNodeType::LAMBDA_EXPR => println!("AstNode Type: LAMBDA_EXPR"),
        common::AstNodeType::APPLICATION => println!("AstNode Type: APPLICATION"),
        common::AstNodeType::VAR => println!("AstNode Type: VAR"),
        common::AstNodeType::DEFINITION => println!("AstNode Type: DEFINITION"),
    }
}

pub fn print_ast(node: &common::AstNode) {
    if common::is_null_ast(node) {
        return;
    }
    match node.type_ {
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(le) = &node.node {
                print!("(LAMBDA {} : {}", le.parameter, le.type_);
                if let Some(body) = &le.body {
                    print_ast(body);
                }
                print!(") ");
            }
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(app) = &node.node {
                print!("(APP ");
                if let Some(f) = &app.function {
                    print_ast(f);
                }
                if let Some(a) = &app.argument {
                    print_ast(a);
                }
                print!(") ");
            }
        }
        common::AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(v) = &node.node {
                print!("(VAR {} ", v.name);
                if !v.type_.is_empty() {
                    print!(": {}", v.type_);
                }
                print!(")");
            }
        }
        common::AstNodeType::DEFINITION => {
            if let common::AstNodeUnion::Variable(v) = &node.node {
                print!("(DEFINITION {}) ", v.name);
            }
        }
    }
}

pub fn is_variable(token: char) -> bool {
    let c = token as i32;
    if c == ('_' as i32) {
        return true;
    }
    (c >= 97 && c <= 122) || (c >= 65 && c <= 90)
}

/// Peek at the next character in the file without consuming it.
/// Returns '\u{FFFF}' on EOF.
pub fn peek(in_: &mut File) -> char {
    let mut buf = [0u8; 1];
    match in_.read(&mut buf) {
        Ok(0) => '\u{FFFF}',
        Ok(_) => {
            // Seek back one byte
            let _ = in_.seek(SeekFrom::Current(-1));
            buf[0] as char
        }
        Err(_) => '\u{FFFF}',
    }
}

pub fn peek_print(in_: &mut File, n: usize) {
    let mut buffer: Vec<u8> = Vec::with_capacity(n);
    let mut count = 0usize;
    let mut tmp = [0u8; 1];
    for _ in 0..n {
        match in_.read(&mut tmp) {
            Ok(0) => break,
            Ok(_) => {
                buffer.push(tmp[0]);
                count += 1;
            }
            Err(_) => break,
        }
    }
    let s: String = buffer.iter().map(|b| *b as char).collect();
    print!("{}", s);
    // Seek back the count bytes
    if count > 0 {
        let _ = in_.seek(SeekFrom::Current(-(count as i64)));
    }
}

pub fn consume(t: common::tokens_t, in_: &mut File, expected: &str) {
    let c = match crate_io::next(in_) {
        Ok(ch) => ch,
        Err(_) => '\u{FFFF}',
    };
    let p = parse_token(c);
    if p != t {
        expect(expected, c);
    }
}

pub fn create_variable(name: &str, type_: &str) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}

pub fn create_application(function: &common::AstNode, argument: &common::AstNode) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::APPLICATION,
        node: common::AstNodeUnion::Application(common::Application {
            function: Some(Box::new(deepcopy_node(function))),
            argument: Some(Box::new(deepcopy_node(argument))),
        }),
    }
}

pub fn create_lambda(variable: &str, body: &common::AstNode, type_: &str) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::LAMBDA_EXPR,
        node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
            parameter: variable.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(deepcopy_node(body))),
        }),
    }
}

/// Helper to deep-copy a node (used by create_application/create_lambda).
fn deepcopy_node(n: &common::AstNode) -> common::AstNode {
    if common::is_null_ast(n) {
        return common::AstNode::default();
    }
    match &n.node {
        common::AstNodeUnion::Variable(v) => common::AstNode {
            type_: match n.type_ {
                common::AstNodeType::DEFINITION => common::AstNodeType::DEFINITION,
                _ => common::AstNodeType::VAR,
            },
            node: common::AstNodeUnion::Variable(common::Variable {
                name: v.name.clone(),
                type_: v.type_.clone(),
            }),
        },
        common::AstNodeUnion::LambdaExpr(le) => {
            let body = match &le.body {
                Some(b) => Some(Box::new(deepcopy_node(b))),
                None => None,
            };
            common::AstNode {
                type_: common::AstNodeType::LAMBDA_EXPR,
                node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
                    parameter: le.parameter.clone(),
                    type_: le.type_.clone(),
                    body,
                }),
            }
        }
        common::AstNodeUnion::Application(app) => {
            let f = match &app.function {
                Some(b) => Some(Box::new(deepcopy_node(b))),
                None => None,
            };
            let a = match &app.argument {
                Some(b) => Some(Box::new(deepcopy_node(b))),
                None => None,
            };
            common::AstNode {
                type_: common::AstNodeType::APPLICATION,
                node: common::AstNodeUnion::Application(common::Application {
                    function: f,
                    argument: a,
                }),
            }
        }
    }
}

pub fn alpha_convert(old: &str) -> String {
    let n = ALPHA_N.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", old, n)
}

pub fn is_used(table: &hash_table::HashTable, variable: &str) -> bool {
    table.table_exists(variable)
}

pub fn parse_space_chars(in_: &mut File) {
    let mut c = peek(in_);
    while c == ' ' || c == '\n' || c == '\t' {
        let _ = crate_io::next(in_);
        c = peek(in_);
    }
}

pub fn parse_lambda(table: &mut hash_table::HashTable, in_: &mut File) -> common::AstNode {
    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        expect("A variable", '?');
    }

    let var = parse_variable(in_);
    let mut new_var: Option<String> = None;
    if is_used(table, &var) {
        if table.search(&var).is_some() {
            eprintln!(
                "A definition with name {} already exists. Cannot use same name for lambda abstraction.",
                var
            );
            std::process::exit(1);
        }
        let nv = alpha_convert(&var);
        table.insert(&nv, common::AstNode::default());
        new_var = Some(nv);
    } else {
        table.insert(&var, common::AstNode::default());
    }

    parse_space_chars(in_);

    consume(common::tokens_t::COLON, in_, ":");

    parse_space_chars(in_);

    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        eprintln!("Lambda abstractions should be typed.");
        std::process::exit(1);
    }
    let type_ = parse_type(table, in_);

    consume(common::tokens_t::DOT, in_, ".");

    let mut body = parse_expression(table, in_);

    if let Some(nv) = new_var {
        reducer::replace(&mut body, &var, &nv);
        return create_lambda(&nv, &body, &type_);
    }
    create_lambda(&var, &body, &type_)
}

pub fn parse_expression(table: &mut hash_table::HashTable, in_: &mut File) -> common::AstNode {
    while parse_token(peek(in_)) == common::tokens_t::WHITESPACE
        || parse_token(peek(in_)) == common::tokens_t::NEWLINE
    {
        let _ = crate_io::next(in_);
    }
    let scanned = parse_token(peek(in_));

    if scanned == common::tokens_t::ERROR {
        println!("Error: {} is  a valid token", peek(in_));
        std::process::exit(1);
    }

    if scanned == common::tokens_t::LAMBDA {
        let _ = crate_io::next(in_);
        return parse_lambda(table, in_);
    } else if scanned == common::tokens_t::L_PAREN {
        let _ = crate_io::next(in_);
        let expr = parse_expression(table, in_);

        print_ast(&expr);
        let next_token = parse_token(peek(in_));

        if next_token == common::tokens_t::WHITESPACE {
            let expr_2 = parse_expression(table, in_);
            let app = common::AstNode {
                type_: common::AstNodeType::APPLICATION,
                node: common::AstNodeUnion::Application(common::Application {
                    function: Some(Box::new(expr)),
                    argument: Some(Box::new(expr_2)),
                }),
            };
            consume(common::tokens_t::R_PAREN, in_, ")");
            return app;
        }
        consume(common::tokens_t::R_PAREN, in_, ")");
        return expr;
    } else if scanned == common::tokens_t::VARIABLE {
        let var_name = parse_variable(in_);

        if var_name == "def" {
            parse_definition(table, in_);
            if peek(in_) != '\u{FFFF}' {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        } else if var_name == "import" {
            parse_import(table, in_);
            if peek(in_) != '\u{FFFF}' {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        } else if var_name == "type" {
            parse_type_definition(table, in_);
            if peek(in_) != '\u{FFFF}' {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        }

        let mut type_ = String::new();
        parse_space_chars(in_);
        if !table.table_exists(&var_name) {
            if parse_token(peek(in_)) != common::tokens_t::COLON {
                eprintln!(
                    "Constant Variable {} is not typed. Please provide a type.",
                    var_name
                );
                std::process::exit(1);
            }
            consume(common::tokens_t::COLON, in_, ":");
            parse_space_chars(in_);
            type_ = parse_type(table, in_);
        }

        let mut variable = create_variable(&var_name, &type_);
        if table.search(&var_name).is_some() {
            variable.type_ = common::AstNodeType::DEFINITION;
        }
        return variable;
    }
    common::AstNode::default()
}

pub fn parse_import(table: &mut hash_table::HashTable, in_: &mut File) {
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");
    consume(common::tokens_t::QUOTE, in_, "\"");

    let mut file_path = String::new();
    let mut next_token = match crate_io::next(in_) {
        Ok(c) => c,
        Err(_) => '\u{FFFF}',
    };
    let mut n = parse_token(next_token);

    let mut current_length = 0;
    while n != common::tokens_t::QUOTE {
        if current_length < 100 - 1 {
            file_path.push(next_token);
            current_length += 1;
        } else {
            eprintln!("File path is too long. Please make sure it is less than 100 characters.");
            std::process::exit(1);
        }
        next_token = match crate_io::next(in_) {
            Ok(c) => c,
            Err(_) => '\u{FFFF}',
        };
        n = parse_token(next_token);
    }

    let mut imported_file = match crate_io::get_file(&file_path, "r") {
        Ok(f) => f,
        Err(_) => {
            eprintln!("ERROR: Could not open file {}", file_path);
            std::process::exit(1);
        }
    };

    loop {
        let imported_tkn = peek(&mut imported_file);
        if imported_tkn == '\u{FFFF}' {
            break;
        }
        let scanned = parse_token(imported_tkn);
        parse_space_chars(&mut imported_file);
        if scanned == common::tokens_t::VARIABLE {
            let var_name = parse_variable(&mut imported_file);
            if var_name == "def" {
                parse_definition(table, &mut imported_file);
            } else if var_name == "type" {
                parse_type_definition(table, &mut imported_file);
            } else {
                eprintln!(
                    "Expected a definition in the imported file, but got {}",
                    var_name
                );
                std::process::exit(1);
            }
        } else {
            // skip non-variable tokens
            let _ = crate_io::next(&mut imported_file);
        }
    }
}

pub fn parse_definition(table: &mut hash_table::HashTable, in_: &mut File) {
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");

    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        expect("a variable", peek(in_));
    }

    let def_name = parse_variable(in_);

    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");
    consume(common::tokens_t::EQ, in_, "=");
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");

    let definition = parse_expression(table, in_);
    table.insert(&def_name, definition);
}

pub fn is_uppercase(c: char) -> bool {
    c >= 'A' && c <= 'Z'
}

pub fn parse_type_definition(types_table: &mut hash_table::HashTable, in_: &mut File) {
    let next_token = match crate_io::next(in_) {
        Ok(c) => c,
        Err(_) => '\u{FFFF}',
    };
    let n = parse_token(next_token);
    if n != common::tokens_t::WHITESPACE {
        expect(" ", next_token);
    }

    let next_token2 = peek(in_);
    let n2 = parse_token(next_token2);
    if n2 != common::tokens_t::VARIABLE {
        expect("a type definition", next_token2);
    }

    if !is_uppercase(next_token2) {
        eprintln!("Type names must start with an uppercase letter");
        std::process::exit(1);
    }

    let type_name = parse_variable(in_);
    if types_table.table_exists(&type_name) {
        eprintln!("Type {} was already defined.", type_name);
        std::process::exit(1);
    }
    types_table.insert(&type_name, common::AstNode::default());
}

pub fn parse_type(types_table: &mut hash_table::HashTable, in_: &mut File) -> String {
    let mut type_name = String::new();
    let token = match crate_io::next(in_) {
        Ok(c) => c,
        Err(_) => '\u{FFFF}',
    };

    if !is_uppercase(token) {
        eprintln!("Types should start with an uppercase letter.");
        std::process::exit(1);
    }

    type_name.push(token);

    while is_variable(peek(in_)) {
        let c = match crate_io::next(in_) {
            Ok(c) => c,
            Err(_) => break,
        };
        type_name.push(c);
    }

    if !types_table.table_exists(&type_name) {
        eprintln!("Type {} was not defined.", type_name);
        std::process::exit(1);
    }
    type_name
}

pub fn parse_variable(in_: &mut File) -> String {
    let mut variable_name = String::new();
    while is_variable(peek(in_)) {
        let c = match crate_io::next(in_) {
            Ok(c) => c,
            Err(_) => break,
        };
        variable_name.push(c);
    }
    variable_name
}

pub fn expect(expected: &str, received: char) {
    println!("Syntax Error: Expected {} , received {} ", expected, received);
    std::process::exit(1);
}

pub fn free_ast(_node: &mut common::AstNode) {
    // Memory is managed automatically by Rust's ownership; no-op.
}
