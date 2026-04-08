use crate::{common, hash_table, reducer, io as cio};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicUsize, Ordering};

static N: AtomicUsize = AtomicUsize::new(1);

pub fn parse_token(token: char) -> common::tokens_t {
    match token {
        '(' => common::tokens_t::L_PAREN,
        ')' => common::tokens_t::R_PAREN,
        '@' => common::tokens_t::LAMBDA,
        '.' => common::tokens_t::DOT,
        ' ' => common::tokens_t::WHITESPACE,
        '\n' => common::tokens_t::NEWLINE,
        '=' => common::tokens_t::EQ,
        '"' => common::tokens_t::QUOTE,
        ':' => common::tokens_t::COLON,
        c if is_variable(c) => common::tokens_t::VARIABLE,
        _ => common::tokens_t::ERROR,
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
    match &node.type_ {
        common::AstNodeType::LAMBDA_EXPR => {
            if let common::AstNodeUnion::LambdaExpr(ref le) = node.node {
                print!("(LAMBDA {} : {}", le.parameter, le.type_);
                if let Some(ref body) = le.body {
                    print_ast(body);
                }
                print!(") ");
            }
        }
        common::AstNodeType::APPLICATION => {
            if let common::AstNodeUnion::Application(ref app) = node.node {
                print!("(APP ");
                if let Some(ref f) = app.function {
                    print_ast(f);
                }
                if let Some(ref a) = app.argument {
                    print_ast(a);
                }
                print!(") ");
            }
        }
        common::AstNodeType::VAR => {
            if let common::AstNodeUnion::Variable(ref var) = node.node {
                print!("(VAR {} ", var.name);
                if !var.type_.is_empty() {
                    print!(": {}", var.type_);
                }
                print!(")");
            }
        }
        common::AstNodeType::DEFINITION => {
            if let common::AstNodeUnion::Variable(ref var) = node.node {
                print!("(DEFINITION {}) ", var.name);
            }
        }
    }
}
pub fn is_variable(token: char) -> bool {
    let c = token as u32;
    if c == '_' as u32 { return true; }
    (c >= 97 && c <= 122) || (c >= 65 && c <= 90)
}
pub fn peek(in_: &mut File) -> char {
    let mut buf = [0u8; 1];
    let n = in_.read(&mut buf).unwrap_or(0);
    if n == 0 {
        return char::from(0xff_u8); // EOF
    }
    in_.seek(SeekFrom::Current(-1)).unwrap();
    buf[0] as char
}
pub fn peek_print(in_: &mut File, n: usize) {
    let mut buffer = vec![0u8; n];
    let read = in_.read(&mut buffer).unwrap_or(0);
    let s = String::from_utf8_lossy(&buffer[..read]);
    print!("{}", s);
    if read > 0 {
        in_.seek(SeekFrom::Current(-(read as i64))).unwrap();
    }
}
pub fn consume(t: common::tokens_t, in_: &mut File, expected: &str) {
    let c = next_char(in_);
    let p = parse_token(c);
    if p != t {
        expect(expected, c);
    }
}

fn next_char(in_: &mut File) -> char {
    let mut buf = [0u8; 1];
    let n = in_.read(&mut buf).unwrap_or(0);
    if n == 0 {
        char::from(0xff_u8)
    } else {
        buf[0] as char
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
            function: Some(Box::new(function.clone())),
            argument: Some(Box::new(argument.clone())),
        }),
    }
}
pub fn create_lambda(variable: &str, body: &common::AstNode, type_: &str) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::LAMBDA_EXPR,
        node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
            parameter: variable.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(body.clone())),
        }),
    }
}
pub fn alpha_convert(old: &str) -> String {
    let n = N.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", old, n)
}
pub fn is_used(table: &hash_table::HashTable, variable: &str) -> bool {
    table.table_exists(variable)
}
pub fn parse_space_chars(in_: &mut File) {
    let mut c = peek(in_);
    while c == ' ' || c == '\n' || c == '\t' {
        next_char(in_);
        c = peek(in_);
    }
}
pub fn parse_lambda(table: &mut hash_table::HashTable, in_: &mut File) -> common::AstNode {
    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        expect("A variable", peek(in_));
    }

    let var = parse_variable(in_);
    let mut new_var: Option<String> = None;
    if is_used(table, &var) {
        if table.search(&var).is_some() {
            common::error(
                &format!("A definition with name {} already exists. Cannot use same name for lambda abstraction.\n", var),
                file!(), line!() as i32, "parse_lambda",
            );
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
        common::error("Lambda abstractions should be typed.", file!(), line!() as i32, "parse_lambda");
    }
    let type_ = parse_type(table, in_);

    consume(common::tokens_t::DOT, in_, ".");

    let mut body = parse_expression(table, in_);

    if let Some(ref nv) = new_var {
        reducer::replace(&mut body, &var, nv);
        common::print_verbose("Alpha converted", format_args!("Alpha converted {} to {}\n", var, nv));
        return create_lambda(nv, &body, &type_);
    }
    create_lambda(&var, &body, &type_)
}
pub fn parse_expression(table: &mut hash_table::HashTable, in_: &mut File) -> common::AstNode {
    loop {
        let t = parse_token(peek(in_));
        if t != common::tokens_t::WHITESPACE && t != common::tokens_t::NEWLINE {
            break;
        }
        next_char(in_);
    }

    let scanned = parse_token(peek(in_));

    if scanned == common::tokens_t::ERROR {
        println!("Error: {} is  a valid token", peek(in_));
        std::process::exit(1);
    }

    if scanned == common::tokens_t::LAMBDA {
        next_char(in_);
        return parse_lambda(table, in_);
    } else if scanned == common::tokens_t::L_PAREN {
        next_char(in_);
        let expr = parse_expression(table, in_);
        print_ast(&expr);
        let next_token = parse_token(peek(in_));

        if next_token == common::tokens_t::WHITESPACE {
            let expr_2 = parse_expression(table, in_);
            let application = common::AstNode {
                type_: common::AstNodeType::APPLICATION,
                node: common::AstNodeUnion::Application(common::Application {
                    function: Some(Box::new(expr)),
                    argument: Some(Box::new(expr_2)),
                }),
            };
            consume(common::tokens_t::R_PAREN, in_, ")");
            return application;
        }
        consume(common::tokens_t::R_PAREN, in_, ")");
        return expr;
    } else if scanned == common::tokens_t::VARIABLE {
        let var_name = parse_variable(in_);

        if var_name == "def" {
            parse_definition(table, in_);
            let p = peek(in_);
            if p as u8 != 0xff {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        } else if var_name == "import" {
            parse_import(table, in_);
            let p = peek(in_);
            if p as u8 != 0xff {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        } else if var_name == "type" {
            parse_type_definition(table, in_);
            let p = peek(in_);
            if p as u8 != 0xff {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        }

        let mut type_ = String::new();
        parse_space_chars(in_);

        if !table.table_exists(&var_name) {
            if parse_token(peek(in_)) != common::tokens_t::COLON {
                common::error(
                    &format!("Constant Variable {} is not typed. Please provide a type.\n", var_name),
                    file!(), line!() as i32, "parse_expression",
                );
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
    let mut next_token = next_char(in_);
    let mut n = parse_token(next_token);

    while n != common::tokens_t::QUOTE {
        if file_path.len() < 99 {
            file_path.push(next_token);
        } else {
            common::error(
                "File path is too long. Please make sure it is less than 100 characters.",
                file!(), line!() as i32, "parse_import",
            );
        }
        next_token = next_char(in_);
        n = parse_token(next_token);
    }

    if n != common::tokens_t::QUOTE {
        expect("a closing quote", next_token);
    }

    let mut imported_file = cio::get_file(&file_path, "r").unwrap_or_else(|_| {
        common::error(
            &format!("ERROR: Could not open file {}\n", file_path),
            file!(), line!() as i32, "parse_import",
        );
        unreachable!()
    });

    loop {
        let imported_tkn = peek(&mut imported_file);
        if imported_tkn as u8 == 0xff {
            break;
        }
        let scanned = parse_token(imported_tkn);
        parse_space_chars(&mut imported_file);
        if scanned == common::tokens_t::VARIABLE || parse_token(peek(&mut imported_file)) == common::tokens_t::VARIABLE {
            let var_name = parse_variable(&mut imported_file);
            if var_name == "def" {
                parse_definition(table, &mut imported_file);
            } else if var_name == "type" {
                parse_type_definition(table, &mut imported_file);
            } else {
                common::error(
                    &format!("Expected a definition in the imported file, but got {}\n", var_name),
                    file!(), line!() as i32, "parse_import",
                );
            }
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
    let next_token = next_char(in_);
    let n = parse_token(next_token);
    if n != common::tokens_t::WHITESPACE {
        expect(" ", next_token);
    }

    let next_token = peek(in_);
    let n = parse_token(next_token);
    if n != common::tokens_t::VARIABLE {
        expect("a type definition", next_token);
    }

    if !is_uppercase(next_token) {
        common::error("Type names must start with an uppercase letter", file!(), line!() as i32, "parse_type_definition");
    }

    let type_name = parse_variable(in_);
    if types_table.table_exists(&type_name) {
        common::error(
            &format!("Type {} was already defined.\n", type_name),
            file!(), line!() as i32, "parse_type_definition",
        );
    }
    types_table.insert(&type_name, common::AstNode::default());
}
pub fn parse_type(types_table: &mut hash_table::HashTable, in_: &mut File) -> String {
    let mut type_name = String::new();
    let token = next_char(in_);

    if !is_uppercase(token) {
        common::error("Types should start with an uppercase letter.", file!(), line!() as i32, "parse_type");
    }

    type_name.push(token);

    while is_variable(peek(in_)) {
        type_name.push(next_char(in_));
    }

    if !types_table.table_exists(&type_name) {
        common::error(
            &format!("Type {} was not defined.\n", type_name),
            file!(), line!() as i32, "parse_type",
        );
    }
    type_name
}
pub fn parse_variable(in_: &mut File) -> String {
    let mut variable_name = String::new();
    while is_variable(peek(in_)) {
        variable_name.push(next_char(in_));
    }
    variable_name
}
pub fn expect(expected: &str, received: char) {
    println!("Syntax Error: Expected {} , received {} ", expected, received);
    std::process::exit(1);
}
pub fn free_ast(_node: &mut common::AstNode) {
    // Memory is managed by Rust's ownership system
}
